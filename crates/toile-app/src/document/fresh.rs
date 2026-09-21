use toile_engine::body::Collider;
use toile_engine::draft::Doc;
use toile_engine::session::Session;

use crate::library::shelf::Shelf;

/// The table a new product starts on, over the body `body_for` fits to its
/// document, or to none.
///
/// Cut for the installation's own person when `own` names one the shelf
/// holds: her current session, copied with the link back to her file, is the
/// product's one body and the one its pattern resolves with. Anything else —
/// no preference, a person gone from the library or unreadable, a library that
/// cannot be read — is the blank product an installation without the
/// preference has always had, and says nothing: the mannequin tab is where
/// that is told.
pub(crate) fn table(
    own: Option<&str>,
    shelf: &Shelf,
    mut body_for: impl FnMut(Option<&Doc>) -> Collider,
) -> Session {
    if let Some(doc) = own.and_then(|stem| cut_for(stem, shelf)) {
        let body = body_for(Some(&doc));
        if let Ok(session) = Session::from_doc(doc, body) {
            return session;
        }
    }
    Session::blank(body_for(None))
}

/// A document with nothing drawn yet, resolving against a copy of the person
/// filed under `stem`.
fn cut_for(stem: &str, shelf: &Shelf) -> Option<Doc> {
    let body = shelf.persona(stem)?.to_mannequin(stem).ok()?;
    Some(Doc::new(body))
}

#[cfg(test)]
mod tests;
