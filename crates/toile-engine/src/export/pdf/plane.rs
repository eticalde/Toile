use super::super::units::millimetres;

/// The plane the taped sheets make: one sheet of paper as big as the pile,
/// measured in millimetres from its own top left corner.
///
/// Every place of a piece reaches paper through here — the cut line, the lines
/// drawn inside it, its notches, its grain and its names — so a notch cut on
/// the cut line lands on the cut line, whichever sheet each of them is clipped
/// to.
///
/// One of these per piece, because a plane may carry more than one: the piece's
/// own corner is what its places are measured from, and where that corner sits
/// on the plane is what puts two pieces beside each other instead of on top of
/// each other.
#[derive(Debug)]
pub(super) struct Plane {
    /// Where the box around the cloth opens, in centimetres: what every place
    /// of the piece is measured from.
    low: [f64; 2],
    /// Where that box opens on the plane, in millimetres.
    place: [f64; 2],
}

impl Plane {
    /// The plane a piece whose box opens at `low` is laid on, with that box
    /// opening at `place` on the paper.
    pub(super) fn new(low: [f64; 2], place: [f64; 2]) -> Plane {
        Plane { low, place }
    }

    /// One place of the piece on the plane, in millimetres.
    pub(super) fn onto(&self, at: [f64; 2]) -> [f64; 2] {
        let [x, y] = millimetres([at[0] - self.low[0], at[1] - self.low[1]]);
        [x + self.place[0], y + self.place[1]]
    }
}
