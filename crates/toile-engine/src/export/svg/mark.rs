use std::fmt::Write;

use super::super::block::{self, Room};
use super::super::cut;
use super::super::units::{CAPTION, INK, TITLE, beside, millimetres, number};
use super::xml::escape;
use crate::draft::Piece;

/// The typeface the drawing asks for, and the one it settles for.
const FONT: &str = "sans-serif";

/// What the piece says about itself, line by line and placed, or nothing at
/// all where the drawing has nowhere to say it.
///
/// No size among the lines, which is the one thing the printed sheet adds. A
/// drawing is one page and a piece on it can be measured where it lies; a sheet
/// is cut from its neighbours, so a piece that outgrew it is no longer there to
/// measure.
pub(super) fn said(piece: &Piece, room: &Room<'_>) -> Option<Vec<block::Line>> {
    let handle = cut::handle(piece);
    let cut_out = cut::says(piece);
    let mut lines: Vec<(f64, &str)> = vec![(TITLE, handle.as_str())];
    lines.extend(cut_out.iter().map(|line| (CAPTION, line.as_str())));
    block::laid(&lines, room)
}

/// What a viewer shows for the piece: its name, and where the drawing could
/// not write that piece's own words anywhere but inside the piece beside it,
/// that too.
///
/// A drawing carries no legend to put a count in, so the piece that lost its
/// label says so itself, where a person hovering it is already looking.
pub(super) fn titled(piece: &Piece, said: bool) -> String {
    let name = escape(&piece.name);
    if said {
        return name;
    }
    format!("{name} · sin sitio para su rótulo junto a la pieza")
}

/// What the piece carries in words: beside each node the name the drafter gave
/// it, and then the block, inside the piece's own box.
///
/// The node names arrive as the name and the place together, so there is no
/// re-pairing to get wrong: the flattened cut line is the wrong length and no
/// longer the right type, and a mismatch would have truncated in silence.
///
/// They are also written first, which is the order the block was laid out
/// against: a name is a coordinate a person reads off the drawing, and the
/// block is prose that gives way to it.
pub(super) fn names(out: &mut String, named: &[(String, [f64; 2])], said: Option<&[block::Line]>) {
    for (label, at) in named {
        text(out, beside(millimetres(*at)), CAPTION, &escape(label));
    }
    for line in said.unwrap_or_default() {
        text(out, line.at, line.size, &escape(&line.body));
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
    use crate::draft::{Doc, Draft, MeasureSet, Piece, Point, Winding, block};

    /// The owner's waistband as his file declares it: cut once at the fold,
    /// with an allowance, under the letter he gave it.
    fn waistband() -> Draft {
        let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
        let corners = [[0.0, 0.0], [47.5, 0.0], [47.5, 4.0], [0.0, 4.0]];
        let points: Vec<_> = corners
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        let mut piece = Piece::polygon("PRETINA", points, Winding::Cw);
        piece.letter = Some("C".to_owned());
        piece.seam_allowance = Some(1.5);
        piece.labels = vec!["PRETINA".to_owned(), "cortar 1 al doblez c.b.".to_owned()];
        doc.pieces.insert(piece);
        Draft::from_doc(doc).expect("a rectangle resolves")
    }

    /// A person who opens the drawing instead of printing it reads the same
    /// pattern: the letter, the name, the count, the allowance and the lines
    /// their author wrote, in the words the sheet uses.
    #[test]
    fn a_piece_says_how_it_is_cut_out_in_the_drawing_too() {
        let drawing = to_svg(&waistband()).expect("the waistband draws").text;
        for said in [
            ">C · «PRETINA»</text>",
            ">Cortar 1 · margen 1.5 cm por fuera del contorno</text>",
            ">PRETINA</text>",
            ">cortar 1 al doblez c.b.</text>",
        ] {
            assert!(drawing.contains(said), "the drawing lacks {said}");
        }
    }

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
        let drawing = to_svg(&draft).expect("the block draws").text;
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
