use toile_geom::{length, validate};

use super::defect::Defect;
use super::resolve::to_metres;

/// How short a length counts as none, in centimetres.
///
/// A tenth of a micrometre: far under anything a pattern means and far over
/// the rounding of a sum of a few hundred lengths.
const EPS: f64 = 1.0e-7;

/// The whole cloth a piece drawn against a fold is cut from.
///
/// This is the one place the unfold happens. Everything that asks what the
/// piece really is — the mesher, the drape, the sheet of paper, the perimeter,
/// the fraction a seam is read onto — asks through here, so nothing downstream
/// has to know whether a piece was drawn whole or drawn half.
#[derive(Debug, Clone, PartialEq)]
pub struct Cloth {
    /// The outline in centimetres, y downward, opening at the axis's tail and
    /// running the way the drawn contour runs.
    pub cm: Vec<[f64; 2]>,
    /// The same in metres, y upward: what the mesher takes.
    pub m: Vec<[f64; 2]>,
    /// Its perimeter in centimetres.
    pub perimeter: f64,
    /// Where it opens, as flattened arc length along the drawn contour.
    pub opens_at: f64,
    /// How far the drawn contour runs from there to the axis's other end: the
    /// half of the cloth's boundary that a drawn place can sit on.
    pub walk: f64,
    /// The crease itself, in centimetres: the axis's tail, then its head.
    pub axis: [[f64; 2]; 2],
}

impl Cloth {
    /// One place reflected across the crease: where its mirror image lands on
    /// the other half.
    ///
    /// What the cloth does to the outline, offered to whatever else has to be
    /// shown on both halves — a mark traced on the drawing is a mark on the
    /// cut piece twice, once on each side of the fold.
    pub fn mirror(&self, at: [f64; 2]) -> [f64; 2] {
        let [tail, head] = self.axis;
        Crease::through(tail, head).map_or(at, |crease| crease.mirror(at))
    }

    /// The stretch of boundary that mirrors one stretch of the drawn walk, as
    /// the fraction of the perimeter it opens at and how far it runs — the pair
    /// a place's own fraction is read in.
    ///
    /// A place a given length along the walk reflects onto the place the same
    /// length back from the walk's far end, so the mirror is as long as the
    /// stretch and opens where the stretch's own far end lands.
    ///
    /// `None` for a stretch reaching past the walk, which is one measured the
    /// other way round the cloth: it covers the mirrored half already, and
    /// mirroring it would name the drawn half a second time.
    pub fn mirror_run(&self, (head, span): (f64, f64)) -> Option<(f64, f64)> {
        let reaches = (head + span) * self.perimeter;
        (self.perimeter > 0.0 && reaches <= self.walk + EPS)
            .then(|| (2.0 * self.walk / self.perimeter - head - span, span))
    }
}

/// The cloth a drawn half and its axis make, mirrored across the crease.
///
/// `flat` is the drawn contour flattened and `along` the table
/// [`length::cumulative`] builds over it; `head` and `tail` are where the two
/// ends of the axis fall along that table. The walk from the tail round to the
/// head is the part of the drawing that is cloth; the rest of the contour is
/// the crease and disappears into the middle of the piece, because a fold is
/// where the cloth turns back and not where it is cut.
///
/// # Errors
/// `Defect::FoldAxis` when the axis leaves nothing to mirror or names no line
/// to mirror across, and `Defect::Cloth` when the two halves together are not
/// a simple closed polygon — which is what an axis drawn across the middle of
/// a piece produces, and what would otherwise reach the mesher.
pub fn unfold(flat: &[[f64; 2]], along: &[f64], head: f64, tail: f64) -> Result<Cloth, Defect> {
    let total = along.last().copied().unwrap_or_default();
    if flat.len() < 2 || !total.is_finite() || total <= EPS {
        return Err(Defect::FoldAxis);
    }
    let walk = (head - tail).rem_euclid(total);
    if walk <= EPS {
        return Err(Defect::FoldAxis);
    }
    let tail = tail.rem_euclid(total);
    let drawn = run(flat, along, tail, walk);
    let (Some(&first), Some(&last)) = (drawn.first(), drawn.last()) else {
        return Err(Defect::FoldAxis);
    };
    let crease = Crease::through(first, last).ok_or(Defect::FoldAxis)?;
    let mut cm = drawn.clone();
    // The two ends sit on the crease and are their own mirror, so only what
    // lies between them is reflected: taking them too would double a point and
    // leave the outline with a side of no length at each end of the fold.
    let inner = drawn
        .get(1..drawn.len().saturating_sub(1))
        .unwrap_or_default();
    cm.extend(inner.iter().rev().map(|&at| crease.mirror(at)));
    let m: Vec<[f64; 2]> = cm.iter().map(|&at| to_metres(at)).collect();
    validate::check_closed(&m).map_err(Defect::Cloth)?;
    Ok(Cloth {
        perimeter: length::perimeter(&cm),
        cm,
        m,
        opens_at: tail,
        walk,
        axis: [first, last],
    })
}

