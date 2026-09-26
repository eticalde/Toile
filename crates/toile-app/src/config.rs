/// The size of paper the studio prints on, and the word it is remembered as.
mod paper;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub use paper::Paper;
use serde::{Deserialize, Serialize};

/// The schema version of the preferences file, bumped when its shape changes
/// so an older build refuses a file it would read wrong instead of guessing.
///
/// A new optional field is not a new shape: an older build passes over a key
/// it does not know and reads everything else right, where a bump would make
/// it throw the window and the recent files away with it.
const VERSION: u32 = 1;

/// How many recent patterns to keep.
const RECENTS: usize = 10;

/// The folder Toile keeps its own files in, under each base directory.
const APP: &str = "Toile";

/// What the app remembers between runs: window, recent files, last folder.
///
/// Never the document — that is the `.toile` file. These are conveniences, so
/// every path here is best-effort: a missing or unreadable file loads the
/// defaults, and a write that fails is dropped rather than raised. Losing a
/// preference is a shrug; losing a pattern is not, and patterns do not live
/// here.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// Window geometry as `[x, y, width, height]`: outer top-left, inner size.
    ///
    /// The floating geometry, tracked only while the window is not filling the
    /// screen, so a maximized window still knows the size to restore to.
    pub window: Option<[f32; 4]>,
    /// Whether the window was left filling the screen, to open it that way
    /// again. `None` when it has never been recorded — an old preferences file,
    /// or the first run — which opens maximized: filling the screen is the
    /// default, and only a window deliberately made to float records `false`.
    pub maximized: Option<bool>,
    /// Patterns opened or saved, most recent first.
    pub recents: Vec<PathBuf>,
    /// The folder the file dialog should open in next.
    pub last_dir: Option<PathBuf>,
    /// The library stem of the person a new product is cut for.
    ///
    /// A preference of this installation, never of Toile: with none, or with
    /// a stem the library does not hold, a new product starts as it always
    /// did. Left out of the file while unset, so a file written before it
    /// existed reads and writes back byte for byte.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_persona: Option<String>,
    /// The paper the sheets of a print are laid out on.
    ///
    /// A preference of this installation, never of Toile: which ream is in the
    /// printer says nothing about the garment, and the same pattern printed in
    /// two studios is one pattern on two sizes of paper. Reached through
    /// `paper`, so that an unset preference means one thing in one place. Left
    /// out of the file while unset, so a file written before it existed reads
    /// and writes back byte for byte.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "paper::known"
    )]
    paper: Option<Paper>,
    /// The file these were read from, and the one `save` writes back to.
    /// `None` writes nowhere.
    #[serde(skip)]
    file: Option<PathBuf>,
}

/// The file as it sits on disk: the version alongside the preferences, so a
/// future shape can be told from this one.
#[derive(Serialize, Deserialize)]
struct Stored {
    version: u32,
    #[serde(flatten)]
    prefs: Prefs,
}

impl Prefs {
    /// Reads the preferences from the platform's config directory, or the
    /// defaults when there is nothing to read.
    pub fn load() -> Prefs {
        path().map_or_else(Prefs::default, |file| Prefs::load_from(&file))
    }

    /// Reads the preferences kept in `file`, which `save` then writes back.
    ///
    /// Any fault — no file, unreadable, or a version this build does not know
    /// — yields the defaults. Preferences never stop the app from starting.
    pub fn load_from(file: &Path) -> Prefs {
        let stored = std::fs::read_to_string(file)
            .ok()
            .and_then(|text| serde_json::from_str::<Stored>(&text).ok());
        let mut prefs = match stored {
            Some(stored) if stored.version == VERSION => stored.prefs,
            _ => Prefs::default(),
        };
        prefs.file = Some(file.to_path_buf());
        prefs
    }

    /// Writes the preferences back to their file, creating its folder if it is
    /// missing.
    ///
    /// Best-effort: a failure to write leaves the app running with what it has.
    pub fn save(&self) {
        let Some(path) = &self.file else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let stored = Stored {
            version: VERSION,
            prefs: self.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&stored) {
            let _ = std::fs::write(path, text);
        }
    }

