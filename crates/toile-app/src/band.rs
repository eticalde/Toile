use eframe::egui::{self, Margin, RichText};
use toile_engine::draft::{Command, Doc, MannequinKey};
use toile_engine::session::Session;

use crate::library::shelf::Shelf;
use crate::tabs::UNNAMED;
use crate::theme::Theme;
use crate::widgets::button_secondary;

/// The name the history keeps an update under.
const UPDATE: &str = "actualizar maniquí";

/// A body of the product copied from a person the library has moved on from.
#[derive(Debug, Clone)]
pub struct Offer {
    /// The body, by its key in the product.
    pub mannequin: MannequinKey,
    /// What the product calls the body.
    pub name: String,
    /// The edit that brings the body up to the person's current session.
    update: Command,
}

/// What the two buttons of an offer ask for.
enum Answer {
    Update(usize),
    Keep(usize),
}

/// The offers to update the bodies of the product on the table.
///
/// Worked out when a product opens, by reading alone: opening a product never
/// changes it, so an offer stays an offer until somebody answers it.
#[derive(Debug, Default)]
pub struct Band {
    offers: Vec<Offer>,
    /// The bodies whose offer was declined. They stay declined until another
    /// product takes the table and a new band is worked out.
    kept: Vec<MannequinKey>,
    /// Why the last update was refused, while its offer still stands.
    refused: Option<String>,
}

impl Band {
    /// The band for a product just put on the table.
    ///
    /// The library is read again first: it may have changed while another
    /// product was open, or by hand while the app ran.
    pub fn opened(shelf: &mut Shelf, session: &Session) -> Band {
        shelf.refresh();
        let mut band = Band::default();
        band.recheck(shelf, session);
        band
    }

    /// Works the offers out again after the library changed under the product,
    /// leaving every declined body declined.
    pub fn recheck(&mut self, shelf: &Shelf, session: &Session) {
        self.refused = None;
        self.offers = session
            .draft()
            .map(|draft| offers(shelf, draft.doc(), &self.kept))
            .unwrap_or_default();
    }

    /// The offers standing, in the order the product holds the bodies.
    pub fn offers(&self) -> &[Offer] {
        &self.offers
    }

    /// Why the last update was refused, while its offer stands.
    pub fn refused(&self) -> Option<&str> {
        self.refused.as_deref()
    }

    /// Brings the body of offer `at` up to its person, as one undo entry.
    ///
    /// A refused update leaves the offer standing and says why.
    pub fn update(&mut self, session: &mut Session, at: usize) {
        let Some(offer) = self.offers.get(at) else {
            return;
        };
        session.begin_gesture(UPDATE);
        let answer = session.edit(offer.update.clone());
        session.end_gesture();
        match answer {
            Ok(()) => {
                self.offers.remove(at);
                self.refused = None;
            }
            Err(why) => {
                self.refused = Some(format!("no se pudo actualizar «{}»: {why}", offer.name));
            }
        }
    }

    /// Declines offer `at` while this product stays open, writing nothing.
    pub fn keep(&mut self, at: usize) {
        if at < self.offers.len() {
            let offer = self.offers.remove(at);
            self.kept.push(offer.mannequin);
            self.refused = None;
        }
    }
}

/// Every body of `doc` linked to a person the shelf holds whose current
/// session is no longer the one the body copied.
///
/// A person whose file is gone, or no longer reads, offers nothing: there is
/// nothing to update from.
fn offers(shelf: &Shelf, doc: &Doc, kept: &[MannequinKey]) -> Vec<Offer> {
    doc.mannequins
        .iter()
        .filter(|(key, _)| !kept.contains(key))
        .filter_map(|(mannequin, set)| {
            let origin = set.origin.as_ref()?;
            let persona = shelf.persona(&origin.persona)?;
            if !persona.differs_from(origin) {
                return None;
            }
            let update = persona.refresh(&origin.persona, mannequin).ok()?;
            let name = if set.name.is_empty() {
                UNNAMED
            } else {
                &set.name
            };
            Some(Offer {
                mannequin,
                name: name.to_owned(),
                update,
            })
        })
        .collect()
}

/// The band under the tab bar, drawn while an offer stands.
///
/// Above the tabs rather than inside one, because a product opens on whichever
/// tab was in front; and a panel rather than a dialog, so the product can be
/// worked with the offer left unanswered.
pub fn show(ui: &mut egui::Ui, theme: &Theme, band: &mut Band, session: &mut Session) {
    if band.offers().is_empty() {
        return;
    }
    let frame = egui::Frame::new()
        .fill(theme.raised)
        .inner_margin(Margin::symmetric(16, 6));
    let mut asked = None;
    egui::Panel::top("banda")
        .resizable(false)
        .show_separator_line(false)
        .frame(frame)
        .show(ui, |ui| {
            for (at, offer) in band.offers().iter().enumerate() {
                ui.horizontal(|ui| {
                    let said = format!("Las medidas de {} difieren de tu biblioteca", offer.name);
                    ui.label(RichText::new(said).size(12.0).color(theme.ink));
                    ui.label(RichText::new("—").size(12.0).color(theme.muted));
                    if button_secondary(ui, theme, "Actualizar").clicked() {
                        asked = Some(Answer::Update(at));
                    }
                    if button_secondary(ui, theme, "Mantener").clicked() {
                        asked = Some(Answer::Keep(at));
                    }
                });
            }
            if let Some(why) = band.refused() {
                ui.label(RichText::new(why).size(11.0).color(theme.alert));
            }
        });
    match asked {
        Some(Answer::Update(at)) => band.update(session, at),
        Some(Answer::Keep(at)) => band.keep(at),
        None => {}
    }
}
