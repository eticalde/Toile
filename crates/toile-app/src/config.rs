use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The schema version of the preferences file, bumped when its shape changes
/// so an older build refuses a file it would read wrong instead of guessing.
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
    Some(
        base_dir(Base::Config, |name| std::env::var_os(name))?
            .join(APP)
            .join("prefs.json"),
    )
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
mod tests {
    use super::*;

    /// An environment holding only `pairs`.
    fn env(pairs: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs = pairs.to_vec();
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn with_no_home_there_is_no_base_directory() {
        assert_eq!(base_dir(Base::Config, env(&[])), None);
        assert_eq!(base_dir(Base::Data, env(&[])), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn on_macos_settings_and_data_share_application_support() {
        let home = env(&[("HOME", "/Users/ana"), ("XDG_DATA_HOME", "/elsewhere")]);
        let support = PathBuf::from("/Users/ana/Library/Application Support");
        assert_eq!(base_dir(Base::Config, &home), Some(support.clone()));
        assert_eq!(base_dir(Base::Data, &home), Some(support));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn elsewhere_data_follows_xdg_and_falls_back_under_home() {
        let bare = env(&[("HOME", "/home/ana")]);
        let data = base_dir(Base::Data, &bare);
        assert_eq!(data, Some(PathBuf::from("/home/ana/.local/share")));
        let config = base_dir(Base::Config, &bare);
        assert_eq!(config, Some(PathBuf::from("/home/ana/.config")));
        let set = env(&[("HOME", "/home/ana"), ("XDG_DATA_HOME", "/datos")]);
        assert_eq!(base_dir(Base::Data, &set), Some(PathBuf::from("/datos")));
        let relative = env(&[("HOME", "/home/ana"), ("XDG_DATA_HOME", "datos")]);
        let ignored = base_dir(Base::Data, &relative);
        assert_eq!(ignored, Some(PathBuf::from("/home/ana/.local/share")));
    }
}
