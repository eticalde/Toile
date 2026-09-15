use eframe::egui::{Rect, pos2};
use toile_engine::draft::{Draft, PieceKey};

use super::pick;

/// The room between two pieces the overview lines up itself, in centimetres.
///
/// Wide enough that two outlines side by side never read as one piece, narrow
/// enough that a product of a few pieces still frames at a scale where their
/// names can be read.
pub const GAP_CM: f64 = 10.0;

/// One piece as the overview draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct Laid {
    /// The piece.
    pub piece: PieceKey,
    /// How far the overview moves it from its own coordinates, in centimetres.
    pub shift: [f64; 2],
    /// Its flattened outline, moved that far.
    pub outline: Vec<[f64; 2]>,
}

/// Every piece of the product where the overview draws it, in key order.
///
/// A piece somebody placed is drawn at its placement. One nobody placed takes
/// its slot in a row: in key order each piece is given a slot as wide as its
/// own outline, followed by a gap, with every top on the page's zero. A placed
/// piece keeps its slot all the same, so taking one piece out of the row never
/// slides the ones after it under the pointer.
///
/// The row is worked out afresh on every frame and written nowhere: a product
/// nobody arranged keeps its bytes however many times it is looked at.
pub fn of(draft: &Draft) -> Vec<Laid> {
    let mut across = 0.0;
    let mut laid = Vec::new();
    for (piece, held) in draft.doc().pieces.iter() {
        let own = draft.flat_cm(piece);
        let (slot, width) = match extent(own) {
            Some([left, top, right, _]) => ([across - left, -top], right - left),
            None => ([across, 0.0], 0.0),
        };
        across += width + GAP_CM;
        let shift = held.placement.map_or(slot, |at| [at.x, at.y]);
        let outline = own
            .iter()
            .map(|&[x, y]| [x + shift[0], y + shift[1]])
            .collect();
        laid.push(Laid {
            piece,
            shift,
            outline,
        });
    }
    laid
}

/// The box every piece on the overview occupies, in centimetres; nothing when
/// no piece has an outline to occupy one with.
pub fn bounds(laid: &[Laid]) -> Option<Rect> {
    let every: Vec<[f64; 2]> = laid
        .iter()
        .flat_map(|it| it.outline.iter().copied())
        .collect();
    let [left, top, right, bottom] = extent(&every)?;
    Some(Rect::from_min_max(
        pos2(left as f32, top as f32),
        pos2(right as f32, bottom as f32),
    ))
}

/// The piece under a place on the overview, in centimetres.
///
/// Inside its outline, or within `reach` of the line. Pieces are drawn in key
/// order, so where two overlap the one found is the one drawn on top.
pub fn under(at: [f64; 2], laid: &[Laid], reach: f64) -> Option<&Laid> {
    laid.iter().rev().find(|it| {
        it.outline.len() >= 3 && (inside(&it.outline, at) || near(&it.outline, at, reach))
    })
}

/// The left, top, right and bottom of an outline.
fn extent(outline: &[[f64; 2]]) -> Option<[f64; 4]> {
    let (&[x, y], rest) = outline.split_first()?;
    Some(rest.iter().fold([x, y, x, y], |[l, t, r, b], &[x, y]| {
        [l.min(x), t.min(y), r.max(x), b.max(y)]
    }))
}

/// Whether a place falls inside a closed outline, by the even-odd rule.
fn inside(outline: &[[f64; 2]], at: [f64; 2]) -> bool {
    let mut odd = false;
    for (index, &[ax, ay]) in outline.iter().enumerate() {
        let [bx, by] = outline[(index + 1) % outline.len()];
        // The first test keeps the division off a level side: it only passes
        // for a side whose two ends lie on either side of the place.
        if (ay > at[1]) != (by > at[1]) && at[0] < ax + (at[1] - ay) * (bx - ax) / (by - ay) {
            odd = !odd;
        }
    }
    odd
}

