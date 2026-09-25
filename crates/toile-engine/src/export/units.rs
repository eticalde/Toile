/// Millimetres in a centimetre. The document counts in the second and a sheet
/// of paper in the first, and this is the only place the two meet.
pub(super) const MM_PER_CM: f64 = 10.0;

/// Blank paper left around the pattern, in millimetres.
pub(super) const MARGIN: f64 = 10.0;

/// The weight of a cut line, in millimetres.
pub(super) const CUT: f64 = 0.3;

/// The weight of every line on a sheet except the one it is cut on, in
/// millimetres.
///
/// Thinner than the cut line, so that a sheet says at a glance what to cut and
/// what only to mark. One weight for all of them and not one per kind of mark:
/// what a cutter has to tell apart is the cut line from everything else, and a
/// third weight would only make the second one mean less.
pub(super) const DRAWN: f64 = CUT * 2.0 / 3.0;

/// The dashes a line drawn broken is drawn with, in millimetres: the ink, then
/// the gap.
///
/// Which lines are broken is the pattern's own decision and lives with the
/// kinds. This is only how long the dashes are, and it is the same length on
/// both sheets because a fold broken one way on a screen and another way on
/// paper is the same line arriving in two hands.
pub(super) const BROKEN: [f64; 2] = [4.0, 2.0];

/// The colour a sheet is drawn in, as a drawing states it.
///
/// A pattern is printed, so it is black on white and never the studio's theme.
/// `theme::Theme` rules the screen, where a person picks the light they work in
/// and can pick again; paper is bought in one colour and a printer is not
/// asked, and a pale line is one a cutter cannot follow. The printed sheet sets
/// no colour at all, which is this same decision in a format whose own default
/// is already this one.
pub(super) const INK: &str = "#000000";

/// The type size a piece's own name is written at, in millimetres.
pub(super) const TITLE: f64 = 5.0;

/// The type size everything else on the sheet is written at, in millimetres.
pub(super) const CAPTION: f64 = 3.0;

/// A place on the sheet, from the place the document holds.
pub(super) fn millimetres([x, y]: [f64; 2]) -> [f64; 2] {
    [x * MM_PER_CM, y * MM_PER_CM]
}

/// The way back, for the one question the cloth answers in its own units.
pub(super) fn centimetres([x, y]: [f64; 2]) -> [f64; 2] {
    [x / MM_PER_CM, y / MM_PER_CM]
}

/// Where a name goes beside the place it names, in millimetres.
///
/// Half a caption up and half across: clear of the line the place sits on, and
/// near enough that a sheet carrying forty names says which name belongs to
/// which node. Both sheets ask here, or one pattern would name its nodes in two
/// places.
pub(super) fn beside([x, y]: [f64; 2]) -> [f64; 2] {
    [x + CAPTION / 2.0, y - CAPTION / 2.0]
}

/// A length as a sheet writes it: two decimals, and never a negative zero, so
/// the same pattern always writes the same bytes.
///
/// It is told the number and not the unit on purpose. The drawing counts in
/// millimetres and the printed page in points, and a sheet on which the two
/// rounded differently would be two drawings.
pub(super) fn number(value: f64) -> String {
    let text = format!("{value:.2}");
    if let Some(digits) = text.strip_prefix('-')
        && digits.bytes().all(|byte| byte == b'0' || byte == b'.')
    {
        return digits.to_owned();
    }
    text
}

/// The box around a run of places, in millimetres.
///
/// An empty run answers with an inverted box, which is what lets the sheet
/// fold several of these together and still see that nothing was drawn. It
/// takes the places one at a time rather than as a slice so a caller holding
/// keys beside them need not strip the pairing apart to ask.
pub(super) fn box_of(places: impl IntoIterator<Item = [f64; 2]>) -> ([f64; 2], [f64; 2]) {
    let mut low = [f64::INFINITY; 2];
    let mut high = [f64::NEG_INFINITY; 2];
    for at in places {
        for axis in 0..2 {
            low[axis] = low[axis].min(at[axis]);
            high[axis] = high[axis].max(at[axis]);
        }
    }
    (low, high)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_negative_zero_is_written_without_its_sign() {
        assert_eq!(number(-0.0), "0.00");
        assert_eq!(number(-0.001), "0.00");
        assert_eq!(number(-0.02), "-0.02");
    }

    #[test]
    fn an_empty_run_of_places_has_no_box() {
        let (low, high) = box_of(std::iter::empty());
        assert!(low[0] > high[0], "{low:?} {high:?}");
    }
}
