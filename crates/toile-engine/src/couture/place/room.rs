/// Room the body asked for, ordinate band by ordinate band.
///
/// A table of radius to add, sampled at knots one band apart and read straight
/// between them. Straight and not stepped, because a step in the radius is a
/// fold the cloth never had: the cloth would be asked to jump a band's worth of
/// circumference between two rows of vertices a millimetre apart.
#[derive(Debug, Clone)]
pub(super) struct Room {
    /// The ordinate the first knot sits at, and the gap between knots.
    from: f64,
    step: f64,
    /// Radius added at each knot, in metres.
    ease: Vec<f64>,
    /// Knots the body is still swallowing cloth at.
    asked: Vec<bool>,
}

impl Room {
    /// A table over `span` with no room asked for anywhere yet.
    pub(super) fn over(span: (f64, f64), step: f64) -> Room {
        let knots = (((span.1 - span.0) / step).ceil().max(1.0) as usize) + 1;
        Room {
            from: span.0,
            step,
            ease: vec![0.0; knots],
            asked: vec![false; knots],
        }
    }

    /// Where an ordinate sits in the table, as a knot index with a fraction.
    fn along(&self, y: f64) -> f64 {
        ((y - self.from) / self.step).clamp(0.0, (self.ease.len() - 1) as f64)
    }

    /// The room at one ordinate, in metres of radius.
    pub(super) fn at(&self, y: f64) -> f64 {
        let along = self.along(y);
        let knot = along as usize;
        let next = (knot + 1).min(self.ease.len() - 1);
        self.ease[knot] + (self.ease[next] - self.ease[knot]) * (along - knot as f64)
    }

    /// Marks the two knots an ordinate is read between.
    ///
    /// Both, because the reading there is a blend of the two: opening only the
    /// nearer one would move the radius at that ordinate by less than the step
    /// it was opened by, and the walk would circle.
    pub(super) fn ask(&mut self, y: f64) {
        let knot = self.along(y) as usize;
        let next = (knot + 1).min(self.asked.len() - 1);
        self.asked[knot] = true;
        self.asked[next] = true;
    }

    /// Opens every marked knot that is not already `ceiling` out, clears the
    /// marks, and answers whether anything moved.
    pub(super) fn open(&mut self, by: f64, ceiling: f64) -> bool {
        let mut moved = false;
        for knot in 0..self.ease.len() {
            if std::mem::replace(&mut self.asked[knot], false) && self.ease[knot] < ceiling {
                self.ease[knot] += by;
                moved = true;
            }
        }
        if !moved {
            return false;
        }
        // No knot ends more than one step from its neighbours, so the surface
        // never leans further than a band of height per band of radius. Once
        // down and once up is the whole of it: the upward pass can only raise a
        // knot to within a step of the one below, which leaves it within a step
        // of the one above too.
        for knot in (0..self.ease.len() - 1).rev() {
            self.ease[knot] = self.ease[knot].max(self.ease[knot + 1] - by);
        }
        for knot in 1..self.ease.len() {
            self.ease[knot] = self.ease[knot].max(self.ease[knot - 1] - by);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table over half a metre in five-millimetre bands.
    fn room() -> Room {
        Room::over((-0.50, 0.0), 0.005)
    }

    /// A knot opens by one step, and the ordinate between two knots opens by
    /// one step too: the reading there is a blend of the pair, so both are
    /// marked.
    #[test]
    fn asking_between_two_knots_opens_both_of_them() {
        let mut table = room();
        table.ask(-0.2525);
        assert!(table.open(0.005, 1.0));
        assert!(
            (table.at(-0.2525) - 0.005).abs() < 1.0e-12,
            "{}",
            table.at(-0.2525)
        );
        assert!(
            (table.at(-0.250) - 0.005).abs() < 1.0e-12,
            "the knot above it"
        );
        assert!(
            (table.at(-0.255) - 0.005).abs() < 1.0e-12,
            "and the one below"
        );
    }

    /// No knot ends more than one step from its neighbours, however many rounds
    /// one band is opened for: the profile has no cliff for the cloth to fold
    /// at.
    #[test]
    fn the_table_never_leans_further_than_a_step_a_band() {
        let mut table = room();
        for _ in 0..20 {
            table.ask(-0.25);
            table.open(0.005, 1.0);
        }
        assert!(
            (table.at(-0.25) - 0.100).abs() < 1.0e-12,
            "{}",
            table.at(-0.25)
        );
        for k in 1..table.ease.len() {
            let step = (table.ease[k] - table.ease[k - 1]).abs();
            assert!(step <= 0.005 + 1.0e-12, "knot {k} leans {step}");
        }
        // And the lean is all it took to get there: twenty bands out and twenty
        // steps down from the band that asked, the table is back to nothing.
        assert!(table.at(-0.35) < 1.0e-12, "{}", table.at(-0.35));
    }

    /// A ceiling stops it: a knot already that far out is not opened again, and
    /// a round that opens nothing says so.
    #[test]
    fn a_knot_at_the_ceiling_stops_the_round() {
        let mut table = room();
        let mut rounds = 0;
        loop {
            table.ask(-0.25);
            if !table.open(0.005, 0.02) {
                break;
            }
            rounds += 1;
            assert!(rounds < 100, "it never stopped");
        }
        assert_eq!(rounds, 4);
        assert!(
            (table.at(-0.25) - 0.020).abs() < 1.0e-12,
            "{}",
            table.at(-0.25)
        );
    }
}
