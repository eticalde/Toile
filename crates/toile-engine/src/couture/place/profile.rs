use super::super::pipeline::ShapePipeline;

/// One edge of a piece's outline, lower end first.
#[derive(Debug, Clone, Copy)]
struct Chord {
    from: [f64; 2],
    to: [f64; 2],
}

impl Chord {
    /// Where this edge stands at ordinate `y`; `None` when it is not there.
    ///
    /// A level edge answers nothing, and nothing is lost by that. Its own two
    /// ends are the ends of the edges either side of it, and those climb, so a
    /// straight waistline or hem is already in the reading by the time the run
    /// of level edges is reached — whichever end of it the ordinate asked for.
    fn at(&self, y: f64) -> Option<f64> {
        let (lo, hi) = (self.from[1], self.to[1]);
        if y < lo || y > hi || hi <= lo {
            return None;
        }
        Some(self.from[0] + (self.to[0] - self.from[0]) * (y - lo) / (hi - lo))
    }
}

/// How wide one piece's cloth runs at any pattern ordinate.
///
/// An exact scanline of the piece's own outline, and not of the box around it:
/// what a garment carries round the body at a given height is the cloth that
/// is actually there at that height, which on a skirt half climbs from the
/// waistline to the hip and then stops climbing.
///
/// Indexed by ordinate, because the clearance walk reads this once per vertex
/// per round: a reading costs a binary search and the two or three edges that
/// cross where it asks, not the whole outline.
#[derive(Debug, Clone)]
pub(super) struct Profile {
    /// Every ordinate an outline vertex sits at, ascending and without
    /// repeats. There are `rungs.len() - 1` slabs between them.
    rungs: Vec<f64>,
    /// The edges reaching into each slab, flattened: slab `k` is
    /// `chords[starts[k]..starts[k + 1]]`.
    chords: Vec<Chord>,
    starts: Vec<usize>,
}

impl Profile {
    /// Reads one meshed piece's outline; `None` for a piece too degenerate to
    /// have a width anywhere.
    ///
    /// The mesh's own boundary and not the document's contour: these are the
    /// vertices the solver holds, so a width read here is a width the cloth
    /// really has, curves already flattened and refinement points included.
    pub(super) fn of(pipe: &ShapePipeline) -> Option<Profile> {
        let ring = pipe.boundary_run((0.0, 1.0));
        if ring.len() < 3 {
            return None;
        }
        let mut rungs: Vec<f64> = ring.iter().map(|&v| pipe.pos2d[v as usize][1]).collect();
        rungs.sort_by(f64::total_cmp);
        rungs.dedup();
        if rungs.len() < 2 {
            return None;
        }
        let outline: Vec<Chord> = (0..ring.len())
            .map(|k| {
                let a = pipe.pos2d[ring[k] as usize];
                let b = pipe.pos2d[ring[(k + 1) % ring.len()] as usize];
                if a[1] <= b[1] {
                    Chord { from: a, to: b }
                } else {
                    Chord { from: b, to: a }
                }
            })
            .collect();
        let mut chords = Vec::new();
        let mut starts = Vec::with_capacity(rungs.len());
        for slab in rungs.windows(2) {
            starts.push(chords.len());
            let reaches = |c: &&Chord| c.from[1] <= slab[1] && c.to[1] >= slab[0];
            chords.extend(outline.iter().filter(reaches).copied());
        }
        starts.push(chords.len());
        Some(Profile {
            rungs,
            chords,
            starts,
        })
    }

    /// The lowest and highest ordinate the piece reaches.
    pub(super) fn span(&self) -> (f64, f64) {
        (self.rungs[0], self.rungs[self.rungs.len() - 1])
    }

    /// The abscissa the piece runs between at one ordinate.
    ///
    /// An ordinate outside the piece is clamped into it rather than refused,
    /// because a placement that answered `None` there would drop the whole
    /// piece back to the flat release over a rounding error.
    ///
    /// The clamp is not a rounding allowance, and reading it as one is how the
    /// strip came to be opened at a width that is not there: asked about the
    /// 25 mm the trouser back rises above its front, this hands back the
    /// front's own top edge, unchanged, as though the cloth ran on. Which of
    /// the two readings a surface owes there is decided once, by the caller
    /// that holds the strip's whole range; [`Profile::carries`] is the other
    /// one, and it is the one that says no cloth.
    pub(super) fn extent(&self, y: f64) -> (f64, f64) {
        let top = self.rungs.len() - 1;
        let y = y.clamp(self.rungs[0], self.rungs[top]);
        // The slab whose lower rung the ordinate sits on or above, and the last
        // slab for the top rung itself, which no slab has as its lower end.
        let slab = (self.rungs.partition_point(|&r| r <= y).max(1) - 1).min(top - 1);
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for chord in &self.chords[self.starts[slab]..self.starts[slab + 1]] {
            if let Some(x) = chord.at(y) {
                lo = lo.min(x);
                hi = hi.max(x);
            }
        }
        // A closed outline always crosses the slab an ordinate falls in, so
        // this is unreachable; it reads as no cloth rather than panicking,
        // because a placement is not worth a crash.
        if lo <= hi { (lo, hi) } else { (0.0, 0.0) }
    }

    /// How much cloth the piece carries at one ordinate, in metres.
    pub(super) fn width(&self, y: f64) -> f64 {
        let (lo, hi) = self.extent(y);
        hi - lo
    }

