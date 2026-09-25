use std::fmt::Write;

use super::units::{escape, mm};
use crate::draft::{Draft, PieceKey, PointKey};

/// The weight of everything drawn inside a cut line, in millimetres.
const THIN: f64 = 0.2;

/// Type sizes in millimetres: the name of a piece, and the name of a node.
const TITLE: f64 = 5.0;
const CAPTION: f64 = 3.0;

/// The share of the shorter side of a piece that its grain line runs for.
const SHARE: f64 = 0.6;

/// One barb of a grain line's arrow: how long it is, and how far it opens.
const BARB: f64 = 4.0;
const OPENING: f64 = 0.42;

/// The typeface the drawing asks for, and the one it settles for.
const FONT: &str = "sans-serif";

/// The grain line: the direction the warp runs, arrowed at both ends.
///
/// It is drawn from the middle of the piece rather than from a node, because
/// the grain is a property of the cloth under the piece and not of any point
/// on its contour.
pub fn grain(out: &mut String, outline: &[[f64; 2]], radians: f64) {
    let (centre, span) = middle(outline);
    let along = [radians.cos(), radians.sin()];
    let reach = SHARE * span / 2.0;
    let head = [centre[0] + along[0] * reach, centre[1] + along[1] * reach];
    let tail = [centre[0] - along[0] * reach, centre[1] - along[1] * reach];
    let mut path = String::new();
    segment(&mut path, tail, head);
    arrow(&mut path, head, [-along[0], -along[1]]);
    arrow(&mut path, tail, along);
    let _ = writeln!(
        out,
        "    <path d=\"{path}\" fill=\"none\" stroke=\"#000000\" stroke-width=\"{}\"/>",
        mm(THIN)
    );
}

/// The names the piece carries: its own, over its top corner, and the node
/// names the drafter gave it, beside the nodes that hold them.
///
/// Each node arrives as the key and the place together, so there is no
/// re-pairing to get wrong: the flattened cut line is the wrong length and no
/// longer the right type, and a mismatch would have truncated in silence.
pub fn names(
    out: &mut String,
    draft: &Draft,
    piece: PieceKey,
    nodes: &[(PointKey, [f64; 2])],
    name: &str,
) {
    text(out, corner_of(nodes), TITLE, &escape(name));
    for &(key, at) in nodes {
        let Some(label) = draft.doc().label_of(piece, key) else {
            continue;
        };
        let beside = [at[0] + CAPTION / 2.0, at[1] - CAPTION / 2.0];
        text(out, beside, CAPTION, &escape(&label));
    }
}

/// One line of type, anchored at its left baseline.
fn text(out: &mut String, at: [f64; 2], size: f64, body: &str) {
    let _ = writeln!(
        out,
        "    <text x=\"{}\" y=\"{}\" font-family=\"{FONT}\" font-size=\"{}\" \
         fill=\"#000000\">{body}</text>",
        mm(at[0]),
        mm(at[1]),
        mm(size)
    );
}

/// The middle of a contour and the shorter side of the box around it.
fn middle(outline: &[[f64; 2]]) -> ([f64; 2], f64) {
    let (low, high) = box_of(outline.iter().copied());
    let centre = [
        f64::midpoint(low[0], high[0]),
        f64::midpoint(low[1], high[1]),
    ];
    (centre, (high[0] - low[0]).min(high[1] - low[1]))
}

/// Where the name of a piece goes: over the top left of the box around its
/// nodes.
fn corner_of(nodes: &[(PointKey, [f64; 2])]) -> [f64; 2] {
    let (low, _) = box_of(nodes.iter().map(|&(_, at)| at));
    [low[0], low[1] - TITLE / 2.0]
}

/// The box around a run of places, in millimetres.
///
/// An empty run answers with an inverted box, which is what lets the sheet
/// fold several of these together and still see that nothing was drawn. It
/// takes the places one at a time rather than as a slice so a caller holding
/// keys beside them need not strip the pairing apart to ask.
pub(super) fn box_of(places: impl IntoIterator<Item = [f64; 2]>) -> ([f64; 2], [f64; 2]) {
    let mut low = [f64::INFINITY; 2];
    let mut high = [f64::NEG_INFINITY; 2];
    for at in places {
        for axis in 0..2 {
            low[axis] = low[axis].min(at[axis]);
            high[axis] = high[axis].max(at[axis]);
        }
    }
    (low, high)
}

/// One straight run of a path.
fn segment(path: &mut String, from: [f64; 2], to: [f64; 2]) {
    let _ = write!(
        path,
        "M {} {} L {} {} ",
        mm(from[0]),
        mm(from[1]),
        mm(to[0]),
        mm(to[1])
    );
}

/// The two barbs of an arrow at `tip`, opening back along `back`.
fn arrow(path: &mut String, tip: [f64; 2], back: [f64; 2]) {
    for opening in [OPENING, -OPENING] {
        let (sin, cos) = opening.sin_cos();
        let turned = [back[0] * cos - back[1] * sin, back[0] * sin + back[1] * cos];
        let barb = [tip[0] + turned[0] * BARB, tip[1] + turned[1] * BARB];
        segment(path, tip, barb);
    }
}

#[cfg(test)]
mod tests {
    use super::super::to_svg;
    use super::super::units::{MM_PER_CM, mm};
    use super::{CAPTION, FONT};
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
            let beside = [x * MM_PER_CM + CAPTION / 2.0, y * MM_PER_CM - CAPTION / 2.0];
            let want = format!(
                "x=\"{}\" y=\"{}\" font-family=\"{FONT}\" font-size=\"{}\" \
                 fill=\"#000000\">{label}</text>",
                mm(beside[0]),
                mm(beside[1]),
                mm(CAPTION)
            );
            assert!(drawing.contains(&want), "`{label}` is not at its node");
            named += 1;
        }
        assert!(named >= 2, "the block names nodes of its own");
    }
}
