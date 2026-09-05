use eframe::egui::{self, Rect, Response};
use toile_engine::draft::{Command, Doc, PieceKey, PointKey};

use super::curve::Bend;
use super::gesture::{EditContext, Gesture, Input, Mods, Stack};
use super::state::State;
use super::tract::Tract;
use super::{input, modal};
use crate::theme::Theme;

/// How much one notch of the wheel is worth in scale.
const ZOOM_RATE: f32 = 0.004;

/// The extremes a single wheel event may move the scale by.
const ZOOM_LIMIT: [f64; 2] = [0.2, 5.0];

/// What the mat asks the session to do, once the drawing is over.
///
/// The panels draw from a document they only borrow, so nothing they decide is
/// applied while they are drawing it; the tab plays this list afterwards.
#[derive(Debug, Clone, PartialEq)]
pub enum Verb {
    /// Open an undo entry under this name.
    Begin(&'static str),
    /// Edit the document.
    Edit(Box<Command>),
    /// Close the entry. One that edited nothing leaves nothing.
    End,
    /// Take the last entry back.
    Undo,
    /// Take it back and throw it away: what was refused is not redoable.
    Cancel,
    /// Put it back in.
    Redo,
}

/// The drawing on the table, as one frame worked it out.
///
/// The tracts and the bends are built once and lent to every event of the
/// frame: flattening the contour again per event would be the same answer at
/// nine times the price.
pub struct Table<'a> {
    /// The document, for the bindings a drag rewrites.
    pub doc: &'a Doc,
    /// The piece on the table.
    pub piece: PieceKey,
    /// Its nodes, resolved, in contour order, in centimetres.
    pub nodes: &'a [(PointKey, [f64; 2])],
    /// Its tracts as the drawing paints them.
    pub tracts: &'a [Tract],
    /// Its bent tracts, with every handle resolved.
    pub bends: &'a [Bend],
}

/// Runs this frame's events through the reducer, in the order they happened.
///
/// Everything the gesture decides comes back as feedback: the view slides, the
/// selection moves, the tool changes, the undo stack opens and closes, and the
/// commands go on the list the tab plays afterwards.
pub fn reduce(
    ui: &egui::Ui,
    resp: &Response,
    table: &Table<'_>,
    state: &mut State,
    verbs: &mut Vec<Verb>,
) {
    for event in events_of(ui, resp) {
        // Built per event: what a press takes in hand is whatever the event
        // before it left chosen.
        let ctx = EditContext {
            doc: table.doc,
            piece: table.piece,
            nodes: table.nodes,
            tracts: table.tracts,
            bends: table.bends,
            selection: state.selection.clone(),
            tool: state.tool,
            view: state.view,
            snap: state.snap,
        };
        let held = std::mem::take(&mut state.gesture);
        let (next, commands, feedback) = input::update(held, event, &ctx);
        state.gesture = next;
        if let Some(Stack::Open(label) | Stack::Once(label)) = feedback.stack {
            verbs.push(Verb::Begin(label));
        }
        verbs.extend(
            commands
                .into_iter()
                .map(|command| Verb::Edit(Box::new(command))),
        );
        match feedback.stack {
            Some(Stack::Close | Stack::Once(_)) => verbs.push(Verb::End),
            Some(Stack::Undo) => verbs.push(Verb::Undo),
            Some(Stack::Cancel) => verbs.push(Verb::Cancel),
            Some(Stack::Redo) => verbs.push(Verb::Redo),
            Some(Stack::Open(_)) | None => {}
        }
        if let Some(select) = feedback.select {
            state.selection = select;
        }
        if let Some(tool) = feedback.tool {
            state.tool = tool;
        }
        state.view.pan(feedback.pan);
        // The candidate is kept between frames: a pointer that stops moving
        // has not stopped being on the node the ring is drawn around.
        state.caught = feedback.snapped.or(state.caught);
        state.ask = feedback.ask.or(state.ask.take());
    }
    if state.gesture == Gesture::Idle {
        state.caught = None;
    }
}

