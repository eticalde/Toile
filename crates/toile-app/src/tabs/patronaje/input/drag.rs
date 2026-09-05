use eframe::egui::{Key, Pos2};
use toile_engine::draft::{Axis, Binding, Command};

use super::super::curve;
use super::super::gesture::{self, Ask, Drag, EditContext, Feedback, Gesture, Mods, Stack, Typed};
use super::super::snap::{self, SnapConfig, SnapContext};

/// How far the pointer has to travel before a press becomes a drag, in screen
/// points. Under it the gesture is a click, and a click edits nothing.
const HAIR: f32 = 2.0;

/// A drag frame: the nodes follow the pointer, wherever the snap lets the one
/// under it land.
pub(super) fn moved(
    mut drag: Drag,
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    if drag.typed.is_some() {
        // The keyboard has the gesture: a jog of the mouse does not fight it.
        return (gesture::holding(drag), Vec::new(), Feedback::default());
    }
    // The break latches: a tangent that healed itself the moment the key came
    // back up would undo the asymmetry the key was held down to make.
    drag.free |= mods.alt;
    let cfg = SnapConfig {
        on: ctx.snap.on && !mods.ctrl,
        axis: mods.shift,
        ..ctx.snap
    };
    let scale = ctx.view.scale().max(f64::EPSILON);
    let anchor = drag.anchor().from;
    let raw = [
        anchor[0] + f64::from(at.x - drag.grab.x) / scale,
        anchor[1] + f64::from(at.y - drag.grab.y) / scale,
    ];
    let shown = curve::handles(ctx.bends, &ctx.selection);
    let hand = drag.keys();
    let snapped = snap::resolve(
        raw,
        &SnapContext {
            nodes: ctx.nodes,
            handles: &shown,
            tracts: ctx.tracts,
            held: &hand,
            anchor,
            scale,
        },
        cfg,
    );
    let seen = Feedback {
        snapped: Some(snapped),
        ..Feedback::default()
    };
    if !drag.moved && (at - drag.grab).length() <= HAIR {
        return (gesture::holding(drag), Vec::new(), seen);
    }
    drag.moved = true;
    drag.to = snapped.at;
    drag.step = cfg.step_cm();
    let commands = drag
        .placed(drag.step)
        .into_iter()
        .map(|(point, to)| Command::MovePoint { point, to })
        .collect();
    (gesture::holding(drag), commands, seen)
}

/// Letting go. A drag over a formula does not close its entry: the modal does.
pub(super) fn release(drag: &Drag) -> (Gesture, Vec<Command>, Feedback) {
    if !drag.moved {
        return rest(Feedback {
            stack: Some(Stack::Close),
            ..Feedback::default()
        });
    }
    match Ask::of(drag, drag.step, None) {
        Some(ask) => rest(Feedback {
            ask: Some(ask),
            ..Feedback::default()
        }),
        None => rest(Feedback {
            stack: Some(Stack::Close),
            ..Feedback::default()
        }),
    }
}

/// A key pressed mid-drag: the precision box, or the way out.
pub(super) fn during(mut drag: Drag, key: Key, mods: Mods) -> (Gesture, Vec<Command>, Feedback) {
    match key {
        // The first way out is the precision box, when one is open: a number
        // typed by mistake costs the escape key, not the whole gesture.
        Key::Escape if drag.typed.is_some() => {
            drag.typed = None;
            (gesture::holding(drag), Vec::new(), Feedback::default())
        }
        // An abandoned drag is refused, not undone: what the user backed out
        // of has no business on the redo stack.
        Key::Escape if drag.moved => rest(Feedback {
            stack: Some(Stack::Cancel),
            ..Feedback::default()
        }),
        Key::Escape => rest(Feedback {
            stack: Some(Stack::Close),
            ..Feedback::default()
        }),
        Key::Tab => {
            drag.typed = next_box(drag.typed.as_ref(), mods);
            (gesture::holding(drag), Vec::new(), Feedback::default())
        }
        Key::Backspace => {
            if let Some(typed) = drag.typed.as_mut() {
                typed.buffer.pop();
            }
            (gesture::holding(drag), Vec::new(), Feedback::default())
        }
        Key::Enter => commit(drag),
        _ => (gesture::holding(drag), Vec::new(), Feedback::default()),
    }
}

/// The coordinate the precision box writes next: X, then Y, then gone.
fn next_box(open: Option<&Typed>, mods: Mods) -> Option<Typed> {
    let axis = match open.map(|typed| typed.axis) {
        None => Axis::X,
        Some(Axis::X) if !mods.shift => Axis::Y,
        Some(_) => return None,
    };
    Some(Typed {
        axis,
        buffer: String::new(),
    })
}

/// Enter: the typed value takes over its coordinate and the gesture is over.
///
/// It writes the node the pointer took hold of, which is the one the box is
/// drawn beside; the rest of the group keeps where the drag put it.
///
/// Text that does not parse is kept, not thrown away, so the box can paint the
/// fault and the person can fix the character they missed.
fn commit(mut drag: Drag) -> (Gesture, Vec<Command>, Feedback) {
    let Some(typed) = drag.typed.take() else {
        return release(&drag);
    };
    let Ok(to) = Binding::parse(typed.buffer.trim()) else {
        drag.typed = Some(typed);
        return (gesture::holding(drag), Vec::new(), Feedback::default());
    };
    let ask = Ask::of(&drag, drag.step, Some(typed.axis));
    let stack = ask.is_none().then_some(Stack::Close);
    let command = Command::SetBinding {
        point: drag.anchor().point,
        axis: typed.axis,
        to,
    };
    (
        Gesture::Idle,
        vec![command],
        Feedback {
            stack,
            ask,
            ..Feedback::default()
        },
    )
}

/// Characters typed into the precision box, if one is open.
pub(super) fn typing(mut drag: Drag, text: &str) -> (Gesture, Vec<Command>, Feedback) {
    if let Some(typed) = drag.typed.as_mut() {
        typed.buffer.push_str(text);
    }
    (gesture::holding(drag), Vec::new(), Feedback::default())
}

/// Back to looking, carrying whatever the event left to say.
fn rest(feedback: Feedback) -> (Gesture, Vec<Command>, Feedback) {
    (Gesture::Idle, Vec::new(), feedback)
}