/// The polyline of the closed `flat` from arc length `from`, for `len`.
///
/// It opens and closes on the exact places the two arc lengths name, whether or
/// not the flattening has a vertex there, and carries every vertex strictly
/// between them. A fold on a whole tract therefore keeps that tract's own
/// samples, and one that stops halfway along a curve cuts it where it was cut.
fn run(flat: &[[f64; 2]], along: &[f64], from: f64, len: f64) -> Vec<[f64; 2]> {
    let n = flat.len();
    let total = along[n];
    let from = from.rem_euclid(total);
    let mut out = vec![at(flat, along, from)];
    // The first vertex past the opening. The table rises, so walking forward
    // from there with the closure taken raises the distance from the opening
    // step by step, and the first vertex past the end is where the walk stops.
    // A vertex at or before the opening has been walked past, so its distance
    // is measured the long way round and it is never taken for a near one.
    let base = (0..n).find(|&k| along[k] > from + EPS).unwrap_or(0);
    for step in 0..n {
        let k = (base + step) % n;
        let away = if along[k] > from + EPS {
            along[k] - from
        } else {
            along[k] + total - from
        };
        if away >= len - EPS {
            break;
        }
        out.push(flat[k]);
    }
    out.push(at(flat, along, from + len));
    out
}

/// The place a given arc length along the closed `flat` falls on.
fn at(flat: &[[f64; 2]], along: &[f64], arc: f64) -> [f64; 2] {
    let n = flat.len();
    let arc = arc.rem_euclid(along[n]);
    let k = along[..n]
        .partition_point(|&reached| reached <= arc)
        .saturating_sub(1);
    let span = along[k + 1] - along[k];
    let t = if span > 0.0 {
        (arc - along[k]) / span
    } else {
        0.0
    };
    let (a, b) = (flat[k], flat[(k + 1) % n]);
    [0, 1].map(|axis| a[axis] + (b[axis] - a[axis]) * t)
}

/// The line the drawing is turned back on.
struct Crease {
    at: [f64; 2],
    dir: [f64; 2],
    span: f64,
}

impl Crease {
    /// The line through two places, or nothing when they are one place.
    fn through(a: [f64; 2], b: [f64; 2]) -> Option<Crease> {
        let dir = [b[0] - a[0], b[1] - a[1]];
        let span = dir[0] * dir[0] + dir[1] * dir[1];
        (span.is_finite() && span > EPS * EPS).then_some(Crease { at: a, dir, span })
    }