/// This frame's events, in the order they happened.
///
/// The press is read off the response, so a chip lying over the mat keeps the
/// click it was given; the moves and the release are read off the pointer, so
/// a drag that leaves the mat is still the same drag. The keyboard is read raw
/// and only while no field has the focus, so a gesture never steals a
/// character from a text box.
pub fn events_of(ui: &egui::Ui, resp: &Response) -> Vec<Input> {
    let mods = mods(ui);
    let mut out = Vec::new();
    let (pressed, released, stirred) = ui.input(|i| {
        (
            i.pointer.primary_pressed(),
            i.pointer.primary_released(),
            i.pointer.delta() != egui::Vec2::ZERO,
        )
    });
    if pressed && let Some(at) = resp.interact_pointer_pos() {
        out.push(Input::Down(at, mods));
    }
    if let Some(at) = ui.input(|i| i.pointer.latest_pos()) {
        if stirred {
            out.push(Input::Move(at, mods));
        }
        if released {
            out.push(Input::Up(at, mods));
        }
    }
    if ui.memory(eframe::egui::Memory::focused).is_some() {
        return out;
    }
    ui.input(|i| {
        for event in &i.events {
            match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => out.push(Input::Key(*key, of(*modifiers))),
                egui::Event::Text(text) => out.push(Input::Text(text.clone())),
                _ => {}
            }
        }
    });
    out
}

/// The modifiers held right now.
///
/// Space is not one of egui's, but it is one of the mat's: held down it turns
/// a drag over the paper into a pan instead of a marquee.
fn mods(ui: &egui::Ui) -> Mods {
    let (modifiers, space) = ui.input(|i| (i.modifiers, i.key_down(egui::Key::Space)));
    Mods {
        space,
        ..of(modifiers)
    }
}

/// The modifiers a gesture cares about, out of the ones egui reports.
fn of(modifiers: egui::Modifiers) -> Mods {
    Mods {
        shift: modifiers.shift,
        ctrl: modifiers.ctrl,
        alt: modifiers.alt,
        command: modifiers.command,
        space: false,
    }
}

/// Wheel and the view keys, which belong to the glass and not to the document.
pub fn view_keys(ui: &egui::Ui, resp: &Response, state: &mut State) {
    if let Some(at) = resp.hover_pos() {
        let (wheel, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let factor = f64::from((1.0 + wheel * ZOOM_RATE) * pinch);
        if (factor - 1.0).abs() > 1.0e-6 {
            state
                .view
                .zoom_at(at, factor.clamp(ZOOM_LIMIT[0], ZOOM_LIMIT[1]));
        }
    }
    // A field with the focus owns the keyboard: `f` typed into a formula is a
    // character, not a request to reframe the drawing.
    if ui.memory(egui::Memory::focused).is_some() {
        return;
    }
    let ppp = ui.ctx().pixels_per_point();
    ui.input(|i| {
        if i.key_pressed(egui::Key::F) {
            state.frame = true;
        }
        if i.key_pressed(egui::Key::Num1) {
            state.view.one_to_one(ppp);
        }
    });
}

/// The question a drag over a formula left on the mat, and what it is worth.
pub fn answer(
    ui: &mut egui::Ui,
    theme: &Theme,
    rect: Rect,
    state: &mut State,
    verbs: &mut Vec<Verb>,
) {
    let Some(ask) = state.ask.as_ref() else {
        return;
    };
    let Some(answer) = modal::show(ui, theme, rect, ask) else {
        return;
    };
    verbs.push(match answer {
        modal::Answer::Adapt => Verb::End,
        // The drag is refused, not stepped back through: a redo that put it
        // back would rewrite the very formula the answer asked to respect.
        modal::Answer::Respect => Verb::Cancel,
    });
    state.ask = None;
}

#[cfg(test)]
mod tests;
