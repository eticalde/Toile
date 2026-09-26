/// The darts a piece carries, as the panel and the mat read them.
mod cut;
/// What the mat draws of a dart, and of the wedge being cut.
mod mark;
/// The wedge the presses of the tool build, before it is cut.
mod wedge;

pub use cut::{Cut, on};
use eframe::egui::{Key, Pos2};
pub use mark::{cutting, drawn};
use toile_engine::draft::{Command, Dart, DartWedge, Identity, PointKey, SeamKey};
pub use wedge::{Darting, Leg};

use super::gesture::{EditContext, Feedback, Gesture, Input, Mods, Stack};
use super::snap::{self, SnapConfig, SnapContext, SnapKind, Snapped};
use super::state::Selection;
use super::{curve, pick, tract};

/// The name one dart cut leaves in the undo stack.
pub const PUT_ON: &str = "poner pinza";

/// The name taking one off leaves there.
pub const TAKE_OFF: &str = "quitar la pinza";

/// How many legs a wedge has, which is what the second press finishes.
pub const LEGS: usize = 2;

const OFF_EDGE: &str = "la pata de una pinza se pone en un tramo del contorno, no en un nodo ni \
                        sobre la tela";
const OTHER_TRACT: &str = "las dos patas van en el mismo tramo; Esc deja esta pinza y empieza otra";
const BENT: &str = "ese tramo es una curva, y la boca de una pinza se corta en un tramo recto";
const ONE_PLACE: &str = "la segunda pata va en otro sitio del tramo, no encima de la primera";
const OFF_CLOTH: &str = "el pico de la pinza va dentro de la pieza, no en su contorno ni fuera";

/// Opens a wedge where a press landed, and opens none where it landed
/// elsewhere.
pub fn begin(at: Pos2, mods: Mods, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    match leg(at, None, mods, ctx) {
        Ok(leg) => held(Darting {
            legs: vec![leg],
            rubber: leg.cm,
        }),
        Err(why) => refused(Gesture::Idle, why),
    }
}

/// Reduces one event against the wedge being cut.
///
/// Pure, like the reducer that traces a line: the dart comes back as a command
/// for the caller to apply, and no command goes out until the apex is placed.
pub fn update(
    darting: Darting,
    event: &Input,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    match event {
        Input::Down(_, mods) if mods.space => held(darting),
        Input::Down(at, mods) => pressed(darting, *at, *mods, ctx),
        Input::Move(at, mods) => {
            let rubber = caught(*at, anchor(&darting, *at, ctx), *mods, ctx).at;
            held(Darting { rubber, ..darting })
        }
        // Nothing of it reached the document, so there is nothing to unwind.
        Input::Key(Key::Escape, _) => (Gesture::Idle, Vec::new(), Feedback::default()),
        Input::Key(Key::Backspace | Key::Delete, _) => {
            let mut darting = darting;
            darting.legs.pop();
            if darting.legs.is_empty() {
                return (Gesture::Idle, Vec::new(), Feedback::default());
            }
            held(darting)
        }
        Input::Up(..) | Input::Key(..) | Input::Text(_) => held(darting),
    }
}

/// A press: the first two put the legs of the wedge in the contour, and the
/// third puts its apex in the cloth and cuts the dart.
fn pressed(
    darting: Darting,
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    if darting.legs.len() < LEGS {
        return match leg(at, darting.legs.first(), mods, ctx) {
            Ok(leg) => {
                let mut darting = darting;
                darting.rubber = leg.cm;
                darting.legs.push(leg);
                held(darting)
            }
            Err(why) => refused(Gesture::Darting(darting), why),
        };
    }
    match apex(at, &darting, mods, ctx) {
        Ok(apex) => cut(&darting, apex, ctx),
        Err(why) => refused(Gesture::Darting(darting), why),
    }
}

/// Where a press puts one leg of the wedge, or why it puts none.
///
/// On the contour and nowhere else: the wedge is taken out of the edge, so a
/// leg is a place along one tract of it. Both legs go on one tract because the
/// three nodes of a wedge stand together in the contour — legs on two tracts
/// would swallow every node between them. A tract that bends is refused rather
/// than straightened: cutting a curve in three would have to hand the halves
/// handles, and a wedge node carries no new ones. That is asked of the first
/// leg alone, because the second one lands on the tract the first already
/// cleared.
fn leg(
    at: Pos2,
    first: Option<&Leg>,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> Result<Leg, &'static str> {
    let anchor = first.map_or_else(|| ctx.view.to_document(at), |leg| leg.cm);
    let caught = caught(at, anchor, mods, ctx);
    let Some(SnapKind::Edge { from, t }) = caught.kind else {
        return Err(OFF_EDGE);
    };
    if let Some(first) = first {
        if first.from != from {
            return Err(OTHER_TRACT);
        }
        if pick::away(first.cm, caught.at) <= f64::EPSILON {
            return Err(ONE_PLACE);
        }
    } else if ctx.bends.iter().any(|bend| bend.node == from) {
        return Err(BENT);
    }
    Ok(Leg {
        from,
        t,
        cm: caught.at,
    })
}

