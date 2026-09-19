use toile_engine::draft::{Draft, EdgeRange, Seam, SeamKind};
use toile_engine::session::SeamFault;

/// Why a seam never reached the cloth: an end that is not on its own contour.
const LOOSE: &str = "un extremo no cae sobre un nodo de su pieza";
/// The same, for a side with no length between its two ends.
const PINCHED: &str = "los dos extremos caen en el mismo punto";

/// What the two sides of a seam measure, in centimetres.
///
/// One judge for every panel that renders a verdict about a seam: the table
/// under the drape and the inspector over the drawing read the same lengths
/// against the same tolerance, so they can never disagree about one seam.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lengths {
    /// Side A, walked head to tail along its contour.
    pub a: f64,
    /// Side B, walked the same way.
    pub b: f64,
}

impl Lengths {
    /// Both sides measured, when the draft can answer for all four anchors.
    pub fn of(draft: &Draft, seam: &Seam) -> Option<Lengths> {
        let (a, b) = side_cm(draft, &seam.a).zip(side_cm(draft, &seam.b))?;
        Some(Lengths { a, b })
    }

    /// How much longer side A is than side B; negative when B is the longer.
    pub fn delta(self) -> f64 {
        self.a - self.b
    }

    /// Whether the two lengths agree, by the seam's own rule.
    ///
    /// `None` for a gathered seam: it is judged on a ratio of a long side to a
    /// short one, and the document does not say which of the two is which. A
    /// mark drawn from a guess about that is exactly the verdict a panel must
    /// not render.
    pub fn meets(self, draft: &Draft, seam: &Seam) -> Option<bool> {
        if matches!(seam.kind, SeamKind::Gathered { .. }) {
            return None;
        }
        let excess = self.delta().abs() - seam.expected_cm();
        Some(excess.abs() <= tolerance_cm(draft, seam))
    }
}

/// The centimetres of mismatch a seam carries before it complains: its own
/// when it has one, the document's variable otherwise.
pub fn tolerance_cm(draft: &Draft, seam: &Seam) -> f64 {
    seam.tolerance
        .or_else(|| draft.env().value(seam.tolerance_variable()))
        .unwrap_or(Seam::DEFAULT_TOLERANCE_CM)
}

/// What one side measures along its piece's flattened contour, in centimetres.
///
/// The walk is the one the contour runs, head to tail, over the same fractions
/// the solver pairs the two sides on; a side that passes the closure is as long
/// as that walk and never as short as the gap it leaves.
pub fn side_cm(draft: &Draft, range: &EdgeRange) -> Option<f64> {
    let piece = range.piece()?;
    let head = draft.anchor_fraction(&range.head)?;
    let tail = draft.anchor_fraction(&range.tail)?;
    Some((tail - head).rem_euclid(1.0) * draft.perimeter_cm(piece))
}

/// The two nodes a side runs between, under the names its piece shows.
pub fn name(draft: &Draft, range: &EdgeRange) -> Option<String> {
    let doc = draft.doc();
    let piece = range.piece()?;
    let head = doc.label_of(piece, range.head.from)?;
    let tail = doc.label_of(piece, range.tail.from)?;
    Some(format!("{head} → {tail}"))
}

/// Why the engine could not pair a seam onto the cloth, in the panels' words.
///
/// The fault is the engine's to find and the interface's to name, so the
/// wording is here and not in the error the engine raises.
pub fn why(fault: &SeamFault) -> &'static str {
    match fault {
        SeamFault::Unanchored => LOOSE,
        SeamFault::EmptyRange => PINCHED,
    }
}
