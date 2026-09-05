#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::formula::Formula;

/// The source a drag would write, at the tenth of a centimetre the app snaps
/// to when no grid is on.
fn nudged(src: &str, delta: f64) -> String {
    Formula::parse(src)
        .expect("the source parses")
        .nudged_source(delta, 0.1)
}

#[test]
fn the_user_spacing_survives() {
    assert_eq!(nudged("cintura /4  +   1", 0.6), "cintura /4  +   1.6");
    assert_eq!(nudged("cintura/4+1", 0.6), "cintura/4+1.6");
}

#[test]
fn a_group_absorbs_inside_itself() {
    assert_eq!(nudged("(cintura / 4 + 1)", 0.6), "(cintura / 4 + 1.6)");
    assert_eq!(nudged("((cadera))", 0.6), "((cadera + 0.6))");
}

#[test]
fn a_conditional_absorbs_on_both_arms() {
    assert_eq!(
        nudged("cadera < 90 ? 20 : cintura / 4", 1.0),
        "cadera < 90 ? 21 : cintura / 4 + 1"
    );
    assert_eq!(
        nudged("(cadera < 90 ? 20 : 22)", 1.0),
        "(cadera < 90 ? 21 : 23)"
    );
    assert_eq!(
        nudged("a < b ? c < d ? 1 : 2 : 3", 1.0),
        "a < b ? c < d ? 2 : 3 : 4"
    );
}

#[test]
fn nudging_back_gives_the_source_it_started_from() {
    for src in [
        "cintura / 4 + 1",
        "cintura /4  +   1",
        "22",
        "(cadera < 90 ? 20 : 22)",
    ] {
        let there = nudged(src, 0.6);
        assert_eq!(nudged(&there, -0.6), src, "{src} does not come back");
    }
}

#[test]
fn a_term_that_was_appended_comes_back_to_zero() {
    // The term stays: dropping it again would rewrite bytes its author wrote,
    // and `+ 0` is the honest record of a drag that went nowhere.
    assert_eq!(nudged(&nudged("cadera / 4", 0.6), -0.6), "cadera / 4 + 0");
}
