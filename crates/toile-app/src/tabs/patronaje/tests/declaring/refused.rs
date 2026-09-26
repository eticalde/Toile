use toile_engine::draft::{
    Axis, Binding, Command, ContourNode, Handle, Handles, Identity, Point, PointKey, SegmentEdit,
};

use super::super::super::dart::NOWHERE;
use super::super::super::dart::declare::{
    CURVED, DARTED, IN_LINE, NODES, NOT_BEYOND, NOT_NEXT, SAME_NODE,
};
use super::super::super::gesture::Gesture;
use super::super::studio::Studio;
use super::{contour, drafted, drafted_from, press_nodes};

/// How far inside the paper of the back the bare-cloth press lands, measured
/// from the apex of the drawn wedge: clear of every tract and of every node.
const INTO_THE_CLOTH: [f64; 2] = [8.0, 8.0];

/// What the back looked like before a press was refused, so that every one of
/// these can be held against it.
struct Untouched {
    contour: Vec<ContourNode>,
    revision: u64,
}

/// The back with the wedge drawn in, the dart tool in hand, and a note of what
/// nothing is allowed to change.
fn ready() -> (Studio, Untouched, [PointKey; NODES]) {
    let (studio, piece, nodes) = drafted();
    let before = Untouched {
        contour: contour(&studio, piece),
        revision: studio.session.revision(),
    };
    (studio, before, nodes)
}

/// Whatever the last press said it would not do, with nothing of it written.
fn said(studio: &Studio, before: &Untouched) -> Option<String> {
    let piece = studio.state.active.expect("a piece is in front");
    assert!(studio.doc().darts.is_empty(), "no dart was declared");
    assert_eq!(&contour(studio, piece), &before.contour, "nor a node moved");
    assert_eq!(
        studio.session.revision(),
        before.revision,
        "nothing reached the document, so nothing could be refused there"
    );
    studio.state.refused.clone()
}

/// Where a place a few centimetres from a node of the drawn wedge falls on the
/// glass.
fn off(studio: &Studio, node: PointKey, away: [f64; 2]) -> eframe::egui::Pos2 {
    let at = studio
        .session
        .draft()
        .expect("a product is open")
        .resolved(node)
        .expect("the node resolves");
    studio
        .state
        .view
        .to_screen([0, 1].map(|axis| at[axis] + away[axis]))
}

/// A press on the bare cloth opens neither of the tool's gestures, and the bar
/// names the two places a press could have gone.
#[test]
fn a_press_on_the_cloth_opens_neither_gesture_and_names_both() {
    let (mut studio, before, nodes) = ready();
    let at = off(&studio, nodes[1], INTO_THE_CLOTH);
    studio.click(at);
    studio.frame(Vec::new());
    assert_eq!(studio.state.gesture, Gesture::Idle);
    assert_eq!(said(&studio, &before).as_deref(), Some(NOWHERE));
}

/// The same node pressed twice chooses nothing the second time, and the nodes
/// already chosen are kept.
#[test]
fn the_same_node_pressed_twice_is_refused() {
    let (mut studio, before, nodes) = ready();
    for _ in 0..2 {
        let at = studio.on_glass(nodes[0]);
        studio.click(at);
    }
    studio.frame(Vec::new());
    assert_eq!(said(&studio, &before).as_deref(), Some(SAME_NODE));
    assert_eq!(held(&studio).len(), 1, "the first node is kept");
}

/// The second press goes to a node beside the first, either way round the
/// contour: three nodes that do not stand together are no wedge.
#[test]
fn a_second_node_that_is_not_beside_the_first_is_refused() {
    let (mut studio, before, nodes) = ready();
    let apart = named(&studio, "bajo_lat_tras");
    for node in [nodes[0], apart] {
        let at = studio.on_glass(node);
        studio.click(at);
    }
    studio.frame(Vec::new());
    assert_eq!(said(&studio, &before).as_deref(), Some(NOT_NEXT));
    assert_eq!(held(&studio).len(), 1, "the first node is kept");
}

/// The third press goes past the apex on the side the first two chose, so there
/// is exactly one node it can be.
#[test]
fn a_third_node_that_does_not_carry_the_walk_on_is_refused() {
    let (mut studio, before, nodes) = ready();
    let back = named(&studio, "cintura_cb");
    for node in [nodes[0], nodes[1], back] {
        let at = studio.on_glass(node);
        studio.click(at);
    }
    studio.frame(Vec::new());
    assert_eq!(said(&studio, &before).as_deref(), Some(NOT_BEYOND));
    assert_eq!(held(&studio).len(), 2, "both nodes are kept");
}

/// A node one dart already names is not one another dart may name.
///
/// Two darts over one node are two seams pulling one place and two mouths
/// crossing on one printed sheet, and the plainest case is the same wedge
/// declared twice — a press that landed twice.
#[test]
fn a_node_a_dart_already_names_refuses_the_next_declaration() {
    let (mut studio, _, nodes) = drafted();
    press_nodes(&mut studio, nodes);
    let revision = studio.session.revision();
    let at = studio.on_glass(nodes[2]);
    studio.click(at);
    studio.frame(Vec::new());
    assert_eq!(studio.state.refused.as_deref(), Some(DARTED));
    assert_eq!(studio.state.gesture, Gesture::Idle, "nothing was opened");
    assert_eq!(studio.doc().darts.len(), 1, "and no second dart");
    assert_eq!(studio.session.revision(), revision, "nothing was written");
}