    /// Makes the person filed under `stem` the one new products are cut for,
    /// taking the place from whoever had it; the person who already had it
    /// gives it up instead, and new products start blank again.
    pub fn toggle_default(&mut self, stem: &str) {
        self.default_persona = if self.default_persona.as_deref() == Some(stem) {
            None
        } else {
            Some(stem.to_owned())
        };
    }

    /// The paper a print is laid out on.
    pub fn paper(&self) -> Paper {
        self.paper.unwrap_or_default()
    }

    /// Lays every print from now on out on the next size of paper.
    ///
    /// Stepped and not chosen from a list: there are two sizes, and a control
    /// that steps says which one is current in the room a list would need for
    /// its handle alone.
    pub fn step_paper(&mut self) {
        self.paper = Some(self.paper().next());
    }

    /// Records a pattern as the most recent, and its folder as the last used.
    ///
    /// The path moves to the front, a duplicate is not kept twice, and the
    /// list is capped so it cannot grow without bound.
    pub fn remember(&mut self, file: &Path) {
        self.recents.retain(|p| p != file);
        self.recents.insert(0, file.to_path_buf());
        self.recents.truncate(RECENTS);
        if let Some(dir) = file.parent() {
            self.last_dir = Some(dir.to_path_buf());
        }
    }
}

/// The preferences file, under the platform's config directory for Toile.
///
/// A test build names no file, so no test can read or write the real
/// preferences: every test keeps its own in a scratch folder.
fn path() -> Option<PathBuf> {
    let base = base_dir(Base::Config, |name| std::env::var_os(name)).filter(|_| !cfg!(test))?;
    Some(base.join(APP).join("prefs.json"))
}

/// The folder the library of people is kept in, under the platform's data
/// directory for Toile.
///
/// A test build has no such function, so no test can name the real library:
/// every test hands the library a scratch folder instead.
#[cfg(not(test))]
pub fn library_dir() -> Option<PathBuf> {
    Some(
        base_dir(Base::Data, |name| std::env::var_os(name))?
            .join(APP)
            .join("personas"),
    )
}

/// The folder baked body fields are kept in, beside the library of people.
///
/// Under the data directory and not the cache one: a body field is tens of
/// megabytes and costs half a second to make again, and the platform is free
/// to empty a cache folder whenever it likes. It holds voxels and a hash and
/// never a name or a tape, but it is derived from body measurements, so it
/// stays on the machine beside the people it was derived from.
///
/// A test build has no such function, so no test can name the real cache:
/// every test hands the cache a scratch folder instead.
#[cfg(not(test))]
pub fn cache_dir() -> Option<PathBuf> {
    Some(
        base_dir(Base::Data, |name| std::env::var_os(name))?
            .join(APP)
            .join("sdf"),
    )
}

/// The folder patterns are kept in by default, where the file dialogs open
/// until the person has saved somewhere else.
///
/// A visible place under the user's documents, not the hidden config tree:
/// patterns are the person's own files, to find and back up like any other.
/// `None` where the environment does not name a home.
pub fn patterns_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Documents").join("Toile"))
}

/// Which of the platform's base directories a file belongs under.
#[derive(Debug, Clone, Copy)]
enum Base {
    /// Settings, which the app can lose and start again without.
    Config,
    /// What the person made and would miss.
    Data,
}

/// A base directory of the OS, resolved from the environment `var` reads.
///
/// A hand-rolled resolver rather than a crate: the whole need is two paths on
/// macOS and Linux, and the obvious crate for it (`directories`) pulls an
/// MPL-2.0 dependency the licence gate rejects. `None` where the environment
/// does not say.
fn base_dir(base: Base, var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let home = var("HOME").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    {
        // macOS keeps settings and data in the one place.
        let _ = base;
        home.map(|home| home.join("Library/Application Support"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let (xdg, fallback) = match base {
            Base::Config => ("XDG_CONFIG_HOME", ".config"),
            Base::Data => ("XDG_DATA_HOME", ".local/share"),
        };
        var(xdg)
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .or_else(|| home.map(|home| home.join(fallback)))
    }
}

#[cfg(test)]
mod tests;
