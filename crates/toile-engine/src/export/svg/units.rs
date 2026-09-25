/// Millimetres in a centimetre. The document counts in the second and a sheet
/// of paper in the first, and this is the only place the two meet.
pub(super) const MM_PER_CM: f64 = 10.0;

/// A place on the sheet, from the place the document holds.
pub(super) fn millimetres([x, y]: [f64; 2]) -> [f64; 2] {
    [x * MM_PER_CM, y * MM_PER_CM]
}

/// The way back, for the one question the cloth answers in its own units.
pub(super) fn centimetres([x, y]: [f64; 2]) -> [f64; 2] {
    [x / MM_PER_CM, y / MM_PER_CM]
}

/// A millimetre as the drawing writes it: two decimals, and never a negative
/// zero, so the same pattern always writes the same bytes.
pub(super) fn mm(value: f64) -> String {
    let text = format!("{value:.2}");
    if let Some(digits) = text.strip_prefix('-')
        && digits.bytes().all(|byte| byte == b'0' || byte == b'.')
    {
        return digits.to_owned();
    }
    text
}

/// Text as XML takes it.
pub(super) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_negative_zero_is_written_without_its_sign() {
        assert_eq!(mm(-0.0), "0.00");
        assert_eq!(mm(-0.001), "0.00");
        assert_eq!(mm(-0.02), "-0.02");
    }

    #[test]
    fn xml_metacharacters_in_a_name_are_escaped() {
        assert_eq!(escape("A & <B>"), "A &amp; &lt;B&gt;");
    }
}
