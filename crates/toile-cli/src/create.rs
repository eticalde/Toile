use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// Writes a file that must not exist yet.
///
/// The owner's folders are the owner's: a run that would land on a file already
/// there says so and writes nothing, rather than replacing work nobody asked it
/// to replace.
pub fn file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|why| format!("no se pudo crear «{}»: {why}", path.display()))?;
    file.write_all(bytes)
        .map_err(|why| format!("no se pudo escribir «{}»: {why}", path.display()))
}

/// Says a path is taken, in the one wording every subcommand uses for it.
pub fn taken(path: &Path) -> String {
    format!("«{}» ya existe: no se sobrescribe", path.display())
}
