use std::ops::Range;

use eframe::egui::text::{CCursor, CCursorRange};
use eframe::egui::text_edit::TextEditState;
use eframe::egui::{Context, Id, Response};
use toile_engine::draft::{Doc, MeasureSet, VariableKey};

use super::super::state::{Field, State};
use super::write::id_of;
use crate::widgets::Mention;

const IDLE: &str = "Escribe en una fórmula y pulsa un nombre para insertarlo.";
const LIVE: &str = "Pulsa un nombre: entra en la fórmula donde está el cursor.";
const NUMBER: &str = "Este campo es un número: los nombres solo entran en fórmulas.";

/// Where a name pressed on the panel goes this frame.
///
/// It is decided before any row is drawn, and that order is the whole trick. A
/// row learns that its box lost the focus while the row itself draws, which is
/// before the name that took the press has been drawn at all; left to itself
/// it would confirm the half written formula on the spot. egui hit-tests each
/// frame against the rects of the frame before, so what the press landed on
/// can be read back ahead of drawing it, and the row told to hold its text.
#[derive(Debug)]
pub struct Cite {
    /// The formula being written, while one has the focus.
    into: Option<Field>,
    /// Whether the focus is on a field that takes a number and no name.
    number: bool,
    /// Whether the pointer pressed or clicked a name this frame.
    claimed: bool,
}

impl Cite {
    /// Reads, ahead of drawing, what is being written and what was pressed.
    pub fn begin(ctx: &Context, doc: &Doc, state: &State) -> Cite {
        let focused = state
            .editing
            .as_ref()
            .map(|edit| &edit.of)
            .filter(|of| ctx.memory(|m| m.has_focus(id_of(of))));
        let into = focused
            .filter(|of| matches!(of, Field::Coordinate(..) | Field::Variable(_)))
            .cloned();
        let number = focused.is_some() && into.is_none();
        let claimed = into.is_some()
            && ids(doc).any(|id| ctx.read_response(id).is_some_and(|hit| taken(ctx, &hit)));
        Cite {
            into,
            number,
            claimed,
        }
    }

    /// How a measurement's name answers a press.
    pub fn measure(&self) -> Mention {
        match self.into {
            Some(_) => Mention::Live,
            None => Mention::Inert,
        }
    }

    /// How a variable's name answers a press.
    ///
    /// Never into its own formula: a variable that reads itself is a cycle,
    /// and a press that can only write one is not offered.
    pub fn variable(&self, key: VariableKey) -> Mention {
        match &self.into {
            Some(Field::Variable(own)) if *own == key => Mention::Inert,
            Some(_) => Mention::Live,
            None => Mention::Inert,
        }
    }

    /// Whether `of` lost its focus to a name meant for it, and so has to keep
    /// its text instead of confirming it.
    pub fn keeps(&self, of: &Field) -> bool {
        self.claimed && self.into.as_ref() == Some(of)
    }

    /// The line that says how a name gets into a formula, from where the
    /// person stands.
    pub fn hint(&self) -> &'static str {
        if self.into.is_some() {
            LIVE
        } else if self.number {
            NUMBER
        } else {
            IDLE
        }
    }

    /// Puts `name` into the formula being written, over whatever of it is
    /// selected, and leaves the caret right after the name with the focus on
    /// the field.
    pub fn press(&self, ctx: &Context, state: &mut State, name: &str) {
        let Some(edit) = state.editing.as_mut() else {
            return;
        };
        if self.into.as_ref() != Some(&edit.of) {
            return;
        }
        let id = id_of(&edit.of);
        let mut held = TextEditState::load(ctx, id).unwrap_or_default();
        let end = edit.buffer.chars().count();
        let span = held.cursor.char_range().map_or(end..end, |range| {
            let (a, b) = (range.primary.index.0, range.secondary.index.0);
            a.min(b)..a.max(b)
        });
        let (buffer, caret) = spliced(&edit.buffer, span, name);
        edit.buffer = buffer;
        let at = CCursor::new(caret);
        held.cursor.set_char_range(Some(CCursorRange::one(at)));
        held.store(ctx, id);
        ctx.memory_mut(|m| m.request_focus(id));
        // The box was drawn before the name was pressed, so only the next
        // frame shows the name in it.
        ctx.request_repaint();
    }
}

/// The identity a measurement's name is pressed under.
pub fn measure_id(name: &str) -> Id {
    Id::new(("patronaje-measure-name", name))
}

/// The identity a variable's name is pressed under.
///
/// By key and not by name: a variable may borrow the name of a measurement
/// its body does not carry, and both are on the panel at once.
pub fn variable_id(key: VariableKey) -> Id {
    Id::new(("patronaje-variable-name", key))
}

/// `text` with the characters in `span` replaced by `name`, and the character
/// offset right after the name.
pub fn spliced(text: &str, span: Range<usize>, name: &str) -> (String, usize) {
    let byte = |at: usize| text.char_indices().nth(at).map_or(text.len(), |(i, _)| i);
    let (from, to) = (byte(span.start), byte(span.end.max(span.start)));
    let mut out = String::with_capacity(text.len() + name.len());
    out.push_str(&text[..from]);
    out.push_str(name);
    out.push_str(&text[to..]);
    let start = text[..from].chars().count();
    (out, start + name.chars().count())
}

/// Every name the panel offers a press on.
fn ids(doc: &Doc) -> impl Iterator<Item = Id> + '_ {
    let carried = doc
        .measures()
        .map(MeasureSet::uncatalogued)
        .unwrap_or_default();
    MeasureSet::CATALOGUE
        .into_iter()
        .chain(carried)
        .map(measure_id)
        .chain(doc.variables.keys().map(variable_id))
}

/// Whether this frame's pointer took `hit`: the press that starts a click on
/// it, or the release that ends one.
///
/// Both, because egui can be told to take the focus away on either.
fn taken(ctx: &Context, hit: &Response) -> bool {
    hit.clicked() || (hit.is_pointer_button_down_on() && ctx.input(|i| i.pointer.any_pressed()))
}
