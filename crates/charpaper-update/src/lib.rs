// Updates the app from its GitHub releases: finds a newer stable version, downloads this
// platform's archive and checks it against the release's SHA256SUMS.txt, unpacks it beside
// the executable, and swaps it in once the app has stopped. Blocking throughout; the app
// runs each step on a thread of its own.

mod download;
mod install;
mod notes;
mod release;

use std::io;
use std::path::Path;
use std::path::PathBuf;

use thiserror::Error;

pub use crate::download::Staged;
pub use crate::download::download;
pub use crate::install::install;
pub use crate::install::tidy;
pub use crate::notes::Notes;
pub use crate::notes::Section;
pub use crate::release::Release;
pub use crate::release::check;
// For comparing versions without a semver dependency of one's own.
pub use semver::Version;

pub const REPOSITORY: &str = "Zira3l137/CharPaper";

// Everything an update keeps between steps, next to the executable: the download, the
// unpacked release, and what it replaced. Removed by tidy on the next start.
const WORK_DIR: &str = ".update";
// The release's own copy is only added to, so a suite shipped with the app never replaces
// one the user has.
const CHARACTERS_DIR: &str = "characters";

struct Platform {
    os: &'static str,
    arch: &'static str,
    // As build.yml's matrix names it, which ends each of its archives' names.
    name: &'static str,
    archive: Archive,
}

// Every platform releases carry a build for. A new one also needs its entry in build.yml.
const PLATFORMS: [Platform; 1] =
    [Platform { os: "windows", arch: "x86_64", name: "windows-x64", archive: Archive::Zip }];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Archive {
    Zip,
}

impl Archive {
    fn extension(self) -> &'static str {
        match self {
            Archive::Zip => "zip",
        }
    }
}

fn platform() -> Option<&'static Platform> {
    use std::env::consts::ARCH;
    use std::env::consts::OS;
    PLATFORMS.iter().find(|platform| platform.os == OS && platform.arch == ARCH)
}

// Whether releases carry a build for this platform at all.
pub fn supported() -> bool {
    platform().is_some()
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("releases carry no build for this platform")]
    Unsupported,
    #[error("cannot reach GitHub: {0}")]
    Http(#[from] ureq::Error),
    #[error("GitHub's answer can't be read: {0}")]
    Json(#[from] serde_json::Error),
    #[error("release {0} carries no build for this platform")]
    NoArchive(String),
    #[error("release {release} has no checksum for {file}")]
    NoChecksum { release: String, file: String },
    #[error("the download is damaged: its checksum doesn't match the release's")]
    ChecksumMismatch,
    #[error("the download can't be unpacked: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("the download is not a CharPaper release: {0}")]
    BadArchive(String),
    #[error("{}: {source}", path.display())]
    File { path: PathBuf, source: io::Error },
    // What it replaced could not all be put back, so it was kept aside rather than lost.
    #[error("the update failed ({cause}), and putting the old files back failed too; they are in {}", kept.display())]
    Stranded { cause: Box<Error>, kept: PathBuf },
}

fn at(path: &Path) -> impl FnOnce(io::Error) -> Error + '_ {
    move |source| Error::File { path: path.to_path_buf(), source }
}

fn work_dir(app_dir: &Path) -> PathBuf {
    app_dir.join(WORK_DIR)
}

// The executable as the release names it, which may differ from the running one's name if
// the user renamed it.
fn release_exe() -> String {
    format!("charpaper{}", std::env::consts::EXE_SUFFIX)
}
