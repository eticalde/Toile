use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use toile_doc::PERSONA_EXTENSION;

/// The file `stem` names in the library kept in `dir`.
pub fn path(dir: &Path, stem: &str) -> PathBuf {
    dir.join(format!("{stem}.{PERSONA_EXTENSION}"))
}

/// Why a stem already taken is not written over.
pub fn taken(dir: &Path, stem: &str) -> String {
    format!(
        "ya hay una persona en la biblioteca con el archivo «{}»: no se sobrescribe; elige otro \
         nombre con --persona",
        path(dir, stem).display()
    )
}

/// Files `text` in the library kept in `dir` as the person under `stem`, whole
/// or not at all, and returns where.
///
/// The app numbers a taken stem instead; from the command line a taken stem
/// most likely means the same person imported twice, and a second file would
/// split her history in two. The temporary file is named the way the app
/// names its own, so one left behind by a run that died is swept the next time
/// the app lists the library.
pub fn file(dir: &Path, stem: &str, text: &str) -> Result<PathBuf, String> {
    let target = path(dir, stem);
    if fs::symlink_metadata(&target).is_ok() {
        return Err(taken(dir, stem));
    }
    fs::create_dir_all(dir)
        .map_err(|why| format!("no se pudo crear «{}»: {why}", dir.display()))?;
    let staged = Staged::write(dir, stem, text)
        .map_err(|why| format!("no se pudo escribir en «{}»: {why}", dir.display()))?;
    match staged.claim(&target) {
        Ok(true) => Ok(target),
        Ok(false) => Err(taken(dir, stem)),
        Err(why) => Err(format!("no se pudo escribir «{}»: {why}", target.display())),
    }
}

/// A complete file beside where it is going, under a hidden name; dropped, it
/// removes that name and only that name.
struct Staged {
    path: PathBuf,
}

impl Staged {
    /// Writes and flushes `text`, so what lands on the person's name is never
    /// a file still being written.
    fn write(dir: &Path, stem: &str, text: &str) -> io::Result<Staged> {
        let pid = std::process::id();
        let mut n = 0u64;
        let (mut file, staged) = loop {
            let path = dir.join(format!(".{stem}.{pid}.{n}.tmp"));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => break (file, Staged { path }),
                // Left by an earlier process with this id that died mid-save.
                Err(why) if why.kind() == io::ErrorKind::AlreadyExists => n += 1,
                Err(why) => return Err(why),
            }
        };
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        Ok(staged)
    }

    /// Puts the file at `target` only if that name is free, and says whether
    /// it did: a hard link cannot land on a name that exists, so a person the
    /// app files in the same instant is never written over.
    fn claim(&self, target: &Path) -> io::Result<bool> {
        match fs::hard_link(&self.path, target) {
            Ok(()) => {}
            Err(why) if why.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
            // A file system without hard links falls back to looking first.
            Err(_) if fs::symlink_metadata(target).is_ok() => return Ok(false),
            Err(_) => fs::rename(&self.path, target)?,
        }
        // Best effort: the name is in place whether or not the folder's record
        // of it reaches the disk before a power cut.
        if let Some(dir) = target.parent()
            && let Ok(handle) = File::open(dir)
        {
            let _ = handle.sync_all();
        }
        Ok(true)
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
