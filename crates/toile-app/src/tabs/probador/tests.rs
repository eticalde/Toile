use eframe::egui::{Rect, pos2, vec2};
use toile_engine::body::Collider;
use toile_engine::draft::block;
use toile_engine::sync::Hanging;

use super::*;

/// Paints one pass of whatever `add` draws, over a screen-sized window.
///
/// A panel here is a painter and a set of allocations, nothing else: no GPU,
/// no window, and no state of its own to seed.
fn paint(add: impl FnMut(&mut egui::Ui, &Theme)) {
    painted(WINDOW, add);
}

/// The width the app opens at, in points: `main`'s own fallback inner size.
const WINDOW: f32 = 1320.0;

/// Every box the register can light at once, which is the widest the bar ever
/// gets.
const WIDEST: Read<'static> = Read {
    crossed: Some("14 · entrepierna"),
    hanging: Some("2 tramos · 1.1 mm del anillo"),
    worn: Some("pecho_alto · inferido cadera"),
    loose: Some("2.1× su tela · cintura"),
    adrift: Some("4 piezas sin sitio"),
};

/// The same pass over a window `wide` points across, and every line of text it
/// painted with the span that line takes.
///
/// What the frame painted and not what a layout promised, which is the only
/// honest record of what the person was shown: egui does not paint a label it
/// finds outside the clip, so a box that ran off the window went quiet about
/// its own name and said nothing at all about having done so.
fn painted(wide: f32, mut add: impl FnMut(&mut egui::Ui, &Theme)) -> Vec<(String, f32, f32)> {
    let ctx = egui::Context::default();
    let theme = Theme::sastreria();
    theme.apply(&ctx);
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(wide, 780.0));
    let input = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    let pass = ctx.run_ui(input, |ui| add(ui, &theme));
    let said = lines(&pass.shapes);
    pass.drop_without_applying_deltas();
    said
}

