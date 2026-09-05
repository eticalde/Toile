/// The text a finite number takes in a canonical file.
///
/// Rust's shortest round-tripping form, which is always positional: a
/// centimetre never reads as `1e2` in a diff, and reading the text back gives
/// the very bits it was written from. A negative zero is written as a zero, so
/// that two documents nobody can tell apart cannot differ in their bytes.
pub(super) fn f64_text(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    format!("{value}")
}

/// The same, for the single precision a pin is held in.
///
/// The value is formatted as the `f32` it is: widened to `f64` first, 0.1
/// would write itself as 0.10000000149011612.
pub(super) fn f32_text(value: f32) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    format!("{value}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_number_carries_no_decimal_point() {
        assert_eq!(f64_text(104.0), "104");
        assert_eq!(f64_text(-6.0), "-6");
        assert_eq!(f32_text(1.0), "1");
    }

    #[test]
    fn a_negative_zero_is_written_as_a_zero() {
        assert_eq!(f64_text(-0.0), "0");
        assert_eq!(f64_text(0.0), "0");
        assert_eq!(f32_text(-0.0), "0");
    }

    #[test]
    fn a_magnitude_no_pattern_reaches_is_still_not_a_power_of_ten() {
        assert_eq!(f64_text(1.0e-7), "0.0000001");
        assert_eq!(f64_text(1.0e21), "1000000000000000000000");
    }

    #[test]
    fn the_text_reads_back_as_the_number_it_was_written_from() {
        for value in [
            20.875,
            1.0 / 3.0,
            0.1 + 0.2,
            std::f64::consts::FRAC_PI_2,
            -6.125,
            f64::MIN_POSITIVE,
        ] {
            let text = f64_text(value);
            assert_eq!(text.parse::<f64>(), Ok(value), "{text} lost {value}");
        }
        let single = 0.1f32;
        assert_eq!(f32_text(single), "0.1");
        assert_eq!(f32_text(single).parse::<f32>(), Ok(single));
    }
}
