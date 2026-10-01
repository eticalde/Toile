use toile_engine::draft::{
    Command, Doc, EdgeRange, Identity, MeasureSet, Piece, PieceKey, Point, PointKey, Seam,
    SeamOrientation, Winding,
};

/// Where the back is drawn beside the front on the table, in centimetres.
const BESIDE: f64 = 70.0;

/// Half the waistline of cloth one piece carries.
const WAIST: f64 = 24.0;
/// How far across the hem's outer corner sits.
const HEM_OUT: f64 = 18.0;
/// How far across the hem's inner corner sits.
const HEM_IN: f64 = 7.0;
/// Waist to hem down the front.
///
/// Short enough that the hem hangs clear of the ground the body stands on.
/// Let go at the thigh, a leg of a metre starts a quarter of it through the
/// floor plane, and the fold that pushes back up pinches the hem's own sewn
/// pairs from the first substep — a reading of the heap, not of the placement.
const LENGTH: f64 = 70.0;
/// How far below the waist the front's crotch sits.
const RISE: f64 = 20.0;

/// The name the front carries in the product tree.
pub const FRONT: &str = "Delantero";
/// The name the back carries.
pub const BACK: &str = "Trasero";

/// How much longer each of the front's two sides is cut than the back's, in
/// centimetres.
///
/// A declared mismatch and not a second set of coordinates, because what two
/// cuts of this fixture differ by is exactly this pair of numbers: the back's
/// length and rise are solved from them, so the matched cut and the mismatched
/// one are the same draft rule asked for a different ease.
#[derive(Debug, Clone, Copy)]
pub struct Ease {
    /// Centimetres the front's outside leg seam is longer than the back's.
    pub side: f64,
    /// Centimetres the front's inside leg seam is longer than the back's.
    pub inseam: f64,
}

impl Ease {
    /// The two sides cut to the same length, which is the control.
    pub const MATCHED: Ease = Ease {
        side: 0.0,
        inseam: 0.0,
    };

    /// The mismatch the owner's own jeans carry, measured off his file: the
    /// outside leg seam 2.11 cm out and the inside one 0.72 cm, against the
    /// half centimetre his document allows.
    pub const HIS_JEANS: Ease = Ease {
        side: 2.11,
        inseam: 0.72,
    };

    /// What the front's outside leg seam measures, in centimetres.
    pub fn front_side() -> f64 {
        across(HEM_OUT - WAIST, LENGTH)
    }

    /// What the front's inside leg seam measures, in centimetres.
    pub fn front_inseam() -> f64 {
        across(HEM_IN, LENGTH - RISE)
    }

    /// Waist to hem down the back, solved so its outside leg seam comes out
    /// `side` centimetres shorter than the front's.
    ///
    /// Closed form rather than a nudged constant: the back's side runs from
    /// its hem corner to its waist corner across the same `WAIST - HEM_OUT`,
    /// so the length that leaves a given seam is the other leg of that
    /// triangle, and a fixture that solved it by hand would be a number
    /// nobody could re-derive.
    fn back_length(self) -> f64 {
        let seam = Ease::front_side() - self.side;
        other_leg(seam, WAIST - HEM_OUT)
    }

    /// How far below the back's waist its crotch sits, solved the same way
    /// from `inseam` once the back's length is known.
    fn back_rise(self) -> f64 {
        let seam = Ease::front_inseam() - self.inseam;
        self.back_length() - other_leg(seam, HEM_IN)
    }
}