/// Every line of text a frame's shapes carry, groups opened up, each with the
/// abscissae it runs between.
fn lines(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, f32, f32)> {
    fn open(shape: &egui::Shape, out: &mut Vec<(String, f32, f32)>) {
        match shape {
            egui::Shape::Vec(inner) => inner.iter().for_each(|one| open(one, out)),
            egui::Shape::Text(text) => out.push((
                text.galley.job.text.clone(),
                text.pos.x,
                text.pos.x + text.galley.size().x,
            )),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in shapes {
        open(&clipped.shape, &mut out);
    }
    out
}

/// Paints the inspector over `session`.
fn panel(session: &Session) {
    paint(|ui, theme| {
        right_panel(ui, theme, |ui| inspector(ui, theme, session));
    });
}

/// The seam table draws over a table with nothing on it, and over the block.
///
/// The rows it draws come from the document, so the panel has to survive a
/// document that has no seams as readily as one that has two.
#[test]
fn the_seam_table_paints_with_and_without_a_document() {
    panel(&Session::blank(Collider::demo()));
    let session = Session::from_doc(block::trousers(), Collider::demo()).expect("the block opens");
    assert_eq!(
        seams::measured(session.draft().expect("a document")).len(),
        2
    );
    panel(&session);
}

/// The bar names the body the document resolves against, and never an empty
/// box: a product made from scratch carries a mannequin nobody has named.
#[test]
fn the_bar_names_the_body_even_when_the_document_has_not() {
    let blank = Session::blank(Collider::demo());
    assert_eq!(fitted(&blank), Some(UNNAMED));
    let session = Session::from_doc(block::trousers(), Collider::demo()).expect("the block opens");
    assert_eq!(fitted(&session), Some("Etienne"));

    paint(|ui, theme| sub_bar(ui, theme, &session, "en caché", Read::default()));
    paint(|ui, theme| sub_bar(ui, theme, &blank, "horneando…", WIDEST));
}

/// Nothing of the widest register the bar can draw falls outside the window —
/// at the size the app opens, and on the 1512 pt screen of a 14" laptop.
///
/// Read off the shapes the frame painted, which is the whole of the point:
/// every box's comment about its own width was true while the row ran 740 pt
/// past a 1288 pt bar. On one line the last four items stood off the window —
/// measured, `sin sitio` wrote its value at 1624 and its caption nowhere, and
/// all three sim controls were outside — so where each line was painted is half
/// the assertion and that every line was painted at all is the other half.
#[test]
fn the_widest_register_fits_the_window_the_app_opens() {
    let session = Session::from_doc(block::trousers(), Collider::demo()).expect("the block opens");
    let names = [
        "MANIQUÍ",
        "Etienne",
        "CUERPO",
        "horneando…",
        "CRUCES",
        "14 · entrepierna",
        "COLGADO",
        "2 tramos · 1.1 mm del anillo",
        "ESTACIÓN",
        "pecho_alto · inferido cadera",
        "ARO",
        "2.1× su tela · cintura",
        "SIN SITIO",
        "4 piezas sin sitio",
        "Reiniciar",
        "Pausar",
        "Simular",
    ];
    for wide in [WINDOW, 1512.0] {
        let said = painted(wide, |ui, theme| {
            sub_bar(ui, theme, &session, "horneando…", WIDEST);
        });
        for name in names {
            assert!(
                said.iter().any(|(text, ..)| text == name),
                "at {wide} pt the bar never painted {name:?}"
            );
        }
        for (text, lo, hi) in &said {
            assert!(
                *lo >= 0.0 && *hi <= wide,
                "at {wide} pt the bar painted {text:?} at {lo}..{hi}, outside the window"
            );
        }
    }
}

/// The bar says how near its rings the body is holding the garment, and says
/// nothing whatever about a garment hung from nothing.
///
/// The second half is the one that matters: every product anybody has drawn so
/// far hangs from nothing, and a box reading `0 tramos` or `0.0 mm` on all of
/// them would be a reading nobody took.
#[test]
fn the_bar_says_how_near_its_ring_the_garment_hangs_and_nothing_for_one_hung_from_nothing() {
    assert_eq!(note::hanging(None), None);
    let one = Hanging {
        runs: 1,
        gap: 0.000_1,
    };
    assert_eq!(
        note::hanging(Some(one)).as_deref(),
        Some("1 tramo · 0.1 mm del anillo")
    );
    // A centimetre and over reads in centimetres: a waistline a hand's breadth
    // below its ring would say `118.0 mm`, and nobody reads a garment in
    // millimetres at that size.
    let far = Hanging {
        runs: 2,
        gap: 0.118,
    };
    assert_eq!(
        note::hanging(Some(far)).as_deref(),
        Some("2 tramos · 11.8 cm del anillo")
    );
    let blank = Session::blank(Collider::demo());
    assert_eq!(blank.snapshot().hanging, None, "and nothing is hung");
    paint(|ui, theme| sub_bar(ui, theme, &blank, "en caché", Read::default()));
}

/// The bar counts the pieces no ring could place, and says nothing at all about
/// a product every one of whose pieces has a place.
///
/// The second half is the one that matters, for the reason the two boxes above
/// have one: the document the app opens is sewn into one chain and placed
/// whole, so a box lit on it would be a box lit on every launch.
#[test]
fn the_bar_counts_the_pieces_with_no_place_and_nothing_when_they_all_have_one() {
    assert_eq!(note::adrift(0), None);
    assert_eq!(note::adrift(1).as_deref(), Some("1 pieza sin sitio"));
    assert_eq!(note::adrift(4).as_deref(), Some("4 piezas sin sitio"));

    let session = Session::from_doc(block::trousers(), Collider::demo()).expect("the block opens");
    assert_eq!(session.adrift(), 0, "the block's ten pieces are all placed");
    assert_eq!(note::adrift(session.adrift()), None);
    paint(|ui, theme| sub_bar(ui, theme, &session, "en caché", Read::default()));
}

/// The bar names both rings when a release had to choose between them, and
/// nothing at all when it did not.
///
/// The second half is the one that matters, for the reason the hanging box's
/// is: every product that is drafted to the person wearing it reads the same
/// ring both ways, and a box lit on all of those would stop being read.
#[test]
fn the_bar_names_both_rings_when_the_release_had_to_choose_and_nothing_when_it_did_not() {
    use toile_engine::session::Elsewhere;

    assert_eq!(note::worn(None), None);
    let apart = Elsewhere {
        declared: "pecho_alto".to_owned(),
        inferred: "cadera".to_owned(),
    };
    assert_eq!(
        note::worn(Some(&apart)).as_deref(),
        Some("pecho_alto · inferido cadera")
    );
    // And the session the app opens on declares no station at all, so the box
    // takes no room on the bar a person actually sees.
    let session = Session::from_doc(block::trousers(), Collider::demo()).expect("the block opens");
    assert_eq!(session.worn_elsewhere(), None);
    paint(|ui, theme| sub_bar(ui, theme, &session, "en caché", Read::default()));
}

/// The bar says where a body crosses itself when a garment can reach it, and
/// says nothing of the soles the body the app opens with crosses at.
#[test]
fn the_bar_names_a_crossing_a_garment_can_reach_and_keeps_quiet_about_soles() {
    use toile_engine::body::bake::crossings;
    use toile_engine::body::{self, NO_LEVERS, Phenotype, body_mesh};

    let solved = body::solve_anny(&body::default_measures(), &Phenotype::default());
    let opening = crossings(&body_mesh(&solved.phenotype, &solved.levers));
    assert!(!opening.all().is_empty(), "its soles do cross");
    assert_eq!(note::crossed(&opening), None);

    let heavy = Phenotype {
        gender: 0.0,
        weight: 1.0,
        muscle: 1.0,
        ..Phenotype::default()
    };
    let found = crossings(&body_mesh(&heavy, &NO_LEVERS));
    assert_eq!(note::crossed(&found).as_deref(), Some("14 · entrepierna"));
}
