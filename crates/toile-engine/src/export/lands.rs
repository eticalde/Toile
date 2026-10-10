/// Whether any of the cloth falls on one box of the plane.
///
/// Asked of a cell of paper, where a cell the cloth misses is a sheet of blank
/// paper and a person who prints it has to work out that it is blank on
/// purpose — a cell the cloth only crosses the middle of carries no cut line
/// at all and is still printed, because a pattern has to be one unbroken sheet
/// of paper by the time it is taped up. And asked of the box around a line of
/// words, where the same answer says whether those words landed on a piece of
/// cloth that is not theirs.
///
/// Three questions, because overlap has three shapes: a corner of the box
/// under the cloth, a node of the cloth in the box, and an edge of each
/// crossing the other with neither end inside.
pub(super) fn on(plane: &[[f64; 2]], corner: [f64; 2], cell: [f64; 2]) -> bool {
    let far = [corner[0] + cell[0], corner[1] + cell[1]];
    let corners = [corner, [far[0], corner[1]], far, [corner[0], far[1]]];
    if corners.iter().any(|&at| inside(plane, at)) {
        return true;
    }
    for (rank, &from) in plane.iter().enumerate() {
        if within(from, corner, far) {
            return true;
        }
        let to = plane[(rank + 1) % plane.len()];
        if !overlaps(from, to, corner, far) {
            continue;
        }
        for edge in 0..4 {
            if crosses(from, to, corners[edge], corners[(edge + 1) % 4]) {
                return true;
            }
        }
    }
    false
}

/// How much of one box of the plane the cloth covers, as a share of the box.
///
/// What [`on`] answers yes or no to, answered by area, because the question a
/// block asks is not whether it touches its own cloth but whether it is mostly
/// on it: a line of words whose ink ends up on the discard side of the cut line
/// is a line that goes in the bin with the trimming, and the sheet still spent
/// the room on it.
///
/// The outline is clipped to the box one edge at a time and what survives is
/// measured, so a concave piece answers for the hollow it leaves: the clip is
/// the box and the box is convex, which is the one shape this walk is exact
/// for. Several separate scraps of cloth inside one box come back joined by
/// edges that run both ways and cancel, so their areas still add up.
pub(super) fn share(plane: &[[f64; 2]], corner: [f64; 2], cell: [f64; 2]) -> f64 {
    let whole = cell[0] * cell[1];
    if whole <= 0.0 || plane.len() < 3 {
        return 0.0;
    }
    let far = [corner[0] + cell[0], corner[1] + cell[1]];
    let mut held = plane.to_vec();
    for (axis, edge, low) in [
        (0, corner[0], true),
        (0, far[0], false),
        (1, corner[1], true),
        (1, far[1], false),
    ] {
        if held.len() < 3 {
            return 0.0;
        }
        held = cut(&held, axis, edge, low);
    }
    (area(&held) / whole).clamp(0.0, 1.0)
}

/// What is left of a ring on one side of one line of the plane.
fn cut(ring: &[[f64; 2]], axis: usize, edge: f64, low: bool) -> Vec<[f64; 2]> {
    let inside = |at: [f64; 2]| {
        if low {
            at[axis] >= edge
        } else {
            at[axis] <= edge
        }
    };
    let mut out = Vec::with_capacity(ring.len() + 4);
    for (rank, &from) in ring.iter().enumerate() {
        let to = ring[(rank + 1) % ring.len()];
        if inside(from) {
            out.push(from);
        }
        if inside(from) != inside(to) {
            let part = (edge - from[axis]) / (to[axis] - from[axis]);
            out.push([
                from[0] + part * (to[0] - from[0]),
                from[1] + part * (to[1] - from[1]),
            ]);
        }
    }
    out
}

/// The area a closed ring encloses, whichever way round it runs.
fn area(ring: &[[f64; 2]]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut twice = 0.0;
    for (rank, &from) in ring.iter().enumerate() {
        let to = ring[(rank + 1) % ring.len()];
        twice += from[0] * to[1] - to[0] * from[1];
    }
    twice.abs() / 2.0
}

/// Whether a place is on a cell of paper.
fn within(at: [f64; 2], low: [f64; 2], high: [f64; 2]) -> bool {
    (0..2).all(|axis| at[axis] >= low[axis] && at[axis] <= high[axis])
}

