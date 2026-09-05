use eframe::egui::Pos2;
use toile_engine::draft::{
    Command, Doc, Draft, MeasureSet, Piece, PieceKey, Point, PointKey, Segment, Winding,
};

use super::super::curve::{self, Bend};
use super::super::gesture::{EditContext, Mods};
use super::super::snap::SnapConfig;
use super::super::state::{Selection, Tool};
use super::super::tract::{self, Tract};
use super::super::view::View;

/// The corners of a ten centimetre square, in contour order, clockwise on the
/// page: y grows downward, so this is the winding the document declares.
pub(super) const SQUARE: [[f64; 2]; 4] = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];

/// The four handles, in the order they are inserted.
///
/// The first and the last are the two that meet at the top left corner, and
/// they are exact reflections of each other about it: that corner is a smooth
/// node, which is the only shape the tangent pairing has anything to say
/// about. Both curves bow outward, so the contour stays a simple polygon.
pub(super) const HANDLES: [[f64; 2]; 4] = [[3.0, -1.0], [7.0, -1.0], [0.0, 6.0], [-3.0, 1.0]];

/// How finely each of the two bent tracts is flattened.
pub(super) const SAMPLES: u16 = 8;

/// A square with two of its tracts bent, on the table.
pub(super) struct Table {
    pub(super) draft: Draft,
    pub(super) piece: PieceKey,
    pub(super) nodes: Vec<(PointKey, [f64; 2])>,
    pub(super) tracts: Vec<Tract>,
    bends: Vec<Bend>,
}

/// The square as a document: four corners, four handles, two bent tracts.
pub(super) fn bent() -> Table {
    let mut doc = Doc::new(MeasureSet::new("Cuadro", [("cintura", 84.0)]));
    let corners: Vec<PointKey> = SQUARE
        .iter()
        .map(|&[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let handles: Vec<PointKey> = HANDLES
        .iter()
        .map(|&[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Cuadro", corners, Winding::Cw));
    let held = doc.pieces.get_mut(piece).expect("the piece is live");
    for (index, pair) in [(0, (0, 1)), (3, (2, 3))] {
        held.contour[index].segment = Segment::Cubic {
            out: handles[pair.0],
            into: handles[pair.1],
        };
        held.contour[index].samples = SAMPLES;
    }
    let draft = Draft::from_doc(doc).expect("the square resolves");
    assert!(
        draft.defects(piece).is_empty(),
        "{:?}",
        draft.defects(piece)
    );
    Table {
        nodes: draft.points_cm(piece).to_vec(),
        tracts: tract::of(&draft, piece),
        bends: curve::bends(&draft, piece),
        draft,
        piece,
    }
}

impl Table {
    /// The context a gesture reduces against, with `chosen` already in hand.
    pub(super) fn holding(&self, chosen: Selection, tool: Tool) -> EditContext<'_> {
        EditContext {
            doc: self.draft.doc(),
            piece: self.piece,
            nodes: &self.nodes,
            tracts: &self.tracts,
            bends: &self.bends,
            selection: chosen,
            tool,
            view: View::default(),
            snap: SnapConfig {
                on: false,
                ..SnapConfig::default()
            },
        }
    }

    /// The same, with the snap live: the ladder is what the test is after.
    pub(super) fn snapping(&self, chosen: Selection, tool: Tool) -> EditContext<'_> {
        EditContext {
            snap: SnapConfig::default(),
            ..self.holding(chosen, tool)
        }
    }

    /// The two handles that meet at the top left corner: the one leaving it
    /// and the one arriving at it.
    pub(super) fn pair(&self) -> (PointKey, PointKey) {
        (self.bends[0].out.0, self.bends[1].into.0)
    }

    /// The square with these commands applied, as a fresh draft.
    pub(super) fn after(&self, commands: &[Command]) -> Draft {
        let mut draft = self.draft.clone();
        play(&mut draft, commands);
        draft
    }
}

/// Where a point of the document sits on the glass.
pub(super) fn on_glass(at: [f64; 2]) -> Pos2 {
    View::default().to_screen(at)
}

/// A distance in centimetres, on the glass.
pub(super) fn glass(cm: f64) -> f32 {
    (cm * View::default().scale()) as f32
}

/// Alt, held.
pub(super) fn alt() -> Mods {
    Mods {
        alt: true,
        ..Mods::default()
    }
}

/// Applies one frame's commands to a draft the test is playing a drag into.
pub(super) fn play(draft: &mut Draft, commands: &[Command]) {
    for command in commands {
        draft.edit(command.clone()).expect("the edit is accepted");
    }
}

/// Where a `MovePoint` puts its point, in centimetres.
pub(super) fn moved_to(command: &Command, draft: &Draft) -> (PointKey, [f64; 2]) {
    let Command::MovePoint { point, to } = command else {
        panic!("a drag frame moves a point: {command:?}");
    };
    let at = [0, 1].map(|k| to[k].eval(draft.env()).expect("the binding resolves"));
    (*point, at)
}