/// The straight line across `x` and `y`, in the units both are given in.
///
/// Written out rather than `hypot`, which is the platform's and not exactly
/// rounded: the fixture's coordinates reach the solver, and a drape has to be
/// the same drape on both architectures. Multiply, add and `sqrt` are exact in
/// IEEE, so this is.
fn across(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// The other leg of a right triangle, from its hypotenuse and one leg: how long
/// the back has to be cut for a seam of `seam` running across `other`.
fn other_leg(seam: f64, other: f64) -> f64 {
    (seam * seam - other * other).sqrt()
}

/// One leg of a trouser as two mirrored pieces, closed by the two seams a
/// pattern cutter sews, with the back cut to leave `ease` on each of them.
///
/// The smallest garment that has the shape of the owner's jeans. Three things
/// about his pattern decide how it is let go and none of them is in the shipped
/// block: the leg narrows from the waist to the hem, the two pieces are drawn
/// running opposite ways round so both seams are opposed, and the two sides of
/// each seam are not the same length. The block's two seams are aligned and
/// both of its sides agree, so nothing in the suite watched any of the three.
pub fn leg(ease: Ease) -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let front = front(&mut doc);
    let back = back(&mut doc, ease);
    sew(&mut doc, front, back);
    doc
}

/// The front: five nodes, the outside leg running waist to hem in contour
/// order and the inside leg running hem to crotch.
fn front(doc: &mut Doc) -> PieceKey {
    let nodes = [
        ("cintura_int", 0.0, 0.0),
        ("cintura_lat", WAIST, 0.0),
        ("bajo_lat", HEM_OUT, LENGTH),
        ("bajo_int", HEM_IN, LENGTH),
        ("tiro", 0.0, RISE),
    ];
    let points = named(doc, &nodes);
    doc.pieces
        .insert(Piece::polygon(FRONT, points, Winding::Cw))
}

/// The back: the same five nodes walked the other way round, which is what
/// makes both of its seam sides run against the front's.
///
/// Drawn beside the front and declared counter-clockwise, because reversing
/// the walk reverses the winding: a piece that said otherwise would be telling
/// the marker the opposite of what its own nodes draw.
fn back(doc: &mut Doc, ease: Ease) -> PieceKey {
    let (length, rise) = (ease.back_length(), ease.back_rise());
    let nodes = [
        ("cintura_int_tras", BESIDE, 0.0),
        ("tiro_tras", BESIDE, rise),
        ("bajo_int_tras", BESIDE + HEM_IN, length),
        ("bajo_lat_tras", BESIDE + HEM_OUT, length),
        ("cintura_lat_tras", BESIDE + WAIST, 0.0),
    ];
    let points = named(doc, &nodes);
    doc.pieces
        .insert(Piece::polygon(BACK, points, Winding::Ccw))
}

/// Inserts a run of labelled points and hands back their keys in order.
fn named(doc: &mut Doc, nodes: &[(&str, f64, f64)]) -> Vec<PointKey> {
    nodes
        .iter()
        .map(|&(label, x, y)| doc.points.insert(Point::at(x, y).named(label)))
        .collect()
}

/// Closes the two pieces into a tube: the outside leg seam and the inside one.
///
/// Both opposed, and not by choice: the front's outside leg runs waist to hem
/// in its own contour order while the back's runs hem to waist, so aligning
/// them would sew a waist to an ankle.
fn sew(doc: &mut Doc, front: PieceKey, back: PieceKey) {
    let side = Seam::plain(
        range(doc, front, "cintura_lat", "bajo_lat"),
        range(doc, back, "bajo_lat_tras", "cintura_lat_tras"),
        SeamOrientation::Opposed,
    );
    let inseam = Seam::plain(
        range(doc, front, "bajo_int", "tiro"),
        range(doc, back, "tiro_tras", "bajo_int_tras"),
        SeamOrientation::Opposed,
    );
    for seam in [side, inseam] {
        Command::AddSeam {
            identity: Identity::New,
            seam,
        }
        .apply(doc)
        .expect("both ends of the fixture's seams are nodes it has just named");
    }
}

/// The stretch of one piece's contour between two of its named nodes.
fn range(doc: &Doc, piece: PieceKey, head: &str, tail: &str) -> EdgeRange {
    let named = |label| {
        doc.shows_label(piece, label)
            .expect("the fixture names every node it goes on to point at")
    };
    EdgeRange::between(piece, named(head), named(tail))
}
