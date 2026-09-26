use eframe::egui::{self, Event, Id, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, vec2};
use toile_engine::draft::BodyShape;
use toile_engine::session::Session;

use super::super::super::identity;
use super::super::use_id;
use super::{ANA, Stand, Table, drag, product};
use crate::tabs::{LEFT_W, left_panel};
use crate::theme::Theme;
use crate::widgets::PAD;

/// What the panel says of the press that replaced a body called Marta by Ana,
/// up to the clause that depends on what happened since.
const REPLACED: &str = "«Ana» sustituye a «Marta», que no se guardaba en ningún sitio. Cmd+Z \
                        devuelve a «Marta»";

/// The clause it gains once the body Ana's copy became has been written into.
const DISCARDS: &str = ", y descarta lo que hayas cambiado desde entonces.";

/// A context dressed the way the app dresses its own.
fn screen() -> egui::Context {
    let ctx = egui::Context::default();
    Theme::sastreria().apply(&ctx);
    ctx
}

/// One frame of the tab's left column — the body's own panel over the library
/// section, as the tab stacks them — fed `events`, with whatever the library
/// asked for done the way the tab does it.
///
/// Answers with every line of text the frame painted, which is the only honest
/// record of what the person was told.
fn frame(table: &mut Table, ctx: &egui::Context, events: Vec<Event>) -> Vec<String> {
    let theme = Theme::sastreria();
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1320.0, 780.0))),
        events,
        ..RawInput::default()
    };
    let mut asked = None;
    let pass = ctx.run_ui(input, |ui| {
        let (session, stand) = (&mut table.session, &mut table.stand);
        let (shelf, people) = (&table.shelf, &mut table.people);
        asked = left_panel(ui, &theme, |ui| {
            identity::panel(ui, &theme, session, stand);
            let kept = Stand::kept(session);
            super::super::panel(ui, &theme, shelf, people, kept, None)
        });
    });
    let said = said(&pass.shapes);
    pass.drop_without_applying_deltas();
    if let Some(plea) = asked {
        table.act(plea);
    }
    said
}

