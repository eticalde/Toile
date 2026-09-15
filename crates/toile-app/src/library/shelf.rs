use toile_engine::draft::Persona;

use super::{Library, LibraryError, Listed};

/// The library as the app last read it.
///
/// It is read again when something can have changed it — the app starting, a
/// product opening, a save from the app — and never once a frame, since a
/// listing parses every person in the folder.
#[derive(Debug)]
pub struct Shelf {
    library: Option<Library>,
    listed: Result<Vec<Listed>, LibraryError>,
}

impl Shelf {
    /// The library in the platform's data directory, read once.
    ///
    /// A test build never looks that folder up, so no test can reach anyone's
    /// real library.
    pub fn platform() -> Shelf {
        #[cfg(not(test))]
        let library = crate::config::library_dir().map(Library::at);
        #[cfg(test)]
        let library = None;
        Shelf::over(library)
    }

    /// The shelf over `library`, read once. `None` is a system that names no
    /// data directory, where there is no library to keep anyone in.
    pub fn over(library: Option<Library>) -> Shelf {
        let mut shelf = Shelf {
            library,
            listed: Ok(Vec::new()),
        };
        shelf.refresh();
        shelf
    }

    /// Reads the folder again.
    pub fn refresh(&mut self) {
        self.listed = self.library.as_ref().map_or(Ok(Vec::new()), Library::list);
    }

    /// The library itself, for a write.
    pub fn library(&self) -> Option<&Library> {
        self.library.as_ref()
    }

    /// Every file the last read found, or why the folder could not be read.
    pub fn listed(&self) -> Result<&[Listed], &LibraryError> {
        self.listed.as_deref()
    }

    /// The person filed under `stem`, when her file read the last time.
    pub fn persona(&self, stem: &str) -> Option<&Persona> {
        self.listed()
            .ok()?
            .iter()
            .find(|listed| listed.stem == stem)?
            .persona
            .as_ref()
            .ok()
    }
}
