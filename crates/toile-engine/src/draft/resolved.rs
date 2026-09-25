use toile_doc::PointKey;

use super::fold::Cloth;

/// A piece as the rest of the program sees it: its nodes, the line they draw
/// once the curves are flattened, and the cloth that line is cut from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Resolved {
    /// The contour nodes in centimetres, y downward, in contour order.
    pub points: Vec<(PointKey, [f64; 2])>,
    /// The whole drawn contour flattened, in centimetres with y downward: the
    /// line the table draws and the pointer catches, curves and all.
    pub flat_cm: Vec<[f64; 2]>,
    /// The same drawn contour in metres, y upward.
    ///
    /// The suffix is here because these two are the same type, hold the same
    /// contour, sit one field apart and differ by a hundred and a sign — not
    /// because the tree carries a rule that every length is suffixed. It does
    /// not: `points` above is centimetres and says nothing, and an unsuffixed
    /// `[f64; 2]` elsewhere may be either unit. Read the suffix as a warning
    /// about this neighbourhood, never as a guarantee about the rest.
    pub drawn_m: Vec<[f64; 2]>,
    /// Where each node opens in the flattening, in contour order.
    pub starts: Vec<usize>,
    /// Flattened arc length in centimetres up to each node, and round to the
    /// first, so the last entry is the drawn contour's perimeter.
    pub cum: Vec<f64>,
    /// The whole cloth, when the piece is drawn against an axis and the drawn
    /// contour is only half of it.
    ///
    /// `None` is the ordinary case and means the drawn contour *is* the cloth,
    /// so a piece nobody folded is bit for bit what it was before folds
    /// existed: the same flattening, the same metres, the same perimeter.
    pub cloth: Option<Cloth>,
}

impl Resolved {
    /// The cloth in centimetres: the whole outline, unfolded.
    pub fn cloth_cm(&self) -> &[[f64; 2]] {
        match &self.cloth {
            Some(cloth) => &cloth.cm,
            None => &self.flat_cm,
        }
    }

    /// The cloth in metres, y upward: what the mesher takes.
    pub fn cloth_m(&self) -> &[[f64; 2]] {
        match &self.cloth {
            Some(cloth) => &cloth.m,
            None => &self.drawn_m,
        }
    }

    /// The cloth's perimeter in centimetres.
    pub fn perimeter(&self) -> f64 {
        match &self.cloth {
            Some(cloth) => cloth.perimeter,
            None => self.cum.last().copied().unwrap_or_default(),
        }
    }
}
