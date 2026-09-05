use eframe::egui::{Key, Pos2};
use toile_engine::draft::{Command, Doc, Identity, Piece, PieceKey, Point, SegmentEdit, Winding};

use super::super::gesture::{EditContext, Feedback, Gesture, Input, Mods, Stack};
use super::super::pick::{self, NODE_PT};
use super::super::snap::{self, SnapConfig, SnapContext, Snapped};
use super::super::state::Selection;
use super::reach;

/// The name the drawn piece leaves in the undo stack.
pub(super) const DRAW: &str = "dibujar pieza";

/// What a drawn piece is called until somebody renames it.
const PIECE: &str = "Pieza";

/// The fewest vertices that close a contour.
const SIDES: usize = 3;

/// The finest a free vertex is written to, in centimetres.
const HUNDREDTHS: f64 = 100.0;

/// The Line tool's first press: the drawing opens on its first vertex.
pub(super) fn start(
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    place(Vec::new(), at, mods, ctx)
}

/// Reduces one event against the drawing in progress.
///
/// No command goes out until the contour closes: Escape walks away from any
/// number of placed vertices without an entry to unwind, and Backspace takes
/// the last one back for free.
pub(super) fn update(
    pending: Vec<[f64; 2]>,
    rubber: [f64; 2],
    event: &Input,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    match *event {
        Input::Down(_, mods) if mods.space => keep(pending, rubber),
        Input::Down(at, mods) => pressed(pending, at, mods, ctx),
        Input::Move(at, mods) => {
            let caught = vertex(&pending, at, mods, ctx);
            (
                Gesture::Drawing {
                    pending,
                    rubber: caught.at,
                },
                Vec::new(),
                Feedback {
                    snapped: Some(caught),
                    ..Feedback::default()
                },
            )
        }
        Input::Key(Key::Enter, _) if pending.len() >= SIDES => close(&pending, ctx),
        Input::Key(Key::Escape, _) => (Gesture::Idle, Vec::new(), Feedback::default()),
        Input::Key(Key::Backspace | Key::Delete, _) => {
            let mut pending = pending;
            pending.pop();
            keep(pending, rubber)
        }
        Input::Up(..) | Input::Key(..) | Input::Text(_) => keep(pending, rubber),
    }
}

/// The drawing as it was, with nothing to say.
fn keep(pending: Vec<[f64; 2]>, rubber: [f64; 2]) -> (Gesture, Vec<Command>, Feedback) {
    (
        Gesture::Drawing { pending, rubber },
        Vec::new(),
        Feedback::default(),
    )
}

/// A press: on the first vertex it closes the contour, anywhere else it
/// places the next one.
///
/// The closing hit is asked of the raw pointer before the snap runs, so that
/// coming back to the first vertex closes the piece rather than landing one
/// more vertex on the grid line beside it.
fn pressed(
    pending: Vec<[f64; 2]>,
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    let cm = ctx.view.to_document(at);
    if pending.len() >= SIDES && pick::away(cm, pending[0]) < reach(ctx, NODE_PT) {
        return close(&pending, ctx);
    }
    place(pending, at, mods, ctx)
}

/// Puts the next vertex where the snap lets it land.
fn place(
    mut pending: Vec<[f64; 2]>,
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    let caught = vertex(&pending, at, mods, ctx);
    // A double click on the same spot would draw a tract of no length, which
    // the contour check downstream would only refuse later and louder.
    if pending.last() != Some(&caught.at) {
        pending.push(caught.at);
    }
    (
        Gesture::Drawing {
            pending,
            rubber: caught.at,
        },
        Vec::new(),
        Feedback {
            snapped: Some(caught),
            ..Feedback::default()
        },
    )
}

/// Where a vertex would land: through the snap ladder, from the last vertex.
///
/// A position nothing caught is written in tenths, like a free drag: a vertex
/// carrying the pointer's arithmetic out to the fifteenth decimal reads as
/// noise everywhere the coordinate is shown. One the ladder caught stays
/// exactly where it was caught.
fn vertex(pending: &[[f64; 2]], at: Pos2, mods: Mods, ctx: &EditContext<'_>) -> Snapped {
    let raw = ctx.view.to_document(at);
    let cfg = SnapConfig {
        on: ctx.snap.on && !mods.ctrl,
        axis: mods.shift,
        ..ctx.snap
    };
    let anchor = pending.last().copied().unwrap_or(raw);
    let caught = snap::resolve(
        raw,
        &SnapContext {
            nodes: ctx.nodes,
            handles: &[],
            tracts: ctx.tracts,
            held: &[],
            anchor,
            scale: ctx.view.scale().max(f64::EPSILON),
        },
        cfg,
    );
    match caught.kind {
        None => Snapped {
            at: caught.at.map(|v| rounded(v, cfg.step_cm())),
            kind: None,
        },
        Some(_) => caught,
    }
}

/// A coordinate written at the resolution the gesture is working in.
fn rounded(value: f64, step: f64) -> f64 {
    let stepped = if step > 0.0 && step.is_finite() {
        (value / step).round() * step
    } else {
        value
    };
    (stepped * HUNDREDTHS).round() / HUNDREDTHS
}

/// Closes the contour: the whole piece goes out as one entry of the history.
///
/// `AddPiece` cannot cite points that are not in the document yet, so the
/// piece lands empty and every vertex follows it as an `InsertNode` at the
/// head of its contour — in reverse, so the contour ends up in the order the
/// vertices were clicked. The commands have to name a piece no key exists for
/// yet, and the arena says where its next insertion lands (`issued`), so the
/// key is written down before the first command is applied.
fn close(pending: &[[f64; 2]], ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    let piece = PieceKey::new(ctx.doc.pieces.issued(), 0);
    let mut commands = vec![Command::AddPiece {
        identity: Identity::New,
        piece: Piece::polygon(&name(ctx.doc), std::iter::empty(), winding(pending)),
    }];
    commands.extend(pending.iter().rev().map(|&[x, y]| Command::InsertNode {
        piece,
        after: None,
        identity: Identity::New,
        value: Point::at(x, y),
        segment: SegmentEdit::Line,
        samples: 1,
    }));
    (
        Gesture::Idle,
        commands,
        Feedback {
            stack: Some(Stack::Once(DRAW)),
            select: Some(Selection::None),
            ..Feedback::default()
        },
    )
}

/// The first "Pieza n" no piece of the document carries yet.
fn name(doc: &Doc) -> String {
    // Pigeonhole: n pieces cannot take all of n + 1 names.
    (1..=doc.pieces.len() + 1)
        .map(|n| format!("{PIECE} {n}"))
        .find(|name| doc.piece_named(name).is_none())
        .expect("one more name than there are pieces")
}

/// The direction the drawn contour runs in, declared from its own area.
///
/// The shoelace sum in document coordinates, where y grows downward: drawn
/// clockwise on the page is a positive area there.
fn winding(pending: &[[f64; 2]]) -> Winding {
    let mut doubled = 0.0;
    for (index, a) in pending.iter().enumerate() {
        let b = pending[(index + 1) % pending.len()];
        doubled += a[0] * b[1] - b[0] * a[1];
    }
    Winding::of_area(doubled / 2.0)
}
