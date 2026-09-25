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
    fn xml_metacharacters_in_a_name_are_escaped() {
        assert_eq!(escape("A & <B>"), "A &amp; &lt;B&gt;");
    }
}
