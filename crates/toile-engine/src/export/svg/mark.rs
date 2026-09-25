use std::fmt::Write;

use super::super::units::{CAPTION, INK, TITLE, beside, millimetres, number};
use super::xml::escape;

/// The typeface the drawing asks for, and the one it settles for.
const FONT: &str = "sans-serif";

/// The names the piece carries: its own, over its top corner, and the node
/// names the drafter gave it, beside the nodes that hold them.
///
/// The names arrive as the name and the place together, so there is no
/// re-pairing to get wrong: the flattened cut line is the wrong length and no
/// longer the right type, and a mismatch would have truncated in silence.
pub(super) fn names(out: &mut String, corner: [f64; 2], named: &[(String, [f64; 2])], name: &str) {
    text(out, corner, TITLE, &escape(name));
    for (label, at) in named {
        text(out, beside(millimetres(*at)), CAPTION, &escape(label));
    }
}

/// One line of type, anchored at its left baseline.
fn text(out: &mut String, at: [f64; 2], size: f64, body: &str) {
    let _ = writeln!(
        out,
        "    <text x=\"{}\" y=\"{}\" font-family=\"{FONT}\" font-size=\"{}\" \
         fill=\"{INK}\">{body}</text>",
        number(at[0]),
        number(at[1]),
        number(size)
    );
}

#[cfg(test)]
mod tests {
    use super::super::super::units::{CAPTION, MM_PER_CM, beside, number};
    use super::super::to_svg;
    use super::FONT;
    use crate::draft::{Draft, block};

    /// Every node name is written at the node that carries it, and not at some
    /// sample of the line those nodes draw.
    ///
    /// Asserting the name appears somewhere in the drawing is not enough: the
    /// flattening is five times longer on this piece, and pairing the names
    /// against it would put every one of them at a place no node is.
    #[test]
    fn a_node_name_is_written_beside_its_own_node() {
        let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
        let piece = draft
            .doc()
            .piece_named(block::FRONT)
            .expect("the block draws one piece");
        let drawing = to_svg(&draft).expect("the block draws");
        let mut named = 0;
        for &(key, [x, y]) in draft.points_cm(piece) {
            let Some(label) = draft.doc().label_of(piece, key) else {
                continue;
            };
            let at = beside([x * MM_PER_CM, y * MM_PER_CM]);
            let want = format!(
                "x=\"{}\" y=\"{}\" font-family=\"{FONT}\" font-size=\"{}\" \
                 fill=\"#000000\">{label}</text>",
                number(at[0]),
                number(at[1]),
                number(CAPTION)
            );
            assert!(drawing.contains(&want), "`{label}` is not at its node");
            named += 1;
        }
        assert!(named >= 2, "the block names nodes of its own");
    }
}
