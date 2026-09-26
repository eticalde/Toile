use std::collections::BTreeMap;

use crate::{
    Axis, Binding, BodyShape, Dart, DartKey, DartWedge, EdgeAnchor, Elastic, ElasticKey, Grain,
    Hang, HangKey, Identity, InternalLine, LineEdit, LineKey, LineKind, MannequinKey, MeasureSet,
    Notch, NotchKey, Origin, Piece, PieceKey, Pin, PinKey, Placement, Point, PointKey, Seam,
    SeamKey, SegmentEdit, Symmetry, SymmetryKey, VariableKey,
};

/// A reversible edit to the document.
///
/// Nothing here is addressed by index: every command names what it edits by
/// key, so the undo stack survives an insertion, a deletion and their undo.
/// The inverse of a deletion carries the whole payload, the key included.
#[allow(
    missing_docs,
    reason = "each variant names its edit, and its fields name themselves"
)]
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Binds both coordinates of a point.
    MovePoint { point: PointKey, to: [Binding; 2] },
    /// Binds one coordinate of a point.
    SetBinding {
        point: PointKey,
        axis: Axis,
        to: Binding,
    },
    /// Binds a pattern variable.
    SetVariable { variable: VariableKey, to: Binding },
    /// Writes one measurement of one body.
    SetMeasure {
        mannequin: MannequinKey,
        name: String,
        to: f64,
    },
    /// Chooses the body the pattern resolves against.
    ResolveWith { mannequin: MannequinKey },
    /// Gives a body the phenotype it is generated from, or takes it away.
    SetPhenotype {
        mannequin: MannequinKey,
        to: Option<BodyShape>,
    },
    /// Replaces what a body carries from the library: its tape, its phenotype
    /// and its link. The name stays the one the document gave it.
    RefreshMannequin {
        mannequin: MannequinKey,
        values: BTreeMap<String, f64>,
        phenotype: Option<BodyShape>,
        origin: Option<Origin>,
    },
    /// Adds a body the pattern can be resolved against.
    AddMannequin {
        identity: Identity<MeasureSet>,
        mannequin: MeasureSet,
    },
    /// Takes a body away, never the one the pattern resolves against.
    RemoveMannequin { mannequin: MannequinKey },
    /// Renames a body.
    RenameMannequin { mannequin: MannequinKey, to: String },
    /// Adds a node to a contour, after another node or at its head.
    ///
    /// The tract travels as an edit rather than as a `Segment`, for the reason
    /// `SetSegment` does: a curve's handles are points of the document, and an
    /// insertion that cuts a bending tract in two brings two of them with it.
    InsertNode {
        piece: PieceKey,
        after: Option<PointKey>,
        identity: Identity<Point>,
        value: Point,
        segment: SegmentEdit,
        samples: u16,
    },
    /// Takes a node out of a contour, and its point out of the document.
    RemoveNode { piece: PieceKey, node: PointKey },
    /// Changes what runs between a node and the next, handles and all.
    ///
    /// It leaves `samples` where it found it: how finely a tract is flattened
    /// is `SetSamples`, and the tool that draws a curve emits both inside one
    /// gesture — the count first, because bending a tract sampled at one
    /// point would give it handles and go on drawing its chord.
    SetSegment {
        piece: PieceKey,
        node: PointKey,
        to: SegmentEdit,
    },
    /// Changes how finely a tract is flattened, within `SAMPLES`.
    SetSamples {
        piece: PieceKey,
        node: PointKey,
        to: u16,
    },
    /// Puts a piece on the table.
    AddPiece {
        identity: Identity<Piece>,
        piece: Piece,
    },
    /// Takes a piece off the table.
    RemovePiece { piece: PieceKey },
    /// Sews two stretches of contour together.
    AddSeam {
        identity: Identity<Seam>,
        seam: Seam,
    },
    /// Unpicks a seam.
    RemoveSeam { seam: SeamKey },
    /// Holds a stretch of contour in, to a ratio of the length it was drawn at.
    AddElastic {
        identity: Identity<Elastic>,
        elastic: Elastic,
    },
    /// Lets that stretch out again.
    RemoveElastic { elastic: ElasticKey },
    /// Writes the ratio an elastic holds its stretch to.
    SetElasticRatio { elastic: ElasticKey, to: f64 },
    /// Writes how hard it holds it.
    SetElasticStrength { elastic: ElasticKey, to: f64 },
    /// Hangs a stretch of contour from a station of the body.
    AddHang {
        identity: Identity<Hang>,
        hang: Hang,
    },
    /// Lets that stretch off the ring.
    RemoveHang { hang: HangKey },
    /// Writes which of the body's rings a stretch hangs from.
    SetHangStation { hang: HangKey, to: String },
    /// Draws a line on a piece that the pattern does not cut.
    ///
    /// The line travels as an edit rather than as an `InternalLine`, for the
    /// reason `SetSegment` does: a span that bends hangs on two handles that
    /// are points of the document, and drawing the line is what creates them.
    AddLine {
        identity: Identity<InternalLine>,
        line: Box<LineEdit>,
    },
    /// Takes that line off the piece.
    RemoveLine { line: LineKey },
    /// Writes what an internal line is for.
    SetLineKind { line: LineKey, to: LineKind },
    /// Names an internal line, or takes its name away.
    LabelLine { line: LineKey, to: Option<String> },
    /// Marks a contour, and the facing contour with it.
    AddNotch {
        identity: Identity<Notch>,
        notch: Notch,
        mate: Option<(Identity<Notch>, Notch)>,
    },
    /// Slides a notch along its contour.
    MoveNotch { notch: NotchKey, to: EdgeAnchor },
    /// Takes a notch off a contour.
    RemoveNotch { notch: NotchKey },
    /// Cuts a dart, with the wedge it takes out of the contour.
    AddDart {
        identity: Identity<Dart>,
        dart: Dart,
        wedge: Box<DartWedge>,
    },
    /// Closes a dart back up.
    RemoveDart { dart: DartKey },
    /// Folds or mirrors a piece on an axis.
    AddSymmetry {
        identity: Identity<Symmetry>,
        symmetry: Symmetry,
    },
    /// Drops an axis of symmetry.
    RemoveSymmetry { symmetry: SymmetryKey },
    /// Renames a piece.
    RenamePiece { piece: PieceKey, to: String },
    /// Turns the grain line of a piece.
    SetGrain { piece: PieceKey, to: Grain },
    /// Moves a piece on the product overview, or hands it back to the
    /// overview's own layout.
    PlacePiece {
        piece: PieceKey,
        to: Option<Placement>,
    },
    /// Names a point, or takes its name away.
    LabelPoint { point: PointKey, to: Option<String> },
    /// Decides whether the drawing shows a point's name unasked.
    ShowLabel { point: PointKey, to: bool },
    /// Holds a piece of cloth to a place in space.
    SetPin { identity: Identity<Pin>, pin: Pin },
    /// Lets it go.
    ClearPin { pin: PinKey },
}
