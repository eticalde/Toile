use toile_engine::couture::ShapePipeline;

/// One piece's meshed boundary, as the closed polyline it is.
///
/// This suite's own reading of an outline, and deliberately not the engine's.
/// What the readings built on it say is how wide the cloth really is at a
/// height and how high the piece reaches, which are the two things the
/// placement is being held to: taken through the code under test they could
/// only ever agree with themselves.
pub struct Outline {
    ring: Vec<[f64; 2]>,
    span: (f64, f64),
}

impl Outline {
    /// Reads one meshed piece.
    pub fn of(pipe: &ShapePipeline) -> Outline {
        let ring: Vec<[f64; 2]> = pipe
            .boundary_run((0.0, 1.0))
            .iter()
            .map(|&v| pipe.pos2d[v as usize])
            .collect();
        let span = pipe.pos2d.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
            (lo.min(p[1]), hi.max(p[1]))
        });
        Outline { ring, span }
    }

    /// The lowest and highest ordinate the piece reaches.
    pub fn span(&self) -> (f64, f64) {
        self.span
    }

    /// The abscissae the piece runs between at `y`; `None` where it is not
    /// there.
    pub fn scan(&self, y: f64) -> Option<(f64, f64)> {
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for k in 0..self.ring.len() {
            let a = self.ring[k];
            let b = self.ring[(k + 1) % self.ring.len()];
            let (from, to) = if a[1] <= b[1] { (a, b) } else { (b, a) };
            if y < from[1] || y > to[1] || to[1] <= from[1] {
                continue;
            }
            let x = from[0] + (to[0] - from[0]) * (y - from[1]) / (to[1] - from[1]);
            lo = lo.min(x);
            hi = hi.max(x);
        }
        (lo <= hi).then_some((lo, hi))
    }

    /// How much cloth the piece carries at `y`, in metres; zero where it is not
    /// there.
    pub fn width(&self, y: f64) -> f64 {
        self.scan(y).map_or(0.0, |(lo, hi)| hi - lo)
    }

    /// Every ordinate the outline turns a corner at.
    pub fn corners(&self) -> impl Iterator<Item = f64> + '_ {
        self.ring.iter().map(|p| p[1])
    }
}
