/// Whether any of the cloth falls on one cell of paper.
///
/// A cell the cloth misses is a sheet of blank paper, and a person who prints
/// it has to work out that it is blank on purpose. Said the other way round: a
/// cell the cloth only crosses the middle of carries no cut line at all and is
/// still printed, because a pattern has to be one unbroken sheet of paper by
/// the time it is taped up.
///
/// Three questions, because overlap has three shapes: a corner of the paper
/// under the cloth, a node of the cloth on the paper, and an edge of each
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
}