/// Whether a place lies within `reach` of the outline itself.
fn near(outline: &[[f64; 2]], at: [f64; 2], reach: f64) -> bool {
    (0..outline.len()).any(|index| {
        let (a, b) = (outline[index], outline[(index + 1) % outline.len()]);
        pick::nearest_on(a, b, at, index).away < reach
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "every slot is a sum of whole centimetres, exact in a double"
    )]

    use toile_engine::draft::{Doc, MeasureSet, Piece, Placement, Point, Winding};

    use super::*;

    /// Rectangles as their top left corner and their size, one piece each, in
    /// key order, drawn nowhere near where a row would put them.
    const RECTS: [([f64; 2], [f64; 2]); 3] = [
        ([5.0, 7.0], [10.0, 20.0]),
        ([-30.0, 40.0], [4.0, 6.0]),
        ([0.0, 0.0], [8.0, 8.0]),
    ];

    /// The rectangles as a product, the first of them placed at `first`.
    fn product(first: Option<Placement>) -> Draft {
        let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
        for (index, ([x, y], [w, h])) in RECTS.into_iter().enumerate() {
            let corners = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
            let points: Vec<_> = corners
                .into_iter()
                .map(|[x, y]| doc.points.insert(Point::at(x, y)))
                .collect();
            let name = format!("Pieza {index}");
            let key = doc
                .pieces
                .insert(Piece::polygon(&name, points, Winding::Cw));
            if index == 0 {
                doc.pieces.get_mut(key).expect("just inserted").placement = first;
            }
        }
        Draft::from_doc(doc).expect("rectangles resolve")
    }

    fn box_of(laid: &Laid) -> [f64; 4] {
        extent(&laid.outline).expect("a rectangle has an outline")
    }

    #[test]
    fn pieces_nobody_placed_stand_in_a_row_in_key_order_a_gap_apart() {
        let laid = of(&product(None));
        let boxes: Vec<[f64; 4]> = laid.iter().map(box_of).collect();
        assert_eq!(boxes[0], [0.0, 0.0, 10.0, 20.0]);
        assert_eq!(boxes[1], [20.0, 0.0, 24.0, 6.0]);
        assert_eq!(boxes[2], [34.0, 0.0, 42.0, 8.0]);
        assert_eq!(
            laid[1].shift,
            [50.0, -40.0],
            "the shift is what takes the piece's own corner to its slot"
        );
    }

    #[test]
    fn a_placed_piece_sits_where_it_was_put_and_the_row_does_not_close_up() {
        let row = of(&product(None));
        let moved = of(&product(Some(Placement::new(100.0, -12.5))));
        assert_eq!(moved[0].shift, [100.0, -12.5]);
        assert_eq!(box_of(&moved[0]), [105.0, -5.5, 115.0, 14.5]);
        assert_eq!(moved[1..], row[1..], "the pieces after it keep their slots");
    }

    #[test]
    fn the_piece_found_under_the_pointer_is_the_one_drawn_on_top() {
        let square = |index: u32, x: f64| Laid {
            piece: PieceKey::new(index, 0),
            shift: [x, 0.0],
            outline: vec![[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 10.0], [x, 10.0]],
        };
        let laid = [square(0, 0.0), square(1, 5.0)];
        let found = |at: [f64; 2]| under(at, &laid, 0.5).map(|it| it.piece.index());
        assert_eq!(found([2.0, 5.0]), Some(0));
        assert_eq!(found([7.0, 5.0]), Some(1), "the later piece is on top");
        assert_eq!(found([15.3, 5.0]), Some(1), "just off the line is on it");
        assert_eq!(found([16.0, 5.0]), None);
        assert_eq!(found([7.0, 30.0]), None);
    }

    #[test]
    fn nothing_laid_out_has_no_bounds() {
        assert_eq!(bounds(&[]), None);
    }
}
