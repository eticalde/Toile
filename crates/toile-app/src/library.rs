mod atomic;
pub mod shelf;
#[cfg(test)]
pub(crate) mod tests;
pub mod today;

use std::path::{Path, PathBuf};
use std::{fs, io};

/// The extension a person is kept under.
pub use toile_engine::draft::PERSONA_EXTENSION as PERSONA_EXT;
use toile_engine::draft::{Origin, Persona, PersonaError, Snapshot};

use self::atomic::Staged;

/// The people a user keeps, one file each, in one folder.
///
/// The folder is handed in, never looked up: the app passes the platform's
/// data directory and a test passes a scratch one, so no test can reach
/// anyone's real library. Nothing here deletes a person's file, and every
/// write lands whole, as a temporary file renamed into place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    dir: PathBuf,
}

/// One file in the library, and the person it holds or why it holds none.
#[derive(Debug)]
pub struct Listed {
    /// The file name without its extension: the identity a document's link
    /// names.
    pub stem: String,
    /// The person, or what is wrong with the file.
    pub persona: Result<Persona, LibraryError>,
}

/// What saving a session did to a person's file.
#[derive(Debug, Clone, PartialEq)]
pub enum Appended {
    /// The session was written after every earlier one.
    Recorded(Persona),
    /// The session repeats the current one, day and fingerprint both, so the
    /// file already said it and was left alone.
    AlreadyCurrent(Persona),
}

/// What stops the library from reading or writing a person, in the words the
/// interface shows.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    /// A stem no library file could have, such as one that climbs out.
    #[error("«{0}» no puede nombrar un archivo de la biblioteca")]
    NotAStem(String),
    /// No file for the stem.
    #[error("no hay nadie en la biblioteca con el archivo «{0}»")]
    Missing(String),
    /// A file that does not read as a person.
    #[error("«{file}» no es una persona: {why}")]
    Unreadable {
        file: String,
        #[source]
        why: PersonaError,
    },
    /// A person the format would not read back, refused before any write.
    #[error("la persona no se puede guardar: {0}")]
    Refused(#[source] PersonaError),
    /// The file system said no.
    #[error("no se pudo {doing} «{file}»: {why}")]
    Io {
        doing: &'static str,
        file: String,
        #[source]
        why: io::Error,
    },
}

impl Library {
    /// The library kept in `dir`, which need not exist until the first person
    /// is created.
    pub fn at(dir: impl Into<PathBuf>) -> Library {
        Library { dir: dir.into() }
    }

    /// Every file in the library that claims to be a person, by stem.
    ///
    /// A file that does not read is listed with the reason, never left out.
    /// Hidden files are not people and are passed over: a save's own
    /// temporary file, or the `._` shadow macOS writes beside a file on a
    /// volume that cannot hold its metadata.
    ///
    /// # Errors
    /// `LibraryError::Io` when the folder is there and cannot be read. A
    /// folder not made yet is an empty library.
    pub fn list(&self) -> Result<Vec<Listed>, LibraryError> {
        atomic::sweep(&self.dir);
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(why) => return Err(io_error("leer", &self.dir, why)),
        };
        let suffix = format!(".{PERSONA_EXT}");
        let mut listed = Vec::new();
        for entry in entries {
            let name = entry
                .map_err(|why| io_error("leer", &self.dir, why))?
                .file_name();
            let name = name.to_string_lossy();
            if let Some(stem) = name.strip_suffix(&suffix)
                && !name.starts_with('.')
            {
                let persona = self.load(stem);
                listed.push(Listed {
                    stem: stem.to_owned(),
                    persona,
                });
            }
        }
        listed.sort_by(|a, b| a.stem.cmp(&b.stem));
        Ok(listed)
    }

    /// The person a stem names.
    ///
    /// # Errors
    /// `NotAStem` for a stem no library file could have, `Missing` when there
    /// is no such file, `Unreadable` when the file is not a person this build
    /// reads, and `Io` when it cannot be read at all.
    pub fn load(&self, stem: &str) -> Result<Persona, LibraryError> {
        let path = self.path(stem)?;
        let text = fs::read_to_string(&path).map_err(|why| match why.kind() {
            io::ErrorKind::NotFound => LibraryError::Missing(stem.to_owned()),
            _ => io_error("leer", &path, why),
        })?;
        Persona::from_json(&text).map_err(|why| LibraryError::Unreadable {
            file: file_name(&path),
            why,
        })
    }

    /// Writes a new person into the library and returns the stem it took.
    ///
    /// The stem is the name folded to a slug, and a stem already on disk is
    /// never written over: the new file takes the next free `-2`, `-3`, and
    /// so on.
    ///
    /// # Errors
    /// `Refused` for a person the format would not read back, checked before
    /// the disk is touched, and `Io` when the file cannot be written.
    pub fn create(&self, persona: &Persona) -> Result<String, LibraryError> {
        let text = persona.to_canonical_json().map_err(LibraryError::Refused)?;
        let base = Origin::stem_of(&persona.name);
        fs::create_dir_all(&self.dir).map_err(|why| io_error("crear", &self.dir, why))?;
        let staged = Staged::write(&self.dir, &base, &text)
            .map_err(|why| io_error("escribir", &self.dir, why))?;
        let mut n = 1u32;
        loop {
            let stem = if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            };
            let path = self.path(&stem)?;
            if staged
                .claim(&path)
                .map_err(|why| io_error("escribir", &path, why))?
            {
                return Ok(stem);
            }
            n += 1;
        }
    }

    /// Saves a new session with the tape after the person's earlier ones.
    ///
    /// The file is read afresh, so an edit made to it since it was listed is
    /// kept, and it is written back whole in its canonical form. A second
    /// session on one day is a second session: both stay, in the order saved,
    /// and the later is current. Only a session that repeats the current one
    /// exactly, the same day and the same fingerprint, writes nothing, so
    /// saving twice does not double the history.
    ///
    /// # Errors
    /// `NotAStem` or `Missing` for a person the library does not hold;
    /// `Unreadable` for a file this build cannot read, which is then never
    /// rewritten; `Refused` for a session dated before the current one or not
    /// written `YYYY-MM-DD`; and `Io` when the write fails, which leaves the
    /// file as it was.
    pub fn append(&self, stem: &str, snapshot: Snapshot) -> Result<Appended, LibraryError> {
        let mut persona = self.load(stem)?;
        let repeat = persona.current().is_some_and(|current| {
            current.date == snapshot.date && current.fingerprint() == snapshot.fingerprint()
        });
        if repeat {
            return Ok(Appended::AlreadyCurrent(persona));
        }
        persona.taken.push(snapshot);
        let text = persona.to_canonical_json().map_err(LibraryError::Refused)?;
        let path = self.path(stem)?;
        Staged::write(&self.dir, stem, &text)
            .and_then(|staged| staged.replace(&path))
            .map_err(|why| io_error("escribir", &path, why))?;
        Ok(Appended::Recorded(persona))
    }

    /// The file a stem names, for a stem that stays inside the library.
    fn path(&self, stem: &str) -> Result<PathBuf, LibraryError> {
        if Origin::is_stem(stem) {
            Ok(self.dir.join(format!("{stem}.{PERSONA_EXT}")))
        } else {
            Err(LibraryError::NotAStem(stem.to_owned()))
        }
    }
}

fn io_error(doing: &'static str, path: &Path, why: io::Error) -> LibraryError {
    LibraryError::Io {
        doing,
        file: file_name(path),
        why,
    }
}

/// What a path is called, for a sentence that has to name it.
fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}
