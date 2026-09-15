use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Counts the temporary files this process has made, so two saves never share
/// one.
static MADE: AtomicU64 = AtomicU64::new(0);

/// A complete file, flushed to disk beside where it is going, under a hidden
/// name ending `.tmp` that no listing takes for a person.
///
/// Dropped, it removes that name and only that name. After a rename the name
/// is already gone; after a claim the file lives on under the person's name;
/// after a failure the folder is left as it was found.
pub(super) struct Staged {
    path: PathBuf,
}

impl Staged {
    /// Writes `text` to a new file in `dir` and flushes it, so what is renamed
    /// onto a person is never a file still being written.
    pub(super) fn write(dir: &Path, stem: &str, text: &str) -> io::Result<Staged> {
        let (mut file, staged) = loop {
            let n = MADE.fetch_add(1, Ordering::Relaxed);
            let path = dir.join(format!(".{stem}.{}.{n}.tmp", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => break (file, Staged { path }),
                // Left by an earlier process with this id that died mid-save.
                Err(why) if why.kind() == io::ErrorKind::AlreadyExists => {}
                Err(why) => return Err(why),
            }
        };
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        Ok(staged)
    }

    /// Puts the file at `target` in place of the one there, in one rename: a
    /// reader finds the old person or the new, never part of either.
    pub(super) fn replace(self, target: &Path) -> io::Result<()> {
        fs::rename(&self.path, target)?;
        sync_parent(target);
        Ok(())
    }

    /// Puts the file at `target` only if that name is free, and says whether
    /// it did. A hard link cannot land on a name that exists, so two saves
    /// racing for one stem cannot both win it.
    pub(super) fn claim(&self, target: &Path) -> io::Result<bool> {
        match fs::hard_link(&self.path, target) {
            Ok(()) => {}
            Err(why) if why.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
            // A file system without hard links, such as FAT or some network
            // shares, falls back to looking before renaming. The gap that
            // leaves is two Toiles creating one stem in the same instant.
            Err(_) if fs::symlink_metadata(target).is_ok() => return Ok(false),
            Err(_) => fs::rename(&self.path, target)?,
        }
        sync_parent(target);
        Ok(true)
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Flushes the folder's record of a file just put in place, so the new name
/// survives a power cut as well as the contents do.
///
/// Best effort: the name is already in place, and a folder that cannot be
/// opened for a flush holds the person all the same.
fn sync_parent(target: &Path) {
    if let Some(dir) = target.parent()
        && let Ok(handle) = File::open(dir)
    {
        let _ = handle.sync_all();
    }
}

/// Removes what a save left behind when its process died between writing the
/// temporary file and putting it in place.
///
/// Such a file is a whole copy of a person, notes and all, under a hidden name
/// no listing shows, so deleting the person's own file would not delete the
/// person. Only names [`Staged::write`] makes are touched, and only once the
/// process that made one is gone: a save still running in another Toile keeps
/// its file.
pub(super) fn sweep(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if let Some(pid) = maker(&entry.file_name().to_string_lossy())
            && pid != std::process::id()
            && !alive(pid)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// The process that made a temporary name, `.{stem}.{pid}.{n}.tmp`, or `None`
/// for any name this module does not make.
fn maker(name: &str) -> Option<u32> {
    let inner = name.strip_prefix('.')?.strip_suffix(".tmp")?;
    let mut parts = inner.rsplitn(3, '.');
    parts.next()?.parse::<u64>().ok()?;
    let pid = parts.next()?.parse::<u32>().ok()?;
    parts.next().filter(|stem| !stem.is_empty())?;
    Some(pid)
}

#[cfg(unix)]
fn alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return true;
    };
    // SAFETY: signal 0 delivers nothing; `kill` only reports whether the
    // process exists and may be signalled.
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    // Another user's process answers EPERM: it exists, so its file stays.
    std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

/// Without a way to ask after a process, every file is some live save's.
#[cfg(not(unix))]
fn alive(_pid: u32) -> bool {
    true
}
