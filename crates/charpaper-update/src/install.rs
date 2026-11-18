use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use tracing::warn;

use crate::CHARACTERS_DIR;
use crate::Error;
use crate::Staged;
use crate::at;
use crate::release_exe;
use crate::work_dir;

const KEPT_DIR: &str = "old";
// Not under the work folder, which tidy removes.
const BACKUP_PREFIX: &str = "charpaper-backup-";

// Swaps the staged release in, entry by entry, beside `exe`: whatever has the same name is
// moved into the work folder and the release's entry takes its place, so a folder like
// locales/ is replaced whole and keeps nothing the release dropped. state.toml, the logs and
// anything else the release doesn't ship stay as they are; characters/ is only added to.
//
// Windows won't overwrite or delete a running executable but will rename it, which is why
// everything is moved rather than copied over: this works from the run being replaced, once
// Bevy has stopped. If a move fails, those done so far are undone.
pub fn install(staged: &Staged, exe: &Path) -> Result<(), Error> {
    let app_dir = exe.parent().ok_or_else(|| Error::File {
        path: exe.to_path_buf(),
        source: io::Error::other("the executable has no parent folder"),
    })?;
    let kept_dir = staged.work.join(KEPT_DIR);
    fs::create_dir_all(&kept_dir).map_err(at(&kept_dir))?;

    let mut entries = Vec::new();
    for entry in fs::read_dir(&staged.dir).map_err(at(&staged.dir))? {
        entries.push(entry.map_err(at(&staged.dir))?.file_name());
    }
    entries.sort();

    let mut done = Vec::new();
    for name in entries.iter().filter(|name| *name != CHARACTERS_DIR) {
        let from = staged.dir.join(name);
        let to = if *name == *release_exe() { exe.to_path_buf() } else { app_dir.join(name) };
        let kept = if fs::symlink_metadata(&to).is_ok() {
            let kept = kept_dir.join(name);
            if let Err(err) = fs::rename(&to, &kept) {
                return Err(roll_back(done, at(&to)(err), &kept_dir, app_dir));
            }
            Some(kept)
        } else {
            None
        };
        let placed = fs::rename(&from, &to);
        done.push(Swap { to: to.clone(), kept, placed: placed.is_ok() });
        if let Err(err) = placed {
            return Err(roll_back(done, at(&to)(err), &kept_dir, app_dir));
        }
    }

    let characters = staged.dir.join(CHARACTERS_DIR);
    if characters.is_dir() {
        let to = app_dir.join(CHARACTERS_DIR);
        if let Err(err) = add_missing(&characters, &to) {
            warn!("cannot add the release's characters to {}: {err}", to.display());
        }
    }
    Ok(())
}

struct Swap {
    to: PathBuf,
    kept: Option<PathBuf>,
    placed: bool,
}

// Should something not go back, everything replaced is moved out of the work folder, so the
// next start's tidy can't delete the user's only copy.
fn roll_back(done: Vec<Swap>, cause: Error, kept_dir: &Path, app_dir: &Path) -> Error {
    let mut stuck = false;
    for swap in done.into_iter().rev() {
        if swap.placed
            && let Err(err) = remove(&swap.to)
        {
            warn!("cannot take {} back out: {err}", swap.to.display());
            stuck = true;
            continue;
        }
        if let Some(kept) = swap.kept
            && let Err(err) = fs::rename(&kept, &swap.to)
        {
            warn!("cannot put {} back: {err}", swap.to.display());
            stuck = true;
        }
    }
    if !stuck {
        return cause;
    }
    let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let backup = app_dir.join(format!("{BACKUP_PREFIX}{}", stamp.as_secs()));
    let kept = if fs::rename(kept_dir, &backup).is_ok() { backup } else { kept_dir.to_path_buf() };
    Error::Stranded { cause: Box::new(cause), kept }
}

fn remove(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn add_missing(from: &Path, to: &Path) -> io::Result<()> {
    if fs::symlink_metadata(to).is_err() {
        return fs::rename(from, to);
    }
    if !(from.is_dir() && to.is_dir()) {
        return Ok(());
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        add_missing(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

// Removes what the last update left: the old executable and folders it replaced, or a
// download that was never installed. Blocks for up to a few seconds, because the run that
// installed this one may still be ending, and its executable can't be deleted until it has.
pub fn tidy(app_dir: &Path) -> Result<(), Error> {
    const TRIES: u32 = 10;
    let work = work_dir(app_dir);
    for attempt in 1..=TRIES {
        match fs::remove_dir_all(&work) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(err) if attempt == TRIES => return Err(at(&work)(err)),
            Err(_) => thread::sleep(Duration::from_millis(500)),
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::download::UNPACKED_DIR;

    pub(crate) fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("charpaper-update-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    // An app renamed by its user, and a release beside it in the work folder.
    fn setup(name: &str) -> (PathBuf, PathBuf, Staged) {
        let app = scratch(name);
        let exe = app.join("wallpaper.exe");
        write(&exe, "old app");
        write(&app.join("locales/en-US/charpaper_ui.ftl"), "en");
        write(&app.join("state.toml"), "state");
        write(&app.join("characters/mine/suite.toml"), "mine");

        let work = work_dir(&app);
        let dir = work.join(UNPACKED_DIR);
        write(&dir.join(release_exe()), "new app");
        write(&dir.join("locales/ru-RU/charpaper_ui.ftl"), "ru");
        write(&dir.join("LICENSE-MIT"), "mit");
        write(&dir.join("characters/mine/suite.toml"), "the release's");
        write(&dir.join("characters/sample/suite.toml"), "sample");
        let staged = Staged { version: semver::Version::new(0, 7, 0), work, dir };
        (app, exe, staged)
    }

    #[test]
    fn replaces_what_the_release_ships_and_keeps_the_rest() {
        let (app, exe, staged) = setup("install");
        install(&staged, &exe).unwrap();

        assert_eq!(read(&exe), "new app");
        assert!(app.join("locales/ru-RU/charpaper_ui.ftl").is_file());
        assert!(!app.join("locales/en-US").exists());
        assert_eq!(read(&app.join("LICENSE-MIT")), "mit");
        assert_eq!(read(&app.join("state.toml")), "state");
        assert_eq!(read(&app.join("characters/mine/suite.toml")), "mine");
        assert_eq!(read(&app.join("characters/sample/suite.toml")), "sample");
        assert_eq!(read(&staged.work.join(KEPT_DIR).join(release_exe())), "old app");

        tidy(&app).unwrap();
        assert!(!staged.work.exists());
    }

    #[test]
    fn a_failed_move_puts_everything_back() {
        let (app, exe, staged) = setup("roll-back");
        // Entries go in name order, so the executable and the license are already in place
        // when locales/ can't be moved aside onto a folder with something in it.
        write(&staged.work.join(KEPT_DIR).join("locales/blocker"), "");
        assert!(install(&staged, &exe).is_err());

        assert_eq!(read(&exe), "old app");
        assert!(!app.join("LICENSE-MIT").exists());
        assert!(app.join("locales/en-US/charpaper_ui.ftl").is_file());
        assert!(!app.join("characters/sample").exists());
    }
}
