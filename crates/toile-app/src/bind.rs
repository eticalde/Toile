use toile_engine::draft::Binding;

/// Hundredths of a centimetre: the finest a coordinate is ever written to.
const HUNDREDTHS: f64 = 100.0;

/// What a gesture makes of a coordinate it took `delta` centimetres, to `at`,
/// working at a resolution of `step` centimetres.
///
/// A coordinate written as a formula keeps its formula: the delta is absorbed
/// into the adjustment term, so the node stays parametric all through the
/// gesture instead of only after it. A plain number stays a plain number. The
/// resolved value is never written back over the binding it came from, which
/// is why every surface that drags a node comes through here.
pub fn placed(origin: &Binding, at: f64, delta: f64, step: f64) -> Binding {
    match origin {
        Binding::Formula(formula) => formula
            .nudge(delta, step)
            .map_or_else(|_| origin.clone(), Binding::Formula),
        Binding::Literal(_) => Binding::literal(quantized(at, step)),
    }
}

/// A place written at the resolution the gesture is working in.
///
/// A coordinate carrying the whole of a pointer's arithmetic reads as noise in
/// the inspector and diffs as noise in the file, and neither the cloth nor the
/// eye can tell the last of those decimals apart.
fn quantized(value: f64, step: f64) -> f64 {
    let stepped = if step > 0.0 && step.is_finite() {
        (value / step).round() * step
    } else {
        value
    };
    (stepped * HUNDREDTHS).round() / HUNDREDTHS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_number_is_written_at_the_resolution_of_the_gesture() {
        let origin = Binding::literal(22.0);
        assert_eq!(placed(&origin, 25.437, 3.437, 0.5), Binding::literal(25.5));
        assert_eq!(placed(&origin, 25.437, 3.437, 0.0), Binding::literal(25.44));
    }

    #[test]
    fn a_formula_keeps_its_formula_and_absorbs_the_delta() {
        let origin = Binding::parse("cintura / 4 + 1").expect("the source parses");
        let moved = placed(&origin, 23.6, 1.6, 0.1);
        assert_eq!(moved.source(), "cintura / 4 + 2.6");
    }

    #[test]
    fn a_formula_that_cannot_be_nudged_is_left_alone() {
        let origin = Binding::parse("cintura / 4").expect("the source parses");
        let moved = placed(&origin, 22.0, 0.0, 0.1);
        assert_eq!(moved.source(), "cintura / 4");
    }
}
