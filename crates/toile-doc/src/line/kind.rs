use serde::{Deserialize, Serialize};

/// What an internal line is for, which is what the pattern draws it as.
///
/// A kind and not a free label, because four readers act on it and none of
/// them can act on prose: the drawing picks a stroke, the cutter decides
/// whether cloth is removed, the mesher decides whether the piece is still one
/// sheet, and the sewer decides whether the needle goes there. The pattern
/// these variants were enumerated from shows why the stroke cannot stand in
/// for the meaning either: it draws a pocket slit solid and a fold dashed, a
/// phone channel dashed and a button solid. Toile keeps the meaning and draws
/// the stroke from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    /// The cloth folds along it, and neither side is cut away.
    ///
    /// The one kind the shape of the piece itself depends on: a waistband
    /// folded at its top is half the cloth it looks like, and whoever unfolds
    /// a piece reads this line to know where the mirror goes.
    Fold,
    /// The needle runs along it through cloth that stays whole.
    ///
    /// A topstitch or a stitched channel. It costs thread, it is sewn once the
    /// piece is assembled, and the cloth is stiffer along its run.
    Stitch,
    /// The cloth is opened along it, inside the contour.
    ///
    /// A pocket mouth, or a cut that lets something through. Cloth comes out
    /// of the middle of the piece, so a piece drawn with one is no longer a
    /// single closed sheet, and whatever finishes the opening is another piece.
    Slit,
    /// A finished hole, as long as the button that passes through it.
    ///
    /// Apart from a slit because its length is a fitting — the button's size,
    /// not an opening someone else's facing will close — and because the
    /// machine that finishes it makes it, rather than the cutter.
    Buttonhole,
    /// Where another piece or a finding lands on this one.
    ///
    /// A yoke line, a flap, a pocket bag, a belt loop, a button. Nothing is
    /// done to the cloth along it: it is matched against the thing that goes
    /// there, which is what tells it from the stitch it looks like.
    Placement,
    /// A direction the draft was reasoned with, carrying nothing.
    ///
    /// A bias line is the plain case. Calling it a placement would answer
    /// "what goes here?" with a line that holds nothing, so it keeps the
    /// variant that promises nothing.
    Reference,
}

impl LineKind {
    /// Whether the cutter opens the cloth along a line of this kind.
    ///
    /// A slit and a buttonhole are holes in the middle of a piece; a fold, a
    /// stitch, a placement and a reference leave the sheet whole. Asked here
    /// rather than matched at each call site, so a kind added later is a
    /// compile error in one place.
    pub fn opens_the_cloth(self) -> bool {
        match self {
            LineKind::Slit | LineKind::Buttonhole => true,
            LineKind::Fold | LineKind::Stitch | LineKind::Placement | LineKind::Reference => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_slit_and_a_buttonhole_take_cloth_out_of_a_piece() {
        assert!(LineKind::Slit.opens_the_cloth());
        assert!(LineKind::Buttonhole.opens_the_cloth());
        for kind in [
            LineKind::Fold,
            LineKind::Stitch,
            LineKind::Placement,
            LineKind::Reference,
        ] {
            assert!(!kind.opens_the_cloth(), "{kind:?}");
        }
    }

    #[test]
    fn a_kind_is_written_as_the_word_a_pattern_calls_it() {
        let written = serde_json::to_string(&LineKind::Buttonhole).expect("a kind writes");
        assert_eq!(written, "\"buttonhole\"");
        let read: LineKind = serde_json::from_str("\"fold\"").expect("the word reads back");
        assert_eq!(read, LineKind::Fold);
    }
}