    /// One place reflected across it.
    fn mirror(&self, p: [f64; 2]) -> [f64; 2] {
        let away = [p[0] - self.at[0], p[1] - self.at[1]];
        let t = (away[0] * self.dir[0] + away[1] * self.dir[1]) / self.span;
        [0, 1].map(|axis| 2.0 * (self.at[axis] + self.dir[axis] * t) - p[axis])
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a reflection across an axis of the grid is exact in a double"
    )]

    use super::*;

    /// A hundred-centimetre square, in contour order, y downward.
    const SQUARE: [[f64; 2]; 4] = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

    /// The square's flattening and its arc-length table.
    fn square() -> (Vec<[f64; 2]>, Vec<f64>) {
        let flat = SQUARE.to_vec();
        (flat.clone(), length::cumulative(&flat))
    }

    /// Folding a square on its left side gives a rectangle twice as wide, with
    /// the crease gone from the outline: the cloth is one piece, not two.
    #[test]
    fn a_square_folded_on_its_left_side_doubles_its_width() {
        let (flat, along) = square();
        // The left side is the tract leaving node 3 and reaching node 0, so the
        // axis runs from 300 cm round to 400, and the walk is the other 300.
        let cloth = unfold(&flat, &along, 300.0, 400.0).expect("the square folds");
        assert_eq!(cloth.walk, 300.0);
        assert_eq!(
            cloth.opens_at, 0.0,
            "the closure and the start are one place"
        );
        assert_eq!(cloth.axis, [[0.0, 0.0], [0.0, 100.0]]);
        assert_eq!(
            cloth.cm,
            [
                [0.0, 0.0],
                [100.0, 0.0],
                [100.0, 100.0],
                [0.0, 100.0],
                [-100.0, 100.0],
                [-100.0, 0.0],
            ]
        );
        assert_eq!(cloth.perimeter, 600.0);
        let wide = cloth.cm.iter().map(|at| at[0]).fold(f64::MIN, f64::max)
            - cloth.cm.iter().map(|at| at[0]).fold(f64::MAX, f64::min);
        assert_eq!(wide, 200.0);
    }

    /// The axis need not stop on a node: an end halfway along a tract cuts that
    /// tract there, and the cloth opens and closes on the place the axis names
    /// rather than on the nearest corner.
    #[test]
    fn an_axis_that_stops_halfway_along_a_tract_cuts_it_there() {
        let (flat, along) = square();
        // From the middle of the bottom side round to the first corner: a
        // diagonal crease, with both remaining corners on one side of it.
        let cloth = unfold(&flat, &along, 250.0, 400.0).expect("the diagonal folds");
        assert_eq!(cloth.axis, [[0.0, 0.0], [50.0, 100.0]]);
        assert_eq!(
            cloth.cm,
            [
                [0.0, 0.0],
                [100.0, 0.0],
                [100.0, 100.0],
                [50.0, 100.0],
                [20.0, 140.0],
                [-60.0, 80.0],
            ]
        );
    }

    /// An axis whose two ends fall on one place names no line to mirror across.
    /// So does one whose two ends are two arc lengths that resolve to one
    /// place, which is the case no comparison of anchors would have caught.
    #[test]
    fn an_axis_with_nothing_to_mirror_is_a_defect_and_not_a_shape() {
        let (flat, along) = square();
        assert_eq!(unfold(&flat, &along, 400.0, 400.0), Err(Defect::FoldAxis));
        assert_eq!(unfold(&[], &[], 0.0, 0.0), Err(Defect::FoldAxis));

        let pinched = vec![[0.0, 0.0], [10.0, 0.0], [0.0, 0.0], [0.0, 10.0]];
        let table = length::cumulative(&pinched);
        assert_eq!(unfold(&pinched, &table, 20.0, 0.0), Err(Defect::FoldAxis));
    }

    /// An axis along part of a straight side leaves the rest of that side lying
    /// on the crease, so the mirror comes back over the drawing. It is a defect
    /// and not a shape, and the piece says so instead of the mesher.
    #[test]
    fn an_axis_that_leaves_part_of_its_own_side_on_the_crease_is_a_defect() {
        let (flat, along) = square();
        let error = unfold(&flat, &along, 350.0, 400.0).expect_err("half a side folds badly");
        assert!(matches!(error, Defect::Cloth(_)), "{error}");
    }

    /// An axis across the middle of a piece folds it onto itself, and that is
    /// caught here rather than in a triangulation.
    #[test]
    fn a_fold_that_lays_the_cloth_over_itself_is_not_a_piece() {
        let flat = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [50.0, 50.0]];
        let along = length::cumulative(&flat);
        let total = along[4];
        let error = unfold(&flat, &along, along[3], total).expect_err("the halves overlap");
        assert!(matches!(error, Defect::Cloth(_)), "{error}");
    }
}
