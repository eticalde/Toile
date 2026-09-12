use toile_engine::session::Session;

use super::*;

/// The cells of one tab's bar, run together the way the bar paints them.
fn said(tab: Tab, session: &Session, body: Body<'_>) -> String {
    tab.status(session, &patronaje::State::default(), body)
        .into_iter()
        .map(|(cell, _)| cell)
        .collect::<Vec<_>>()
        .join(" · ")
}

/// A body nobody has renamed, with the twenty rows the tab lists.
fn body() -> Body<'static> {
    Body {
        name: "Maniquí",
        measures: 20,
    }
}

/// A table with nothing draping has no sim thread, and the bar may not report
/// one — nor name a person, a product or a fabric it is not holding.
///
/// The snapshot such a table answers with is the empty one, whose `converged`
/// is false: the same value a simulation still working reports.
#[test]
fn a_table_with_no_sim_does_not_say_the_sim_is_running() {
    let session = Session::blank();
    let said = said(Tab::Probador, &session, body());

    assert_eq!(said, "sin simulación");
    for invented in ["Etienne", "Pantalón base", "Algodón popelina", "substeps"] {
        assert!(!said.contains(invented), "{invented} in {said}");
    }
}

/// The mannequin bar names the body the tab holds and counts its rows.
#[test]
fn the_maniquies_bar_names_the_body_the_tab_holds() {
    let session = Session::blank();
    assert_eq!(
        said(Tab::Maniquies, &session, body()),
        "Maniquí · 20 medidas · cm"
    );

    let renamed = Body {
        name: "Talla 38",
        measures: 21,
    };
    assert_eq!(
        said(Tab::Maniquies, &session, renamed),
        "Talla 38 · 21 medidas · cm",
        "a rename reaches the bar, and so does a row added or removed"
    );
}

/// The fabric bar reports nothing, because the tab holds nothing to report.
#[test]
fn the_telas_bar_names_no_fabric_the_app_does_not_have() {
    let session = Session::blank();
    assert_eq!(said(Tab::Telas, &session, body()), "");
}
