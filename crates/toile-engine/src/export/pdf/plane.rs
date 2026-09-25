use super::super::units::millimetres;

/// The plane the taped sheets make: one sheet of paper as big as the piece,
/// measured in millimetres from its own top left corner.
///
/// Every place of the piece reaches paper through here — the cut line, the
/// lines drawn inside it, its notches, its grain and its names — so a notch cut
/// on the cut line lands on the cut line, whichever sheet each of them is
/// clipped to.
#[derive(Debug)]
pub(super) struct Plane {
    /// Where the box around the cloth opens, in centimetres: what every place
    /// of the piece is measured from.
    low: [f64; 2],
    /// The paper the piece does not fill, in millimetres.
    spare: f64,
}

impl Plane {
    /// The plane a piece whose box opens at `low` is laid on, with `spare`
    /// millimetres of paper it does not fill.
    pub(super) fn new(low: [f64; 2], spare: f64) -> Plane {
        Plane { low, spare }
    }

    /// One place of the piece on the plane, in millimetres.
    ///
    /// The spare paper is shared out across the columns and kept off the rows,
    /// because a draft traces downward from the waist: a piece hangs from the
    /// top margin of its first row, where its own first line is.
    pub(super) fn onto(&self, at: [f64; 2]) -> [f64; 2] {
        let [x, y] = millimetres([at[0] - self.low[0], at[1] - self.low[1]]);
        [x + self.spare / 2.0, y]
    }
}
