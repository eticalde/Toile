use eframe::egui::{self, Id};
use toile_engine::draft::{
    Command, Doc, EdgeRange, Hang, HangKey, Identity, MeasureSet, PieceKey, PointKey,
};

use super::super::wire::Verb;
use super::elastic::{covers, press};
use crate::tabs::catalogue;
use crate::theme::Theme;
use crate::widgets::{PAD, cycle_named, field_row, footer_note, plain_note, readout, section};

const NONE: &str = "Colgar el tramo lo lleva a la altura de un anillo del cuerpo —la cintura, la \
                    cadera, la muñeca— y ahí lo sostiene. El elástico aprieta el tramo; esto dice \
                    de qué línea del cuerpo cuelga la prenda.";
const HELD: &str = "La prenda cuelga de esa línea y no del roce. La caja pasa al anillo siguiente, \
                    y el contorno es lo que la cinta de este cuerpo mide ahí.";
const NO_BODY: &str = "Este producto no resuelve contra ningún cuerpo, y de lo que se cuelga un \
                       tramo es de un anillo del cuerpo.";
const SPLIT: &str = "Los dos extremos de este tramo están en piezas distintas. Un colgado sostiene \
                     un tramo de una sola pieza.";
const NO_RING: &str = "El cuerpo no mide ningún contorno con ese nombre, así que no hay anillo del \
                       que colgar el tramo. Un largo baja por el cuerpo y no nombra ninguna altura.";

const PUT_ON: &str = "colgar el tramo del cuerpo";
const TAKE_OFF: &str = "descolgar el tramo";
const STATION: &str = "cambiar el anillo del que cuelga";

/// How wide the box that steps the ring is drawn.
const BOX_W: f32 = 150.0;

/// What holds the chosen tract up, and the way to change it.
///
/// Nothing is written by looking: a tract nobody hung offers the press that
/// hangs it and writes nothing until it, and a tract already hung draws what
/// the document holds. Opening a product and reading this section leaves its
/// bytes and its version exactly as they were.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    doc: &Doc,
    at: (PieceKey, PointKey, PointKey),
    verbs: &mut Vec<Verb>,
) {
    let (piece, from, to) = at;
    let run = EdgeRange::between(piece, from, to);
    section(ui, theme, "Colgado");
    match on(doc, run) {
        Some((key, station)) => hung(ui, theme, doc, (key, station), verbs),
        None => offer(ui, theme, doc, (from, run), verbs),
    }
}

/// The hang on one stretch of contour, when the document holds one.
///
/// Node to node and nothing else, by the elastic's rule and with its own
/// reader: the panel writes exactly the stretch that is chosen, so a row it
/// draws is a row it can write back. Two hangs over one stretch are allowed, as
/// two elastics are, and the first the arena holds is the one this section
/// edits.
fn on(doc: &Doc, at: EdgeRange) -> Option<(HangKey, &str)> {
    doc.hangs
        .iter()
        .find(|(_, held)| covers(held.at, at))
        .map(|(key, held)| (key, held.station.as_str()))
}

/// What a tract nobody hung shows: what hanging it does, and the press that
/// does it.
///
/// It goes on at the waist, which is the ring a waistband hangs from, and the
/// box steps from there to any other. What would let the press choose better is
/// which part of the body the tract belongs to, and no pattern in this tree
/// says; a ring guessed from the run's own girth alone would hang a trouser leg
/// round a head.
fn offer(
    ui: &mut egui::Ui,
    theme: &Theme,
    doc: &Doc,
    (node, run): (PointKey, EdgeRange),
    verbs: &mut Vec<Verb>,
) {
    if let Some(why) = refused(doc, run) {
        plain_note(ui, theme, why);
        return;
    }
    let pressed = press(ui, theme, put_on_id(node), "Colgar del cuerpo");
    footer_note(ui, theme, NONE);
    if pressed {
        verbs.extend(super::super::entry(
            PUT_ON,
            Command::AddHang {
                identity: Identity::New,
                hang: Hang::new(run, Hang::WAIST),
            },
        ));
    }
}

/// The ring the tract already hangs from, what the body measures round it, and
/// the press that lets the tract down.
fn hung(
    ui: &mut egui::Ui,
    theme: &Theme,
    doc: &Doc,
    (key, station): (HangKey, &str),
    verbs: &mut Vec<Verb>,
) {
    ring(ui, theme, (key, station), verbs);
    girth(ui, theme, doc, station);
    if press(ui, theme, take_off_id(key), "Descolgar") {
        verbs.extend(super::super::entry(
            TAKE_OFF,
            Command::RemoveHang { hang: key },
        ));
    }
    footer_note(ui, theme, HELD);
}

/// The ring, as a box that steps to the next one the body carries.
///
/// A step and not a menu, because this app opens none. A station no body has a
/// ring for cannot be stepped anywhere, so the box is drawn as the readout it
/// really is and the reason is said under it.
fn ring(ui: &mut egui::Ui, theme: &Theme, (key, station): (HangKey, &str), verbs: &mut Vec<Verb>) {
    let shown = catalogue::label(station);
    let Some(to) = next(station) else {
        stuck(ui, theme, shown);
        return;
    };
    ui.add_space(6.0);
    let pressed = ui
        .horizontal(|ui| {
            ui.add_space(PAD);
            cycle_named(ui, theme, station_id(key), "anillo", shown, BOX_W).clicked()
        })
        .inner;
    ui.add_space(4.0);
    if pressed {
        verbs.extend(super::super::entry(
            STATION,
            Command::SetHangStation {
                hang: key,
                to: to.to_owned(),
            },
        ));
    }
}