    /// How much cloth the piece really has at one ordinate, in metres: zero
    /// where it does not reach that high or that low.
    ///
    /// What a garment carries, as against what the surface it is rolled on
    /// measures. The two differ wherever the strip is not whole, and telling
    /// them apart is the difference between reading a hoop and reading the
    /// cloth on it.
    pub(super) fn carries(&self, y: f64) -> f64 {
        let (low, high) = self.span();
        if y < low || y > high {
            return 0.0;
        }
        self.width(y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A piece meshed from a contour given in metres, wound the way the mesher
    /// takes one.
    fn meshed(contour: &[[f64; 2]]) -> ShapePipeline {
        ShapePipeline::build(contour, 64, 1.0e-3).expect("the contour is finite")
    }

    /// A skirt half: 0.20 m of waistline at the top, 0.50 m of hip a third of a
    /// metre below it.
    fn wedge() -> ShapePipeline {
        meshed(&[[0.0, -0.30], [0.50, -0.30], [0.20, 0.0], [0.0, 0.0]])
    }

    /// A rectangle is one width from top to bottom.
    ///
    /// Read away from the corners, because the mesh's boundary is the contour
    /// sampled at even arc lengths and a corner that falls between two samples
    /// is cut off. That is the cloth the solver actually holds, so it is the
    /// cloth this reads — the piece comes out a hair narrower at its very ends
    /// and exactly its own width everywhere else.
    #[test]
    fn a_rectangle_reads_its_own_width_between_its_corners() {
        let pipe = meshed(&[[0.0, 0.0], [0.40, 0.0], [0.40, 0.30], [0.0, 0.30]]);
        let profile = Profile::of(&pipe).expect("the rectangle has a width");
        for k in 2..=8 {
            let y = 0.30 * f64::from(k) / 10.0;
            let (lo, hi) = profile.extent(y);
            assert!(
                lo.abs() < 1.0e-9 && (hi - 0.40).abs() < 1.0e-9,
                "at {y} it read {lo}..{hi}"
            );
        }
        let (lo, hi) = profile.span();
        assert!(
            lo >= 0.0 && hi <= 0.30,
            "and never past the piece: {lo}..{hi}"
        );
        assert!(
            profile.width(hi) < 0.40,
            "the top corner is cut, so the piece is narrower there: {}",
            profile.width(hi)
        );
    }

    /// The reading follows a slanted edge, ordinate by ordinate: this is the
    /// whole of what a cylinder could not do, since the two differ only where
    /// the cloth's own width changes with height.
    #[test]
    fn a_wedge_widens_with_the_cloth_and_not_with_its_box() {
        let pipe = wedge();
        let profile = Profile::of(&pipe).expect("the wedge has a width");
        // The slant is straight, so a scanline across it reads the cloth
        // exactly wherever the corners are not in the way.
        for (y, want) in [(-0.05, 0.25), (-0.15, 0.35), (-0.25, 0.45)] {
            assert!(
                (profile.width(y) - want).abs() < 1.0e-9,
                "at {y} it read {} and not {want}",
                profile.width(y)
            );
        }
    }

    /// A piece that stops widening stops widening, and the edge that took it
    /// there is not read past its own end.
    ///
    /// The skirt half every fixture here is cut from: it flares from the
    /// waistline to the hip and runs straight from the hip to the hem. Read an
    /// edge a hair beyond where it ends and the hip goes on widening all the
    /// way down, which is a garment that gets bigger the further it is from
    /// the body that asked for it.
    #[test]
    fn a_piece_that_stops_widening_is_read_as_stopping() {
        let pipe = meshed(&[
            [0.0, -0.50],
            [0.30, -0.50],
            [0.30, -0.20],
            [0.20, 0.0],
            [0.0, 0.0],
        ]);
        let profile = Profile::of(&pipe).expect("the half has a width");
        for y in [-0.21, -0.25, -0.30, -0.45] {
            assert!(
                (profile.width(y) - 0.30).abs() < 1.0e-9,
                "at {y} it read {} and not the hip's own 0.30",
                profile.width(y)
            );
        }
        assert!(
            (profile.width(-0.10) - 0.25).abs() < 1.0e-9,
            "and halfway up the flare it read {}",
            profile.width(-0.10)
        );
    }

    /// An ordinate past either end is the piece's own end, so a vertex the
    /// arithmetic nudged a nanometre out of range still places.
    #[test]
    fn an_ordinate_outside_the_piece_reads_its_nearest_edge() {
        let pipe = wedge();
        let profile = Profile::of(&pipe).expect("the wedge has a width");
        assert_eq!(profile.extent(1.0), profile.extent(0.0));
        assert_eq!(profile.extent(-2.0), profile.extent(-0.30));
    }

    /// And carries nothing there, which the extent cannot say.
    ///
    /// The clamp answers with the nearest edge so that a vertex a nanometre out
    /// of range still places; how much cloth a height carries is the other
    /// question, and out there the answer is none.
    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "out past the piece the reading is a literal zero, not a sum"
    )]
    fn a_height_the_piece_does_not_reach_carries_no_cloth() {
        let pipe = wedge();
        let profile = Profile::of(&pipe).expect("the wedge has a width");
        let (lo, hi) = profile.span();
        assert_eq!(profile.carries(hi + 0.001), 0.0, "above its own top");
        assert_eq!(profile.carries(lo - 0.001), 0.0, "and below its hem");
        assert!(
            profile.width(hi + 0.001) > 0.10,
            "where the extent runs the top edge on: {}",
            profile.width(hi + 0.001)
        );
        assert!(
            (profile.carries(-0.15) - profile.width(-0.15)).abs() < 1.0e-12,
            "and inside the piece the two readings are one"
        );
    }
}
