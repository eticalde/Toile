use toile_sim::xpbd::State;

use super::pipeline::ShapePipeline;

/// Where one piece's own width runs once the product is rolled round the body.
#[derive(Debug, Clone, Copy)]
pub struct Wrap {
    /// The turn a pattern abscissa of zero sits at, in radians.
    pub turn: f64,
    /// `1.0` when the piece's rising abscissa runs with the turn, `-1.0` when
    /// it runs against it.
    ///
    /// The two are mirror images of each other, and which one a piece takes is
    /// not a choice: it falls out of the direction its seams are walked in. A
    /// garment whose pieces all ran the same way round could not be sewn shut.
    pub sense: f64,
}

/// Where a sewn product is let go: the ring it is rolled onto, and every
/// piece's place on it.
///
/// The ring stands on the body's own vertical axis. Rolling preserves length —
/// an arc carries exactly as much cloth as it is long — so no piece begins
/// stretched or squeezed, and the rest lengths the solver holds are the flat
/// ones they were compiled from.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The ring's centre: x, then z, in metres.
    pub axis: [f32; 2],
    /// How far from that axis the cloth is rolled, in metres.
    pub radius: f64,
    /// How high the ring sits, in metres: where the crest is let go.
    ///
    /// A garment goes where on the body it belongs, so this is the height of
    /// the body's own ring it was matched to — not a height above the crown.
    /// Once a pattern is a ring, falling from above the head means nothing:
    /// there is no direction for it to fall from that is round the body.
    pub stand: f32,
    /// The highest pattern ordinate over the pieces this places.
    ///
    /// Everything hangs below it by the distance the pattern puts between
    /// them, so the pieces keep the rise they are drafted with: a back that
    /// sits two centimetres above the front on the page sits two centimetres
    /// above it on the body.
    pub crest: f64,
    /// One entry per piece, in the order the combined state holds them;
    /// `None` for a piece the seams do not place.
    pub wraps: Vec<Option<Wrap>>,
}

impl Layout {
    /// Where a pattern point of piece `at` is let go, in metres; `None` for a
    /// piece this does not place.
    pub fn point(&self, at: usize, p: [f64; 2]) -> Option<[f32; 3]> {
        let wrap = (*self.wraps.get(at)?)?;
        // `libm` rather than the intrinsic, for the reason the bake pins it:
        // what makes an angle deterministic here is that it is the same code
        // on every machine, not that it is correctly rounded.
        let (sin, cos) = libm::sincos(wrap.turn + wrap.sense * p[0] / self.radius);
        Some([
            self.axis[0] + (self.radius * sin) as f32,
            self.stand + (p[1] - self.crest) as f32,
            self.axis[1] + (self.radius * cos) as f32,
        ])
    }

    /// Writes one piece's released block into a state that holds the whole
    /// product, beginning at `base`. Answers whether it placed the piece.
    pub(super) fn wrap_into(
        &self,
        state: &mut State,
        base: usize,
        pipe: &ShapePipeline,
        at: usize,
    ) -> bool {
        if self.wraps.get(at).copied().flatten().is_none() {
            return false;
        }
        for (i, &p) in pipe.pos2d.iter().enumerate() {
            let Some([x, y, z]) = self.point(at, p) else {
                return false;
            };
            state.px[base + i] = x;
            state.py[base + i] = y;
            state.pz[base + i] = z;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipeline(w: f64, h: f64) -> ShapePipeline {
        let rectangle = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
        ShapePipeline::build(&rectangle, 16, 0.01).expect("the rectangle is finite")
    }

    fn layout(wraps: Vec<Option<Wrap>>, stand: f32) -> Layout {
        Layout {
            axis: [0.0, 0.0],
            radius: 0.25,
            stand,
            crest: 0.20,
            wraps,
        }
    }

    /// Rolling is length-preserving: two points a given distance apart across
    /// the piece come out that far apart along the arc they are rolled onto.
    #[test]
    fn an_arc_carries_exactly_as_much_cloth_as_it_is_long() {
        let place = layout(
            vec![Some(Wrap {
                turn: 0.0,
                sense: 1.0,
            })],
            1.0,
        );
        let (near, far) = (
            place.point(0, [0.0, 0.0]).expect("placed"),
            place.point(0, [0.10, 0.0]).expect("placed"),
        );
        let turn = (0.10f64 / place.radius) * 0.5;
        let chord = 2.0 * place.radius * turn.sin();
        let got = f64::from((far[0] - near[0]).hypot(far[2] - near[2]));
        assert!((got - chord).abs() < 1.0e-9, "{got} against {chord}");
    }

    /// The sense is the mirror: the same piece walked the other way lands the
    /// same points at the same distance from the axis, turned the other way.
    #[test]
    fn the_two_senses_are_mirror_images() {
        let with = layout(
            vec![Some(Wrap {
                turn: 0.0,
                sense: 1.0,
            })],
            1.0,
        );
        let against = layout(
            vec![Some(Wrap {
                turn: 0.0,
                sense: -1.0,
            })],
            1.0,
        );
        let (a, b) = (
            with.point(0, [0.10, -0.05]).expect("placed"),
            against.point(0, [0.10, -0.05]).expect("placed"),
        );
        assert!((a[0] + b[0]).abs() < 1.0e-7, "the turn flipped");
        assert!((a[2] - b[2]).abs() < 1.0e-7, "and nothing else did");
        assert!((a[1] - b[1]).abs() < 1.0e-7);
    }

    /// The crest is what the ring's height names: the product's highest point
    /// starts there, and every other point hangs below it by the distance the
    /// pattern puts between them.
    #[test]
    fn the_highest_pattern_point_is_the_one_let_go_at_the_ring() {
        let place = layout(
            vec![Some(Wrap {
                turn: 0.3,
                sense: 1.0,
            })],
            1.5,
        );
        let top = place.point(0, [0.0, 0.20]).expect("placed");
        let below = place.point(0, [0.0, -0.30]).expect("placed");
        assert!((top[1] - 1.5).abs() < 1.0e-7);
        assert!((below[1] - 1.0).abs() < 1.0e-7);
    }

    /// A piece the seams do not place is left for the flat release, and says
    /// so rather than landing somewhere arbitrary.
    #[test]
    fn a_piece_without_a_wrap_is_not_placed() {
        let pipe = pipeline(0.30, 0.20);
        let place = layout(vec![None], 1.0);
        let mut state = State::new(pipe.pos2d.len());
        assert!(!place.wrap_into(&mut state, 0, &pipe, 0));
        assert!(
            !place.wrap_into(&mut state, 0, &pipe, 7),
            "nor is one the layout has never heard of"
        );
        assert_eq!(place.point(0, [0.0, 0.0]), None);
    }
}
