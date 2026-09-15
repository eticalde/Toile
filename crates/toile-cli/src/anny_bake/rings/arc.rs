use super::geom::dist;
use super::intersect::{self, Crossing};
use super::select::ring_point;

/// The sum of an open run of crossings' chords, on the template.
pub(super) fn length(positions: &[[f64; 3]], run: &[Crossing]) -> f64 {
    run.windows(2)
        .map(|pair| {
            dist(
                ring_point(positions, pair[0]),
                ring_point(positions, pair[1]),
            )
        })
        .sum()
}

/// Which way round a closed loop a walk goes: toward higher indices or lower.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Along {
    Forward,
    Backward,
}

impl Along {
    pub fn reversed(self) -> Self {
        match self {
            Along::Forward => Along::Backward,
            Along::Backward => Along::Forward,
        }
    }
}

/// One closed loop a plane leaves on the template, every crossing evaluated
/// once, so an open path can be read off it by walking round.
pub(super) struct Section {
    crossings: Vec<Crossing>,
    points: Vec<[f64; 3]>,
}

impl Section {
    pub fn new(positions: &[[f64; 3]], crossings: Vec<Crossing>) -> Self {
        let points = crossings
            .iter()
            .map(|&c| ring_point(positions, c))
            .collect();
        Self { crossings, points }
    }

    /// Every loop the plane through `point` with `normal` leaves on the mesh.
    pub fn all(
        positions: &[[f64; 3]],
        tris: &[[u32; 3]],
        point: [f64; 3],
        normal: [f64; 3],
    ) -> Vec<Self> {
        intersect::loops(positions, tris, point, normal)
            .into_iter()
            .map(|l| Self::new(positions, l))
            .collect()
    }

    /// Of the loops that plane leaves, the one that passes nearest `anchor`.
    ///
    /// # Panics
    /// If the plane misses the mesh: every caller puts a body point on it.
    pub fn through(
        positions: &[[f64; 3]],
        tris: &[[u32; 3]],
        point: [f64; 3],
        normal: [f64; 3],
        anchor: [f64; 3],
    ) -> Self {
        let mut best: Option<(Self, f64)> = None;
        for section in Self::all(positions, tris, point, normal) {
            let d = dist(section.at(section.nearest(anchor)), anchor);
            if best.as_ref().is_none_or(|(_, best_d)| d < *best_d) {
                best = Some((section, d));
            }
        }
        best.expect("a plane through a body point crosses the body")
            .0
    }

    pub fn at(&self, i: usize) -> [f64; 3] {
        self.points[i]
    }

    /// The index of the crossing nearest `target`, the lowest index on a tie.
    pub fn nearest(&self, target: [f64; 3]) -> usize {
        let mut best = (0, f64::INFINITY);
        for (i, &p) in self.points.iter().enumerate() {
            let d = dist(p, target);
            if d < best.1 {
                best = (i, d);
            }
        }
        best.0
    }

    /// The index one step from `i`, wrapping round the loop.
    pub fn next(&self, i: usize, way: Along) -> usize {
        let n = self.points.len();
        match way {
            Along::Forward => (i + 1) % n,
            Along::Backward => (i + n - 1) % n,
        }
    }

    /// The way from `i` whose first step lands lower (smaller `y`) than the
    /// other way's.
    pub fn downward(&self, i: usize) -> Along {
        let (ahead, behind) = (self.next(i, Along::Forward), self.next(i, Along::Backward));
        if self.points[ahead][1] < self.points[behind][1] {
            Along::Forward
        } else {
            Along::Backward
        }
    }

    /// The first index after `from`, walking `way`, whose point `done`
    /// accepts, handed the point it stepped from and the point it reached.
    ///
    /// # Panics
    /// If a whole turn round the loop finds none: the caller's stop condition
    /// does not hold anywhere on this section.
    pub fn walk_until(
        &self,
        from: usize,
        way: Along,
        done: impl Fn([f64; 3], [f64; 3]) -> bool,
    ) -> usize {
        let mut i = from;
        for _ in 0..self.points.len() {
            let previous = i;
            i = self.next(i, way);
            if done(self.points[previous], self.points[i]) {
                return i;
            }
        }
        panic!("a whole turn round the section from crossing {from} never stopped")
    }

    /// Walking `way` from `from`, the crossing nearest height `y`: the first
    /// at or below it, or the one before that when it stands nearer.
    pub fn down_to(&self, from: usize, way: Along, y: f64) -> usize {
        let below = self.walk_until(from, way, |_, p| p[1] <= y);
        let above = self.next(below, way.reversed());
        if y - self.points[below][1] <= self.points[above][1] - y {
            below
        } else {
            above
        }
    }

    /// The crossings from `from` to `to`, both included, walking `way`.
    pub fn arc(&self, from: usize, to: usize, way: Along) -> Vec<Crossing> {
        let mut i = from;
        let mut out = vec![self.crossings[i]];
        while i != to {
            i = self.next(i, way);
            out.push(self.crossings[i]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A loop of five crossings, each sitting on its own vertex: a tall
    /// pentagon standing on the `y = 0` line, so heights read off easily.
    fn pentagon() -> Section {
        let positions = vec![
            [0.0, 4.0, 0.0],
            [1.0, 3.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 3.0, 0.0],
        ];
        Section::new(&positions, (0..5).map(|i| (i, i, 0.0)).collect())
    }

    #[test]
    fn stepping_wraps_round_the_loop_both_ways() {
        let s = pentagon();
        assert_eq!(s.next(4, Along::Forward), 0);
        assert_eq!(s.next(0, Along::Backward), 4);
    }

    #[test]
    fn downward_takes_the_lower_neighbour() {
        let s = pentagon();
        assert_eq!(s.downward(1), Along::Forward);
        assert_eq!(s.downward(4), Along::Backward);
    }

    #[test]
    fn down_to_keeps_whichever_of_the_bracketing_crossings_is_nearer() {
        let s = pentagon();
        assert_eq!(s.down_to(0, Along::Forward, 2.5), 1);
        assert_eq!(s.down_to(0, Along::Forward, 0.5), 2);
    }

    #[test]
    fn an_arc_keeps_both_ends_and_wraps() {
        let s = pentagon();
        let arc = s.arc(3, 1, Along::Forward);
        assert_eq!(arc.iter().map(|c| c.0).collect::<Vec<_>>(), [3, 4, 0, 1]);
    }

    #[test]
    #[should_panic(expected = "never stopped")]
    fn a_walk_that_finds_nothing_says_so() {
        pentagon().walk_until(0, Along::Forward, |_, p| p[1] > 10.0);
    }
}
