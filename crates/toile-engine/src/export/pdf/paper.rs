#[cfg(test)]
use super::super::units::MM_PER_CM;

/// Points in an inch.
///
/// A definition and not a measurement: `PDF`'s unit of length is one 72nd of
/// an inch, so this number is exact in binary floating point and will not
/// change under anybody's ruler.
const PT_PER_INCH: f64 = 72.0;

/// Millimetres in an inch.
///
/// A definition too: the international inch has been exactly 25.4 mm since
/// 1959. It is the only number in the transform that binary floating point
/// cannot hold, so the division by it below is the whole of the rounding.
const MM_PER_INCH: f64 = 25.4;

/// A sheet of paper: what it is called, and how big it is in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paper {
    /// What the sheet is called, as the printed page says it.
    pub name: &'static str,
    /// How wide the sheet is, in millimetres.
    pub width_mm: f64,
    /// How tall the sheet is, in millimetres.
    pub height_mm: f64,
}

/// A4, whose size ISO 216 states in millimetres: 210 by 297.
pub const A4: Paper = Paper {
    name: "A4",
    width_mm: 210.0,
    height_mm: 297.0,
};

/// Carta, whose size is stated in inches — 8.5 by 11 — so its millimetres are
/// those inches times the exact 25.4, and are not round numbers.
pub const CARTA: Paper = Paper {
    name: "Carta",
    width_mm: 215.9,
    height_mm: 279.4,
};

/// A length in millimetres as a length in points.
///
/// Written out of the two definitions above rather than out of the 2.8346
/// their ratio comes to, so that a reader checks the line against the
/// definitions and never against a constant somebody typed once.
pub(super) fn points(mm: f64) -> f64 {
    mm * PT_PER_INCH / MM_PER_INCH
}

/// A document centimetre as a page point: the ratio the whole of true scale
/// comes down to.
///
/// Not on the path a sheet is written by — a piece reaches the page in the
/// millimetres the paper is laid out in, and `points` is the only conversion on
/// the way — but a distance read back off a page divided by this is a distance
/// in the document, so it is what every proof of scale here measures with.
#[cfg(test)]
pub(super) fn points_of_cm(cm: f64) -> f64 {
    points(cm * MM_PER_CM)
}

impl Paper {
    /// The sheet's own size in points, which is what the page box states.
    pub(super) fn points(self) -> [f64; 2] {
        [points(self.width_mm), points(self.height_mm)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tenth of a micrometre, as a fraction of a point.
    const TIGHT: f64 = 1.0e-4 * PT_PER_INCH / MM_PER_INCH;

    /// An inch of paper is 72 points, both ways round, which is the whole
    /// reason the transform can be checked by eye.
    #[test]
    fn an_inch_of_paper_is_seventy_two_points() {
        assert!((points(MM_PER_INCH) - PT_PER_INCH).abs() < TIGHT);
        assert!((points_of_cm(2.54) - PT_PER_INCH).abs() < TIGHT);
    }

    /// Carta is 8.5 by 11 inches, and its millimetres are only a way of
    /// saying so.
    #[test]
    fn carta_measures_eight_and_a_half_inches_by_eleven() {
        let [width, height] = CARTA.points();
        assert!((width - 8.5 * PT_PER_INCH).abs() < TIGHT, "{width}");
        assert!((height - 11.0 * PT_PER_INCH).abs() < TIGHT, "{height}");
    }

    #[test]
    fn a4_is_two_hundred_and_ten_millimetres_across() {
        assert!((A4.points()[0] - points(210.0)).abs() < TIGHT);
    }
}