/// Every line of text a frame's shapes carry, groups opened up.
fn said(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    fn open(shape: &Shape, out: &mut Vec<String>) {
        match shape {
            Shape::Vec(inner) => inner.iter().for_each(|shape| open(shape, out)),
            Shape::Text(text) => out.push(text.galley.job.text.clone()),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in shapes {
        open(&clipped.shape, &mut out);
    }
    out
}

/// A few frames with nobody touching anything, so every rect is laid out, and
/// what the last of them painted.
fn settle(table: &mut Table, ctx: &egui::Context) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..3 {
        said = frame(table, ctx, Vec::new());
    }
    said
}

/// The pointer arriving on what the last frame drew under `id`, pressing and
/// letting go, a frame each.
fn click(table: &mut Table, ctx: &egui::Context, id: Id) {
    let at = ctx
        .read_response(id)
        .expect("the panel drew it")
        .rect
        .center();
    frame(table, ctx, vec![Event::PointerMoved(at)]);
    for pressed in [true, false] {
        let event = Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        frame(table, ctx, vec![event]);
    }
}

/// Marta on the stand with no product open, measured by hand, and Ana picked in
/// the library so the press that replaces her is on offer.
fn marta(test: &str) -> Table {
    let mut table = Table::with_ana(test);
    table.session = Session::demo_bodice();
    table.stand.rename(&mut table.session, "Marta");
    drag(&mut table.stand, &mut table.session, "cintura", &[71.5]);
    drag(&mut table.stand, &mut table.session, "cadera", &[96.0]);
    table.people.picked = Some(ANA.to_owned());
    table
}

/// What the stand carries, as an assertion can name it in one line.
fn body(table: &Table) -> (String, Option<f64>, Option<f64>, usize) {
    let set = table.stand.body(&table.session);
    (
        set.name.clone(),
        set.get("cintura"),
        set.get("cadera"),
        set.values.len(),
    )
}

/// Replacing the body on the stand while no product is open is said on the
/// panel, and the step back it names gives the body a person typed back whole.
#[test]
fn replacing_the_body_on_the_stand_is_said_and_one_step_gives_it_back() {
    let mut table = marta("replace-said");
    let ctx = screen();
    let typed = body(&table);
    assert_eq!(typed, ("Marta".to_owned(), Some(71.5), Some(96.0), 20));
    let before = settle(&mut table, &ctx);
    assert!(!before.iter().any(|line| line.contains("sustituye")));

    click(&mut table, &ctx, use_id());
    assert_eq!(body(&table), ("Ana".to_owned(), Some(72.0), Some(98.0), 10));
    let after = settle(&mut table, &ctx);
    assert!(after.contains(&format!("{REPLACED}.")), "{after:#?}");

    // Nothing steps forward from here: the way forward is the button that was
    // pressed, and a redo that did nothing must not spend the way back.
    table.stand.step(&mut table.session, true);
    assert_eq!(body(&table), ("Ana".to_owned(), Some(72.0), Some(98.0), 10));

    table.stand.step(&mut table.session, false);
    assert_eq!(body(&table), typed, "Marta comes back whole");
    let back = settle(&mut table, &ctx);
    assert!(
        !back.iter().any(|line| line.contains("sustituye")),
        "{back:#?}"
    );
    table.stand.step(&mut table.session, false);
    assert_eq!(body(&table), typed, "and there is only ever one step");
}

/// The button says which of the two acts it does: with no product it replaces
/// the body on the stand, and with one it adds to the product.
#[test]
fn the_button_names_the_act_and_does_not_call_a_replacement_a_use() {
    let mut table = marta("replace-label");
    let ctx = screen();
    let loose = settle(&mut table, &ctx);
    let button = ctx.read_response(use_id()).expect("the panel drew it").rect;
    assert!(
        button.max.x < LEFT_W - PAD,
        "the label has to fit the panel: {button:?}"
    );
    assert!(
        loose.contains(&"Sustituir el de la mesa".to_owned()),
        "{loose:#?}"
    );
    assert!(
        !loose.iter().any(|line| line.contains("Usar")),
        "{loose:#?}"
    );

    let (session, stand) = product();
    table.session = session;
    table.stand = stand;
    let open = settle(&mut table, &ctx);
    assert!(
        open.contains(&"Usar en el producto".to_owned()),
        "{open:#?}"
    );
}

/// The same press with a product open is what it always was: the person joins
/// the product, the body that was there stays, and one entry holds both.
#[test]
fn the_same_press_with_a_product_open_adds_a_body_and_keeps_the_old_one() {
    let mut table = Table::with_ana("replace-product");
    let (session, stand) = product();
    table.session = session;
    table.stand = stand;
    let first = table.session.draft().expect("a product").doc().resolve_with;

    table.use_ana();
    let doc = table.session.draft().expect("a product").doc();
    assert_eq!(doc.mannequins.len(), 3, "the old bodies stay");
    assert!(doc.mannequin_named("Etienne").is_some());
    assert_eq!(table.stand.body(&table.session).name, "Ana");
    assert_eq!(table.session.undo_label(), Some("usar persona"));
    assert_eq!(
        table.stand.replaced(),
        None,
        "a product holds its own history"
    );

    table.session.undo().expect("the copy undoes");
    let doc = table.session.draft().expect("a product").doc();
    assert_eq!(doc.resolve_with, first);
    assert_eq!(doc.mannequin_named("Ana"), None);
    assert!(
        !table.session.can_undo(),
        "the body and the choice were one"
    );
}

/// Each of the three ways the tab writes into the loose body says so on the
/// step back, which would throw that writing away in its turn.
#[test]
fn writing_into_the_body_that_took_its_place_is_said_before_the_step_back() {
    for each in 0..3 {
        let mut table = marta(&format!("replace-touched-{each}"));
        let ctx = screen();
        settle(&mut table, &ctx);
        click(&mut table, &ctx, use_id());
        let said = settle(&mut table, &ctx);
        assert!(said.contains(&format!("{REPLACED}.")), "{each}: {said:#?}");

        match each {
            0 => drag(&mut table.stand, &mut table.session, "cintura", &[61.0]),
            1 => table.stand.set_shape(
                &mut table.session,
                BodyShape {
                    build: 0.75,
                    ..BodyShape::default()
                },
            ),
            _ => table.stand.rename(&mut table.session, "Ana bis"),
        }
        let said = settle(&mut table, &ctx);
        let warned = said.iter().any(|line| line.ends_with(DISCARDS));
        assert!(warned, "{each}: {said:#?}");
        // A rename moves the name the sentence has to read now, so only the
        // measure can be matched whole; what every one of them owes is the
        // clause, and that is what the loop checks.
        if each == 0 {
            assert!(said.contains(&format!("{REPLACED}{DISCARDS}")), "{said:#?}");
        }
        table.stand.step(&mut table.session, false);
        assert_eq!(table.stand.body(&table.session).name, "Marta", "{each}");
    }
}

/// A product taking the table takes the step back with it: from then on the key
/// steps the product's own history, and a sentence offering anything else would
/// be a sentence the key does not obey.
#[test]
fn a_product_taking_the_table_takes_the_step_back_with_it() {
    let mut table = marta("replace-forget");
    let ctx = screen();
    settle(&mut table, &ctx);
    click(&mut table, &ctx, use_id());
    assert!(table.stand.replaced().is_some());

    table.stand.forget();
    assert_eq!(table.stand.replaced(), None);
    let said = settle(&mut table, &ctx);
    assert!(
        !said.iter().any(|line| line.contains("sustituye")),
        "{said:#?}"
    );
}
