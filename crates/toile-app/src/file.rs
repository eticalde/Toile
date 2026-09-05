mod bar;
mod dialog;

use std::path::{Path, PathBuf};

pub use bar::show as bar;
pub use dialog::{confirm_discard, open, save_as, svg_target, write};
use toile_engine::draft::Doc;

/// The base block Toile ships, in the very bytes its serialiser writes.
///
/// It is compiled in rather than read from disk so that the example is on the
/// menu wherever the program was started from, and so that a build proves the
/// file it ships can be read back.
const EXAMPLE: &str = include_str!("../../../assets/pantalon-base.toile");

/// What a pattern that has never been written is called.
const UNTITLED: &str = "Sin título";

/// The extension a pattern is kept under, and the SVG it is drawn into.
pub const PATTERN_EXT: &str = "toile";
pub const DRAWING_EXT: &str = "svg";

/// What the interface asks be done with the file a pattern lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Clear the table and start something new.
    New,
    /// Put a pattern from disk on it.
    Open,
    /// Put the base block Toile ships on it.
    Example,
    /// Write the pattern back where it came from.
    Save,
    /// Write it somewhere else, and go on from there.
    SaveAs,
    /// Draw it into an SVG at true scale.
    Svg,
}

/// Something the file has to say, and the revision it was said at.
///
/// A word about a file stops being true the moment the document moves on, so
/// it is kept with the revision it belongs to rather than until something
/// else happens to be said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// What it says.
    pub text: String,
    /// Whether what it says is that something went wrong.
    pub bad: bool,
    /// The revision of the document it was said at.
    pub at: u64,
}

/// The file a pattern lives in.
///
/// The document itself belongs to the session; what lives here is where it
/// came from, and the revision it was at when it was last written — which is
/// the whole of what an unsaved change is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct File {
    path: Option<PathBuf>,
    saved: u64,
    notice: Option<Notice>,
}

impl File {
    /// The base block, read back from the file that ships with the program.
    ///
    /// # Panics
    /// Never in a build that passed its tests: the asset is written by the
    /// same serialiser that reads it, and a test holds the two together.
    pub fn example() -> Doc {
        Doc::from_json(EXAMPLE).expect("the block Toile ships is written by its own serialiser")
    }

    /// Where the pattern is kept, once it has been anywhere.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// What the pattern is called, or that it has never been written.
    pub fn name(&self) -> &str {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or(UNTITLED)
    }

    /// The name without its extension, which is what a save dialog offers.
    pub fn stem(&self) -> &str {
        self.path
            .as_deref()
            .and_then(Path::file_stem)
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or(UNTITLED)
    }

    /// Whether the document has moved since the file was last written.
    pub fn dirty(&self, revision: u64) -> bool {
        revision != self.saved
    }

    /// What the last thing done to the file had to say, for as long as the
    /// document it was said about has not moved on.
    pub fn notice(&self, revision: u64) -> Option<&Notice> {
        self.notice.as_ref().filter(|notice| notice.at == revision)
    }

    /// Reports what was done, in place of whatever was being reported before.
    pub fn say(&mut self, text: impl Into<String>, revision: u64) {
        self.notice = Some(Notice {
            text: text.into(),
            bad: false,
            at: revision,
        });
    }

    /// Reports what could not be done.
    pub fn warn(&mut self, text: impl Into<String>, revision: u64) {
        self.notice = Some(Notice {
            text: text.into(),
            bad: true,
            at: revision,
        });
    }

    /// Records that the document and the file now hold the same pattern.
    pub fn settle(&mut self, path: Option<PathBuf>, revision: u64) {
        self.path = path;
        self.saved = revision;
        self.notice = None;
    }
}

#[cfg(test)]
mod tests {
    use toile_engine::draft::{Draft, block};
    use toile_engine::export;

    use super::*;

    #[test]
    fn the_example_is_the_block_toile_ships() {
        assert_eq!(File::example(), block::trouser_front());
    }

    /// The whole of what the file phase promised: a pattern goes out of the
    /// program, comes back the same, and can be drawn at true scale.
    #[test]
    fn a_pattern_saved_reopened_and_drawn_is_the_same_pattern() {
        let written = File::example().to_canonical_json();
        let read = Doc::from_json(&written).expect("what the program wrote it can read");
        assert_eq!(read.to_canonical_json(), written);
        let draft = Draft::from_doc(read).expect("the example resolves");
        let drawing = export::to_svg(&draft).expect("the example draws");
        assert!(drawing.contains("<svg"), "{drawing}");
    }

    #[test]
    fn a_pattern_that_has_never_been_written_is_untitled() {
        let file = File::default();
        assert_eq!(file.name(), UNTITLED);
        assert_eq!(file.stem(), UNTITLED);
    }

    #[test]
    fn a_written_pattern_is_named_after_its_file() {
        let mut file = File::default();
        file.settle(Some(PathBuf::from("/tmp/pantalón.toile")), 4);
        assert_eq!(file.name(), "pantalón.toile");
        assert_eq!(file.stem(), "pantalón");
    }

    #[test]
    fn a_document_that_has_moved_since_it_was_written_is_dirty() {
        let mut file = File::default();
        file.settle(Some(PathBuf::from("/tmp/base.toile")), 4);
        assert!(!file.dirty(4));
        assert!(file.dirty(5));
    }

    #[test]
    fn a_notice_is_forgotten_once_the_document_moves_on() {
        let mut file = File::default();
        file.warn("no se pudo escribir", 2);
        assert_eq!(file.notice(2).map(|said| said.bad), Some(true));
        assert_eq!(file.notice(3), None);
    }
}