/// The nodes the declaration in progress is holding.
fn held(studio: &Studio) -> Vec<(PointKey, [f64; 2])> {
    match &studio.state.gesture {
        Gesture::Declaring(held) => held.nodes.clone(),
        other => panic!("a wedge is being declared: {other:?}"),
    }
}

/// One node of the piece in front, by the name the block gave it.
fn named(studio: &Studio, label: &str) -> PointKey {
    let piece = studio.state.active.expect("a piece is in front");
    studio
        .doc()
        .shows_label(piece, label)
        .expect("the block names it")
}

/// A wedge the drawing still shows standing together, but the contour no longer
/// does, is refused here and in Spanish.
///
/// A piece that stops resolving keeps its last good geometry and gains a
/// defect, so the drawn list the pointer is tested against can be a node short
/// of the contour the document counts seats in. Read the seats off the drawing
/// and the three look consecutive when they are not: the press goes through and
/// the document sends the refusal back in its own language.
#[test]
fn a_wedge_the_contour_no_longer_holds_together_is_refused_in_spanish() {
    let (mut studio, piece, nodes) = drafted();
    let elsewhere = contour(&studio, piece)
        .iter()
        .map(|node| node.point)
        .find(|point| !nodes.contains(point))
        .expect("the back draws more than the wedge");
    studio
        .session
        .edit(Command::SetBinding {
            point: elsewhere,
            axis: Axis::X,
            to: Binding::parse("no_existe_esta_medida + 1").expect("it parses"),
        })
        .expect("a binding is written whether or not it resolves");
    studio
        .session
        .edit(Command::InsertNode {
            piece,
            after: Some(nodes[0]),
            identity: Identity::New,
            value: Point::at(Binding::literal(46.0), Binding::literal(-101.0)),
            segment: SegmentEdit::Line,
            samples: 1,
        })
        .expect("nothing is drawn over those nodes yet");
    studio.frame(Vec::new());
    let drawn = studio.session.draft().expect("open").points_cm(piece).len();
    assert!(
        drawn < contour(&studio, piece).len(),
        "the drawing is a node short of the contour, which is the whole trap"
    );

    press_nodes(&mut studio, nodes);
    assert_eq!(
        studio.state.refused.as_deref(),
        Some(NOT_NEXT),
        "the apex is two seats from the leg now, and the bar says so in Spanish"
    );
    assert_eq!(
        studio.doc().darts.iter().count(),
        0,
        "and no dart was written"
    );
}

/// Three nodes standing in one line enclose nothing, so they are not a wedge.
///
/// Reachable in eight presses from an empty mat: the Punto tool along a
/// straight hem, then the Pinza tool on the three it left. The document holds
/// such a dart and the printed sheet draws its marks, so the refusal belongs
/// here.
#[test]
fn three_nodes_in_one_line_are_not_a_wedge() {
    const IN_A_ROW: [(&str, &str, &str); NODES] = [
        ("fila_a", "origen_tras + 4", "0"),
        ("fila_pico", "origen_tras + 6", "0"),
        ("fila_b", "origen_tras + 8", "0"),
    ];
    let (mut studio, _, nodes) = drafted_from(IN_A_ROW);
    press_nodes(&mut studio, nodes);
    assert_eq!(studio.state.refused.as_deref(), Some(IN_LINE));
    assert_eq!(
        studio.doc().darts.iter().count(),
        0,
        "and no dart was written"
    );
}

/// A wedge whose side bends is refused, as the cut refuses a bending tract.
///
/// The sheet sets a dart's arrow in by three millimetres on the argument that
/// both of its sides are straight, so a curved side puts that mark on paper the
/// cutter takes off.
#[test]
fn a_wedge_whose_side_bends_is_refused() {
    let (mut studio, piece, nodes) = drafted();
    // The handles go where this file already trusts nothing else to be: clear
    // of every node, so the three presses still catch the three nodes.
    let apex = studio
        .session
        .draft()
        .expect("open")
        .resolved(nodes[1])
        .expect("the apex resolves");
    let bend = |away: f64| Handle {
        identity: Identity::New,
        value: Point::at(
            apex[0] + INTO_THE_CLOTH[0] + away,
            apex[1] + INTO_THE_CLOTH[1] + away,
        ),
    };
    studio
        .session
        .edit(Command::SetSamples {
            piece,
            node: nodes[1],
            to: 16,
        })
        .expect("the tract takes more samples");
    studio
        .session
        .edit(Command::SetSegment {
            piece,
            node: nodes[1],
            to: SegmentEdit::Cubic(Box::new(Handles {
                out: bend(0.0),
                into: bend(1.0),
            })),
        })
        .expect("the tract between the apex and the second leg bends");
    studio.frame(Vec::new());

    press_nodes(&mut studio, nodes);
    assert_eq!(studio.state.refused.as_deref(), Some(CURVED));
    assert_eq!(
        studio.doc().darts.iter().count(),
        0,
        "and no dart was written"
    );
}
