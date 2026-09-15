use toile_engine::draft::{Command, Persona, Snapshot};
use toile_engine::session::Session;

use super::{NEW_NAME, Stand};
use crate::library::{Appended, Library, LibraryError};
use crate::tabs::UNNAMED;

/// The name the history keeps a person's copy under.
const USE: &str = "usar persona";
/// The name the history keeps the link a save rewrites under.
const SAVE: &str = "guardar en biblioteca";

/// What a save did to the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrote {
    /// A file for a new person.
    Created,
    /// A session after the person's earlier ones.
    Recorded,
    /// Nothing: the person's current session was already this one.
    Nothing,
}

/// A body saved to the library, as the panel reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    /// The person's name in the library.
    pub name: String,
    /// The day of the session the body is linked to now.
    pub date: String,
    /// What reached the disk.
    pub wrote: Wrote,
}

impl Stand {
    /// Puts a copy of the person's current session on the stand, linked to
    /// the file `stem` names.
    ///
    /// With a product open the copy joins it as the body the pattern resolves
    /// with, in one undo entry, as a new body does; with none it takes the
    /// loose body's place, and nothing keeps it.
    pub fn use_persona(&mut self, session: &mut Session, stem: &str, persona: &Persona) {
        let mut set = match persona.to_mannequin(stem) {
            Ok(set) => set,
            Err(why) => {
                let why = format!("«{}» no se puede usar: {why}", persona.name);
                self.refusal = Some((why, session.revision()));
                return;
            }
        };
        // Bodies are chosen by name, and the person may be in the product
        // already: the copy takes the next free name, and its link, not its
        // name, says whose it is.
        let base = persona.name.trim();
        let base = if base.is_empty() { NEW_NAME } else { base };
        set.name = Self::free_name(session, base);
        self.add_as(session, set, USE);
    }

    /// Saves the body on the stand to the library as a session dated `day`,
    /// and links the body to what the file now holds.
    ///
    /// A body linked to a person the library holds adds the session to her;
    /// any other body becomes a new person under its own name.
    ///
    /// # Errors
    /// The library's, when the person cannot be read or written. The product
    /// is left untouched then.
    pub fn save_to(
        &mut self,
        session: &mut Session,
        library: &Library,
        day: String,
    ) -> Result<Saved, LibraryError> {
        self.release(session);
        let body = self.body(session).clone();
        let snapshot = Snapshot {
            date: day,
            values: body.values.clone(),
            phenotype: body.phenotype,
        };
        let appended = body.origin.as_ref().map(|origin| {
            let stem = origin.persona.clone();
            (library.append(&stem, snapshot.clone()), stem)
        });
        let (stem, persona, wrote) = match appended {
            Some((Ok(Appended::Recorded(persona)), stem)) => (stem, persona, Wrote::Recorded),
            Some((Ok(Appended::AlreadyCurrent(persona)), stem)) => (stem, persona, Wrote::Nothing),
            // A link to a file that is gone is a person nobody keeps any more,
            // and the body is filed afresh rather than refused.
            Some((Err(LibraryError::Missing(_)), _)) | None => {
                let name = body.name.trim();
                let persona = Persona {
                    name: if name.is_empty() { UNNAMED } else { name }.to_owned(),
                    notes: String::new(),
                    taken: vec![snapshot],
                };
                (library.create(&persona)?, persona, Wrote::Created)
            }
            Some((Err(why), _)) => return Err(why),
        };
        self.relink(session, &stem, &persona);
        Ok(Saved {
            date: persona
                .current()
                .map(|current| current.date.clone())
                .unwrap_or_default(),
            name: persona.name,
            wrote,
        })
    }

    /// Points the body's link at the person's current session, leaving its
    /// tape and its shape as they are.
    ///
    /// Through the history rather than around it: the link is bytes of the
    /// product, so the file has to turn dirty for autosave to keep it; and an
    /// undo or redo replays whole bodies, so a link written outside the
    /// history would be overwritten by the next step across an earlier update.
    fn relink(&mut self, session: &mut Session, stem: &str, persona: &Persona) {
        let Ok(copy) = persona.to_mannequin(stem) else {
            return;
        };
        let body = self.body(session);
        if body.origin == copy.origin {
            return;
        }
        let (values, phenotype) = (body.values.clone(), body.phenotype);
        let Some(draft) = session.draft() else {
            self.loose.origin = copy.origin;
            return;
        };
        let command = Command::RefreshMannequin {
            mannequin: draft.doc().resolve_with,
            values,
            phenotype,
            origin: copy.origin,
        };
        session.begin_gesture(SAVE);
        let answer = session.edit(command);
        session.end_gesture();
        self.answer(session, answer);
    }
}
