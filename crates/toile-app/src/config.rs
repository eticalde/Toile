use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The schema version of the preferences file, bumped when its shape changes
/// so an older build refuses a file it would read wrong instead of guessing.
const VERSION: u32 = 1;

/// How many recent patterns to keep.
const RECENTS: usize = 10;

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
    pub window: Option<[f32; 4]>,
    /// Patterns opened or saved, most recent first.
    pub recents: Vec<PathBuf>,
    /// The folder the file dialog should open in next.
    pub last_dir: Option<PathBuf>,
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
    /// Reads the preferences, or the defaults when there is nothing to read.
    ///
    /// Any fault — no config directory, no file, unreadable, or a version this
    /// build does not know — yields the defaults. Preferences never stop the
    /// app from starting.
    pub fn load() -> Prefs {
        let Some(path) = path() else {
            return Prefs::default();
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Prefs::default();
        };
        match serde_json::from_str::<Stored>(&text) {
            Ok(stored) if stored.version == VERSION => stored.prefs,
            _ => Prefs::default(),
        }
    }

    /// Writes the preferences, creating the config directory if it is missing.
    ///
    /// Best-effort: a failure to write leaves the app running with what it has.
    pub fn save(&self) {
        let Some(path) = path() else {
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
            let _ = std::fs::write(&path, text);
        }
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
fn path() -> Option<PathBuf> {
    Some(config_dir()?.join("Toile").join("prefs.json"))
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

/// The OS config directory, resolved from the environment.
///
/// A hand-rolled resolver rather than a crate: the whole need is one path on
/// macOS and Linux, and the obvious crate for it (`directories`) pulls an
/// MPL-2.0 dependency the licence gate rejects. `None` where the environment
/// does not say — the app then runs without remembering, which is the whole
/// cost of a missing preference.
fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
    }
}