/// Whether the box around one tract of the outline reaches a cell at all.
///
/// Asked before the four crossing tests, and not only to save them: two runs
/// that lie along one line answer that they cross wherever they are, and this
/// is what keeps that answer away from a cell they are nowhere near.
fn overlaps(from: [f64; 2], to: [f64; 2], low: [f64; 2], high: [f64; 2]) -> bool {
    (0..2)
        .all(|axis| from[axis].min(to[axis]) <= high[axis] && from[axis].max(to[axis]) >= low[axis])
}

/// Whether two runs meet, counting a touch as a meeting.
///
/// The doubt falls towards printing the sheet: an extra blank one costs a
/// person a sheet of paper, and a missing one is a hole in the middle of a
/// pattern, found at the cutting table.
fn crosses(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let side = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    side(a, b, c) * side(a, b, d) <= 0.0 && side(c, d, a) * side(c, d, b) <= 0.0
}

/// Whether a place is inside the outline, by the parity of the crossings left
/// of it.
fn inside(plane: &[[f64; 2]], at: [f64; 2]) -> bool {
    let mut inside = false;
    for (rank, &from) in plane.iter().enumerate() {
        let to = plane[(rank + 1) % plane.len()];
        if (from[1] > at[1]) != (to[1] > at[1]) {
            let part = (at[1] - from[1]) / (to[1] - from[1]);
            if at[0] < from[0] + part * (to[0] - from[0]) {
                inside = !inside;
            }
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square of cloth ten by ten, at the plane's own corner.
    const SQUARE: [[f64; 2]; 4] = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];

    #[test]
    fn a_cell_the_cloth_is_nowhere_near_takes_no_paper() {
        assert!(!on(&SQUARE, [20.0, 20.0], [5.0, 5.0]));
        assert!(!on(&SQUARE, [0.0, 11.0], [5.0, 5.0]));
    }

    /// A cell wholly inside the cloth carries no line at all, and is still
    /// paper the pattern is made of.
    #[test]
    fn a_cell_with_cloth_on_all_sides_of_it_takes_paper() {
        assert!(on(&SQUARE, [3.0, 3.0], [4.0, 4.0]));
    }

    #[test]
    fn a_cell_the_cloth_crosses_without_a_node_on_it_takes_paper() {
        // A band across the middle, wider than the cloth both ways.
        assert!(on(&SQUARE, [-5.0, 4.0], [20.0, 2.0]));
    }

    /// The corner a concave piece leaves empty: the cloth's box covers the
    /// cell, and the cloth does not.
    #[test]
    fn the_hollow_of_an_l_leaves_its_cell_blank() {
        let ell = [
            [0.0, 0.0],
            [4.0, 0.0],
            [4.0, 6.0],
            [10.0, 6.0],
            [10.0, 10.0],
            [0.0, 10.0],
        ];
        assert!(!on(&ell, [5.0, 1.0], [4.0, 4.0]));
        assert!(on(&ell, [1.0, 1.0], [2.0, 2.0]));
    }

    /// A cell the cloth only touches is printed, because the other answer is a
    /// hole in the pattern when the touch was really a crossing.
    #[test]
    fn a_cell_the_cloth_only_touches_takes_paper() {
        assert!(on(&SQUARE, [10.0, 0.0], [5.0, 5.0]));
    }

    /// The share a box of words has of its own cloth, which is what decides
    /// where a block may sit: all of it, none of it, or the quarter a corner
    /// overlaps.
    #[test]
    fn a_box_knows_how_much_of_it_the_cloth_covers() {
        assert!((share(&SQUARE, [2.0, 2.0], [4.0, 4.0]) - 1.0).abs() < 1e-9);
        assert!(share(&SQUARE, [20.0, 20.0], [4.0, 4.0]).abs() < 1e-9);
        assert!((share(&SQUARE, [8.0, 8.0], [4.0, 4.0]) - 0.25).abs() < 1e-9);
        // A box the cloth only touches along one edge is paper, not cloth.
        assert!(share(&SQUARE, [10.0, 0.0], [5.0, 5.0]).abs() < 1e-9);
    }

    /// And the hollow of a concave piece is counted as the paper it is: a
    /// bounding box would answer that the whole of this one is cloth.
    #[test]
    fn the_hollow_of_an_l_is_not_counted_as_its_cloth() {
        let ell = [
            [0.0, 0.0],
            [4.0, 0.0],
            [4.0, 6.0],
            [10.0, 6.0],
            [10.0, 10.0],
            [0.0, 10.0],
        ];
        assert!(share(&ell, [5.0, 1.0], [4.0, 4.0]).abs() < 1e-9);
        assert!((share(&ell, [0.0, 0.0], [10.0, 10.0]) - 0.64).abs() < 1e-9);
    }
}