/// The ring of a station no body carries one for: the name as the document
/// holds it, nothing to press, and the reason under it.
fn stuck(ui: &mut egui::Ui, theme: &Theme, shown: &str) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        readout(ui, theme, "anillo", shown, BOX_W);
    });
    ui.add_space(4.0);
    plain_note(ui, theme, NO_RING);
}

/// What the body measures round that ring, when its tape says.
///
/// The tape and not the mesh: this panel reads the document, and what the
/// document knows about a ring is the centimetres somebody wrote down for it. A
/// girth the tape does not carry reads as a dash, as a row of this panel does
/// wherever the document has no number to put there.
fn girth(ui: &mut egui::Ui, theme: &Theme, doc: &Doc, station: &str) {
    let Some(set) = doc.measures() else {
        plain_note(ui, theme, NO_BODY);
        return;
    };
    match set.get(station) {
        Some(cm) => field_row(ui, theme, "contorno", &format!("{cm:.1}"), "cm"),
        None => field_row(ui, theme, "contorno", "—", ""),
    }
}

/// Why the chosen tract cannot be hung at all, in words a person reads.
///
/// The document's own refusals, asked before there is a press rather than after
/// one is turned down: a refused edit reaches the status bar as the engine's
/// own English, and what somebody looking at their drawing needs there is a
/// sentence about the drawing.
fn refused(doc: &Doc, run: EdgeRange) -> Option<&'static str> {
    if run.piece().is_none() {
        return Some(SPLIT);
    }
    doc.measures().is_none().then_some(NO_BODY)
}

/// The ring a press steps to; `None` for a station no body has a ring for.
///
/// The catalogue's girths are the stations the document takes, so the ring this
/// walks is that array and there is no second list to keep in step: a girth
/// added to the catalogue is a station the box reaches the day it is added.
fn next(station: &str) -> Option<&'static str> {
    let at = MeasureSet::GIRTHS
        .iter()
        .position(|&ring| ring == station)?;
    let along = (at + 1) % MeasureSet::GIRTHS.len();
    MeasureSet::GIRTHS.get(along).copied()
}

/// The identity of the press that hangs the tract leaving `node`.
pub(in crate::tabs::patronaje) fn put_on_id(node: PointKey) -> Id {
    Id::new(("patronaje-hang-on", node))
}

/// The identity of the box that steps the ring a hang holds to.
pub(in crate::tabs::patronaje) fn station_id(at: HangKey) -> Id {
    Id::new(("patronaje-hang-station", at))
}

/// The identity of the press that lets the tract down.
pub(in crate::tabs::patronaje) fn take_off_id(at: HangKey) -> Id {
    Id::new(("patronaje-hang-off", at))
}

#[cfg(test)]
mod tests {
    use toile_engine::draft::{DocError, EdgeAnchor, MannequinKey, block};

    use super::*;

    /// The waist of the shipped block, which is a run of one piece.
    fn waist(doc: &Doc) -> EdgeRange {
        let piece = doc.piece_named(block::FRONT).expect("the block draws one");
        let named = |label: &str| {
            doc.shows_label(piece, label)
                .unwrap_or_else(|| panic!("the block names {label}"))
        };
        EdgeRange::between(piece, named("cintura_cf"), named("cintura_lat"))
    }

    /// Stepping the ring reaches every station the document takes, and only
    /// those.
    ///
    /// A ring that missed a girth would leave a station nothing in the app can
    /// ask for; one that offered a length would ask for a station the document
    /// refuses, and the refusal would reach the person in English.
    #[test]
    fn the_step_reaches_every_station_the_document_takes_and_closes_the_ring() {
        let mut seen = vec![Hang::WAIST];
        let mut station = Hang::WAIST;
        for _ in 0..MeasureSet::GIRTHS.len() {
            station = next(station).expect("a girth steps to a girth");
            assert!(Hang::names_a_ring(station), "{station}");
            if station == Hang::WAIST {
                break;
            }
            assert!(!seen.contains(&station), "{station} twice round");
            seen.push(station);
        }
        assert_eq!(station, Hang::WAIST, "the ring closes");
        assert_eq!(seen.len(), MeasureSet::GIRTHS.len(), "{seen:?}");
        for name in MeasureSet::LENGTHS.iter().chain(&MeasureSet::WHOLE) {
            assert_eq!(next(name), None, "{name}");
        }
        assert_eq!(next("largo_manga"), None);
    }

    /// A run whose ends are on two pieces is refused in Spanish, and not by
    /// repeating what the document would have said.
    #[test]
    fn a_run_across_two_pieces_is_refused_in_spanish() {
        let doc = block::trouser_front();
        let run = waist(&doc);
        assert_eq!(refused(&doc, run), None, "one piece, both ends");
        let split = EdgeRange {
            tail: EdgeAnchor {
                piece: PieceKey::new(9, 0),
                ..run.tail
            },
            ..run
        };
        let why = refused(&doc, split).expect("a split run cannot be hung");
        assert_eq!(why, SPLIT);
        assert_ne!(
            why,
            DocError::SplitHang.to_string(),
            "in Spanish, and whole"
        );
    }

    /// And so is a product that resolves against no body.
    #[test]
    fn a_product_that_resolves_against_no_body_is_refused_in_spanish() {
        let mut doc = block::trouser_front();
        doc.resolve_with = MannequinKey::new(9, 0);
        assert_eq!(doc.measures(), None, "the key names no body");
        assert_eq!(refused(&doc, waist(&doc)), Some(NO_BODY));
    }
}