/// Where a press puts the apex, or why it puts none.
///
/// Inside the cloth: an apex caught on the contour would be a wedge with no
/// width at its deep end, and the seam that closes it would have a side of no
/// length for the solver to pair.
fn apex(
    at: Pos2,
    darting: &Darting,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> Result<[f64; 2], &'static str> {
    let caught = caught(at, anchor(darting, at, ctx), mods, ctx);
    let on_the_edge = matches!(caught.kind, Some(SnapKind::Node(_) | SnapKind::Edge { .. }));
    if on_the_edge || !tract::covers(ctx.tracts, caught.at) {
        return Err(OFF_CLOTH);
    }
    Ok(caught.at)
}

/// The dart itself: the whole wedge goes out as one entry of the history.
///
/// The keys the command's own `Dart` names are the ones this edit is about to
/// issue — the apply reads them off the cut it makes and takes only the fold
/// from here — and they are written down rather than left blank so that the
/// panel can open on the dart the hand has just made, the way a traced line's
/// key is.
fn cut(
    darting: &Darting,
    apex: [f64; 2],
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    let Some((first, second)) = darting.ordered() else {
        return held(darting.clone());
    };
    let Some(nodes) = wedge::stated((first, second), apex, ctx) else {
        return held(darting.clone());
    };
    let wedge = DartWedge {
        piece: ctx.piece,
        after: Some(first.from),
        nodes,
    };
    let issued = ctx.doc.points.issued();
    let key = |offset: u32| PointKey::new(issued + offset, 0);
    let dart = Dart {
        apex: key(1),
        legs: (key(0), key(2)),
        seam: SeamKey::new(ctx.doc.seams.issued(), 0),
        fold: darting.fold(),
    };
    let command = Command::AddDart {
        identity: Identity::New,
        dart,
        wedge: Box::new(wedge),
    };
    (
        Gesture::Idle,
        vec![command],
        Feedback {
            stack: Some(Stack::Once(PUT_ON)),
            select: Some(Selection::point(key(1))),
            ..Feedback::default()
        },
    )
}

/// Where a press lands, run through the mat's own snap ladder.
///
/// The same ladder every other gesture catches on, so what the pointer reports
/// and what the press takes cannot disagree. The handles on show are the chosen
/// selection's, and cutting a dart chooses nothing until it is cut.
fn caught(at: Pos2, anchor: [f64; 2], mods: Mods, ctx: &EditContext<'_>) -> Snapped {
    let cfg = SnapConfig {
        on: ctx.snap.on && !mods.ctrl,
        axis: mods.shift,
        ..ctx.snap
    };
    let shown = curve::handles(ctx.bends, &ctx.selection);
    snap::resolve(
        ctx.view.to_document(at),
        &SnapContext {
            nodes: ctx.nodes,
            handles: &shown,
            tracts: ctx.tracts,
            held: &[],
            anchor,
            scale: ctx.view.scale().max(f64::EPSILON),
        },
        cfg,
    )
}

/// What an axis-held press measures from: the leg pressed last.
fn anchor(darting: &Darting, at: Pos2, ctx: &EditContext<'_>) -> [f64; 2] {
    darting
        .legs
        .last()
        .map_or_else(|| ctx.view.to_document(at), |leg| leg.cm)
}

/// The wedge as it stands, with nothing to say.
fn held(darting: Darting) -> (Gesture, Vec<Command>, Feedback) {
    (Gesture::Darting(darting), Vec::new(), Feedback::default())
}

/// Nothing done, and the reason said where the person will read it.
fn refused(gesture: Gesture, why: &'static str) -> (Gesture, Vec<Command>, Feedback) {
    (
        gesture,
        Vec::new(),
        Feedback {
            refused: Some(why),
            ..Feedback::default()
        },
    )
}

#[cfg(test)]
mod tests;
