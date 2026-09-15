use super::super::tests::product;

/// The tape is laid again when the body is solved again or another row is in
/// hand, and never on a frame that changed neither.
#[test]
fn the_tape_is_laid_only_when_the_body_or_the_row_changes() {
    let (session, mut stand) = product();
    let mesh = stand.rebuild(&session);
    assert!(stand.tape_due(), "a body just solved has no tape on it yet");
    assert_eq!(stand.lay(&mesh), None, "no row in hand, no tape");
    assert!(!stand.tape_due(), "a frame that changed nothing");

    stand.highlight = Some("cintura".to_owned());
    assert!(stand.tape_due(), "another row in hand");
    let waist = stand.lay(&mesh).expect("the waist has a tape");
    assert!(waist.closed, "a girth is a loop");
    assert!(!stand.tape_due(), "the same row, still in hand");

    let mesh = stand.rebuild(&session);
    assert!(stand.tape_due(), "the body moved under the tape");
    assert_eq!(
        stand.lay(&mesh),
        Some(waist),
        "the same body, the same tape"
    );
}
