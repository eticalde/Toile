/// Which node a press chooses out of the contour, or why it chooses none.
mod choose;

use choose::{chosen, seat, vacant};
use eframe::egui::{Key, Pos2};
use toile_engine::draft::{Command, Dart, DrawnWedge, FoldDirection, Identity, PointKey, SeamKey};

use super::super::gesture::{EditContext, Feedback, Gesture, Input, Mods, Stack};
use super::super::state::Selection;
use super::{caught, refused};

/// The name one dart declared leaves in the undo stack.
pub const DECLARED: &str = "declarar pinza";

/// How many nodes a drawn wedge has, which is what the third press finishes.
pub const NODES: usize = 3;

// Each of these is a refusal the document itself would make, worded here in the
// language the panels are written in, so that no document error is ever put
// into a Spanish sentence. Two of the document's refusals are not here because
// no press can reach them: a piece key nothing answers to and a node of another
// contour, when the ladder behind all of this looks only at the nodes of the
// piece in front.
pub(in crate::tabs::patronaje) const OFF_NODE: &str =
    "una pinza declarada se apoya en nodos que el contorno ya tiene; pulsa uno de ellos";
pub(in crate::tabs::patronaje) const SAME_NODE: &str =
    "ese nodo ya está elegido; el siguiente va al lado, no encima";
pub(in crate::tabs::patronaje) const NOT_NEXT: &str =
    "el pico es el nodo pegado a la pata, hacia un lado o hacia el otro del contorno";
pub(in crate::tabs::patronaje) const NOT_BEYOND: &str = "la segunda pata es el nodo que sigue al pico por el mismo lado: los tres van seguidos en \
     el contorno";
pub(in crate::tabs::patronaje) const DARTED: &str =
    "ese nodo ya es de una pinza, y una cuña no lleva dos";
pub(in crate::tabs::patronaje) const FLAT: &str =
    "dos de esos nodos están escritos en el mismo sitio, y una cuña se saca entre tres";
pub(in crate::tabs::patronaje) const CURVED: &str =
    "entre esos dos nodos el contorno va curvo, y los dos lados de una cuña van rectos";
pub(in crate::tabs::patronaje) const IN_LINE: &str =
    "los tres nodos están en línea, así que esa cuña no tiene nada dentro que cerrar";

/// The wedge being declared, between the presses that name it.
///
/// Nothing has reached the document yet, which is what lets Escape walk away
/// from it with no entry to unwind and Backspace take the last node back for
/// free.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaring {
    /// The nodes pressed so far, in the order they were pressed.
    pub nodes: Vec<(PointKey, [f64; 2])>,
    /// Where the next press would land, in centimetres.
    pub rubber: [f64; 2],
}

/// Opens a declaration on the node a press landed on, or opens none and says
/// why.
pub(super) fn begin(
    node: (PointKey, [f64; 2]),
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    match vacant(ctx.doc, node.0) {
        Ok(()) => held(Declaring {
            nodes: vec![node],
            rubber: node.1,
        }),
        Err(why) => refused(Gesture::Idle, why),
    }
}

/// Reduces one event against the wedge being declared.
///
/// Pure, like the reducer that cuts one: the dart comes back as a command for
/// the caller to apply, and no command goes out until the third node is chosen.
pub fn update(
    declaring: Declaring,
    event: &Input,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    match event {
        Input::Down(_, mods) if mods.space => held(declaring),
        Input::Down(at, mods) => pressed(declaring, *at, *mods, ctx),
        Input::Move(at, mods) => {
            let rubber = caught(*at, anchor(&declaring, *at, ctx), *mods, ctx).at;
            held(Declaring {
                rubber,
                ..declaring
            })
        }
        // Nothing of it reached the document, so there is nothing to unwind.
        Input::Key(Key::Escape, _) => (Gesture::Idle, Vec::new(), Feedback::default()),
        Input::Key(Key::Backspace | Key::Delete, _) => {
            let mut declaring = declaring;
            declaring.nodes.pop();
            if declaring.nodes.is_empty() {
                return (Gesture::Idle, Vec::new(), Feedback::default());
            }
            held(declaring)
        }
        Input::Up(..) | Input::Key(..) | Input::Text(_) => held(declaring),
    }
}

/// A press: each of the three chooses one node the contour already has, and the
/// third declares the dart over them.
fn pressed(
    declaring: Declaring,
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    match chosen(&declaring, at, mods, ctx) {
        Ok(node) => {
            let mut declaring = declaring;
            declaring.rubber = node.1;
            declaring.nodes.push(node);
            if declaring.nodes.len() < NODES {
                return held(declaring);
            }
            declared(&declaring, ctx)
        }
        Err(why) => refused(Gesture::Declaring(declaring), why),
    }
}

/// The dart itself: the record and the thread that shuts the wedge, as one
/// entry of the history.
///
/// Nothing of the drawing is written, which is the whole of why this exists: a
/// node stated as a formula stops being one the moment anything rewrites it.
/// The keys the command's own `Dart` names are the ones it will hold — the
/// apply reads them off the wedge and takes only the fold from here — and they
/// are written down so the panel can open on the dart the hand has just made.
fn declared(declaring: &Declaring, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    let Some((nodes, fold)) = declaring.walked(ctx) else {
        return held(declaring.clone());
    };
    let dart = Dart {
        apex: nodes[1],
        legs: (nodes[0], nodes[2]),
        seam: SeamKey::new(ctx.doc.seams.issued(), 0),
        fold,
    };
    let command = Command::DeclareDart {
        identity: Identity::New,
        dart,
        wedge: DrawnWedge {
            piece: ctx.piece,
            nodes,
        },
    };
    (
        Gesture::Idle,
        vec![command],
        Feedback {
            stack: Some(Stack::Once(DECLARED)),
            select: Some(Selection::point(dart.apex)),
            ..Feedback::default()
        },
    )
}

impl Declaring {
    /// The three nodes in contour order, and which way the wedge is pressed.
    ///
    /// Toward the leg pressed first, exactly as a cut wedge is pressed: walked
    /// forward along the contour that leg is the first of the three, walked
    /// backward it is the last. Which way a sewn dart lies is the one thing
    /// about it the drawing cannot say, and the order of the presses is
    /// information the gesture already has.
    fn walked(&self, ctx: &EditContext<'_>) -> Option<([PointKey; 3], FoldDirection)> {
        let [first, apex, last] = <[(PointKey, [f64; 2]); NODES]>::try_from(self.nodes.as_slice())
            .ok()
            .map(|held| held.map(|(key, _)| key))?;
        Some(if seat(ctx, first)? < seat(ctx, last)? {
            ([first, apex, last], FoldDirection::TowardStart)
        } else {
            ([last, apex, first], FoldDirection::TowardEnd)
        })
    }
}

/// What an axis-held press measures from: the node pressed last.
fn anchor(declaring: &Declaring, at: Pos2, ctx: &EditContext<'_>) -> [f64; 2] {
    declaring
        .nodes
        .last()
        .map_or_else(|| ctx.view.to_document(at), |&(_, cm)| cm)
}

/// The declaration as it stands, with nothing to say.
fn held(declaring: Declaring) -> (Gesture, Vec<Command>, Feedback) {
    (
        Gesture::Declaring(declaring),
        Vec::new(),
        Feedback::default(),
    )
}
