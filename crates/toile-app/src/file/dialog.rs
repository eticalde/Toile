use std::path::{Path, PathBuf};

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use toile_engine::draft::Doc;

use super::{DRAWING_EXT, PATTERN_EXT};

/// What the dialogs call the two kinds of file.
const PATTERN: &str = "Patrón de Toile";
const DRAWING: &str = "Dibujo SVG";

/// A pattern the person picked, and what came out of the file.
///
/// `None` when they picked nothing, which is not a failure and says nothing.
pub fn open() -> Option<Result<(PathBuf, Doc), String>> {
    let path = FileDialog::new()
        .add_filter(PATTERN, &[PATTERN_EXT])
        .set_title("Abrir patrón")
        .pick_file()?;
    Some(read(path))
}

/// The pattern one file holds.
fn read(path: PathBuf) -> Result<(PathBuf, Doc), String> {
    let text = std::fs::read_to_string(&path)
        .map_err(|why| format!("no se pudo leer «{}»: {why}", name(&path)))?;
    let doc =
        Doc::from_json(&text).map_err(|why| format!("«{}» no es un patrón: {why}", name(&path)))?;
    Ok((path, doc))
}

/// Where the person wants the pattern written, with the extension it is kept
/// under whether or not they typed it.
pub fn save_as(stem: &str) -> Option<PathBuf> {
    let path = FileDialog::new()
        .add_filter(PATTERN, &[PATTERN_EXT])
        .set_file_name(format!("{stem}.{PATTERN_EXT}"))
        .set_title("Guardar patrón como")
        .save_file()?;
    Some(with_extension(path, PATTERN_EXT))
}

/// Where the person wants the drawing written.
pub fn svg_target(stem: &str) -> Option<PathBuf> {
    let path = FileDialog::new()
        .add_filter(DRAWING, &[DRAWING_EXT])
        .set_file_name(format!("{stem}.{DRAWING_EXT}"))
        .set_title("Exportar SVG a escala real")
        .save_file()?;
    Some(with_extension(path, DRAWING_EXT))
}

/// Writes a file, saying in the language of the interface what went wrong.
///
/// # Errors
/// The message shown to the person when the file cannot be written.
pub fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|why| format!("no se pudo escribir «{}»: {why}", name(path)))
}

/// Asks before work nobody has written down is thrown away.
pub fn confirm_discard(name: &str) -> bool {
    let answer = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title("Cambios sin guardar")
        .set_description(format!(
            "«{name}» tiene cambios sin guardar. ¿Descartarlos?"
        ))
        .set_buttons(MessageButtons::YesNo)
        .show();
    answer == MessageDialogResult::Yes
}

/// The same path, under the extension its kind of file is kept in.
///
/// A person who types a name without one gets the right file rather than a
/// pattern the program will not offer to open again.
fn with_extension(path: PathBuf, extension: &str) -> PathBuf {
    if path.extension().is_some() {
        return path;
    }
    path.with_extension(extension)
}

/// What a path is called, for a sentence that has to name it.
fn name(path: &Path) -> String {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_without_an_extension_gains_the_one_its_kind_is_kept_under() {
        let bare = with_extension(PathBuf::from("/tmp/pantalón"), PATTERN_EXT);
        assert_eq!(bare, PathBuf::from("/tmp/pantalón.toile"));
    }

    #[test]
    fn an_extension_the_person_typed_is_left_alone() {
        let typed = with_extension(PathBuf::from("/tmp/base.svg"), DRAWING_EXT);
        assert_eq!(typed, PathBuf::from("/tmp/base.svg"));
    }
}
