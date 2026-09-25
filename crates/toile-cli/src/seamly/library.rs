use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use toile_doc::{PERSONA_EXTENSION, Persona};

/// The file `stem` names in the library kept in `dir`.
pub fn path(dir: &Path, stem: &str) -> PathBuf {
    dir.join(format!("{stem}.{PERSONA_EXTENSION}"))
}

/// Why a stem already taken is not written over.
pub fn taken(dir: &Path, stem: &str) -> String {
    format!(
        "ya hay una persona en la biblioteca con el archivo «{}»: no se sobrescribe; añade \
         --vincular para vincular el producto a ella, o elige otro nombre con --persona",
        path(dir, stem).display()
    )
}

/// Why a stem that names nothing cannot be linked to.
pub fn missing(dir: &Path, stem: &str) -> String {
    format!(
        "no hay ninguna persona en la biblioteca con el archivo «{}»: quita --vincular y se archiva \
         con las medidas del patrón",
        path(dir, stem).display()
    )
}

/// The person `stem` names in the library kept in `dir`.
///
/// Read whole and parsed by the very reader the app uses, so a file the app
/// would refuse is refused here too instead of half-read into a link.
///
/// Consulting the library tidies it, as showing it does in the app: this is
/// the one path a `--vincular` run takes, and it would otherwise leave a copy
/// a killed run staged for the app to find.
pub fn read(dir: &Path, stem: &str) -> Result<Persona, String> {
    sweep(dir);
    let target = path(dir, stem);
    let text = fs::read_to_string(&target)
        .map_err(|why| format!("no se pudo leer «{}»: {why}", target.display()))?;
    Persona::from_json(&text)
        .map_err(|why| format!("«{}» no es una persona legible: {why}", target.display()))
}

/// Files `text` in the library kept in `dir` as the person under `stem`, whole
/// or not at all, and returns where.
///
/// The app numbers a taken stem instead; from the command line a taken stem
/// most likely means the same person imported twice, and a second file would
/// split her history in two. The temporary file is named the way the app names
/// its own, and the folder is tidied before anything else happens here — ahead
/// of the refusal, because the run that leaves a copy behind is most often the
/// one whose name is already taken, and it would keep meeting its own leavings
/// on every retry.
pub fn file(dir: &Path, stem: &str, text: &str) -> Result<PathBuf, String> {
    sweep(dir);
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

/// Removes what a run left behind when it was killed between writing its
/// temporary file and claiming the person's name.
///
/// Drop cannot cover that window, because the process is gone. What it leaves
/// is a whole copy of a person, notes and all, under a hidden name no listing
/// shows, so nobody would find it to delete it. Only names [`Staged::write`]
/// makes are touched, and only once the process that made one is gone: a run
/// still filing somewhere else keeps its file.
fn sweep(dir: &Path) {
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

/// Without a way to ask after a process, every file is some live run's.
#[cfg(not(unix))]
fn alive(_pid: u32) -> bool {
    true
}

// Only a platform that can ask after a process sweeps anything, so only one
// has a sweep to prove.
#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// A scratch library folder nobody else has, removed with the test.
    struct Scratch {
        dir: PathBuf,
    }

    impl Scratch {
        fn new(test: &str) -> Scratch {
            let name = format!("toile-cli-library-{}-{test}", std::process::id());
            let dir = std::env::temp_dir().join(name).join("personas");
            let _ = fs::remove_dir_all(dir.parent().expect("the scratch root"));
            fs::create_dir_all(&dir).expect("a scratch folder");
            Scratch { dir }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            if self.dir.starts_with(std::env::temp_dir()) {
                let _ = fs::remove_dir_all(self.dir.parent().expect("the scratch root"));
            }
        }
    }

    /// The id of a process that has certainly ended.
    fn dead_pid() -> u32 {
        let mut child = std::process::Command::new("true")
            .spawn()
            .expect("a process to outlive");
        let pid = child.id();
        child.wait().expect("it ends");
        pid
    }

    /// Filing a person removes the copy a killed run staged, and leaves alone
    /// both a run still going and any name this protocol does not make.
    #[cfg(unix)]
    #[test]
    fn filing_removes_the_copy_a_dead_run_staged_and_nothing_else() {
        let scratch = Scratch::new("sweep");
        let dir = scratch.dir.as_path();
        let orphan = dir.join(format!(".etienne.{}.0.tmp", dead_pid()));
        let running = dir.join(format!(".etienne.{}.9.tmp", std::process::id()));
        let neither = dir.join(".etienne.notas");
        for at in [&orphan, &running, &neither] {
            fs::write(at, "Etienne, notas incluidas").expect("a hand-made file");
        }

        let filed = file(dir, "etienne", "{}").expect("she is filed");

        assert!(!orphan.exists(), "the copy the dead run left is gone");
        assert!(running.exists(), "a run still filing keeps its file");
        assert!(neither.exists(), "and a name no run makes is not ours");
        let back = fs::read_to_string(&filed).expect("she is on disk");
        assert_eq!(back, "{}", "and she is the file this run wrote");
    }

    /// The case a real kill produces: the run got as far as the person's name
    /// before it died, so every retry is refused — and the retry is the only
    /// thing that would ever come back to tidy up after it.
    #[cfg(unix)]
    #[test]
    fn a_stem_already_taken_is_still_swept_before_it_is_refused() {
        let scratch = Scratch::new("taken");
        let dir = scratch.dir.as_path();
        let orphan = dir.join(format!(".etienne.{}.0.tmp", dead_pid()));
        fs::write(&orphan, "Etienne, notas incluidas").expect("a hand-made file");
        fs::write(path(dir, "etienne"), "{}").expect("and her name is taken");

        let why = file(dir, "etienne", "{}").expect_err("a taken stem is refused");

        assert!(why.contains("no se sobrescribe"), "refused for that: {why}");
        assert!(!orphan.exists(), "and swept on the way to the refusal");
    }

    /// Linking to a person tidies the folder too, which is the only path a
    /// `--vincular` run walks.
    #[cfg(unix)]
    #[test]
    fn linking_to_a_person_sweeps_what_a_dead_run_left() {
        let scratch = Scratch::new("link");
        let dir = scratch.dir.as_path();
        let orphan = dir.join(format!(".etienne.{}.0.tmp", dead_pid()));
        fs::write(&orphan, "Etienne, notas incluidas").expect("a hand-made file");
        fs::write(path(dir, "etienne"), "no es una persona").expect("an unreadable file");

        let why = read(dir, "etienne").expect_err("an unreadable person is refused");

        assert!(why.contains("no es una persona legible"), "refused: {why}");
        assert!(!orphan.exists(), "and swept before it was read");
    }
}
