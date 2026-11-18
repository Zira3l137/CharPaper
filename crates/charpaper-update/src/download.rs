use std::fs;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use semver::Version;
use sha2::Digest;
use sha2::Sha256;
use ureq::Agent;
use zip::ZipArchive;

use crate::Archive;
use crate::Error;
use crate::at;
use crate::platform;
use crate::release::Asset;
use crate::release::Release;
use crate::release::agent;
use crate::release::checksum_for;
use crate::release_exe;
use crate::work_dir;

const CHUNK: usize = 64 * 1024;
pub(crate) const UNPACKED_DIR: &str = "new";

// A release downloaded, checked and unpacked, ready for install once the app has stopped.
#[derive(Debug, Clone)]
pub struct Staged {
    pub version: Version,
    pub(crate) work: PathBuf,
    pub(crate) dir: PathBuf,
}

// `progress` gets the bytes received so far and the archive's size, after every chunk.
pub fn download(
    release: &Release,
    app_dir: &Path,
    mut progress: impl FnMut(u64, u64),
) -> Result<Staged, Error> {
    let archive_kind = platform().ok_or(Error::Unsupported)?.archive;
    let work = work_dir(app_dir);
    // Whatever an earlier attempt left.
    if work.exists() {
        fs::remove_dir_all(&work).map_err(at(&work))?;
    }
    fs::create_dir_all(&work).map_err(at(&work))?;

    let agent = agent();
    let sums = agent.get(&release.sums.url).call()?.body_mut().read_to_string()?;
    let expected = checksum_for(&sums, &release.archive.name)
        .ok_or_else(|| Error::NoChecksum {
            release: release.tag.clone(),
            file: release.archive.name.clone(),
        })?
        .to_ascii_lowercase();

    let archive = work.join(&release.archive.name);
    let actual = fetch(&agent, &release.archive, &archive, &mut progress)?;
    if actual != expected {
        return Err(Error::ChecksumMismatch);
    }
    let dir = work.join(UNPACKED_DIR);
    match archive_kind {
        Archive::Zip => unpack_zip(&archive, &dir)?,
    }
    let _ = fs::remove_file(&archive);
    Ok(Staged { version: release.version.clone(), work, dir })
}

// Hashed as it is written, so the archive is read only once. Returns the SHA-256 in hex.
fn fetch(
    agent: &Agent,
    asset: &Asset,
    to: &Path,
    progress: &mut impl FnMut(u64, u64),
) -> Result<String, Error> {
    let mut response = agent.get(&asset.url).call()?;
    let total = response.body().content_length().unwrap_or(asset.size);
    let mut reader = response.body_mut().as_reader();
    let mut file = BufWriter::new(File::create(to).map_err(at(to))?);
    let mut hasher = Sha256::new();
    let mut chunk = vec![0; CHUNK];
    let mut done = 0;
    progress(0, total);
    loop {
        let read = match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => read,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(ureq::Error::from(err).into()),
        };
        hasher.update(&chunk[..read]);
        file.write_all(&chunk[..read]).map_err(at(to))?;
        done += read as u64;
        progress(done, total);
    }
    file.flush().map_err(at(to))?;
    Ok(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

// The release's zip holds one folder named after itself, so its contents go straight into
// `into`.
pub(crate) fn unpack_zip(archive: &Path, into: &Path) -> Result<(), Error> {
    let file = File::open(archive).map_err(at(archive))?;
    let mut zip = ZipArchive::new(BufReader::new(file))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        // None for a path that would land outside `into`, such as one with `..` in it.
        let path = entry.enclosed_name().ok_or_else(|| {
            Error::BadArchive(format!(
                "it holds a path outside its folder: {}",
                entry.name().unwrap_or_default()
            ))
        })?;
        let mut parts = path.components();
        parts.next();
        let inside = parts.as_path();
        if inside.as_os_str().is_empty() {
            continue;
        }
        let target = into.join(inside);
        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(at(&target))?;
        } else if entry.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(at(parent))?;
            }
            let mut out = File::create(&target).map_err(at(&target))?;
            io::copy(&mut entry, &mut out).map_err(at(&target))?;
            // Keeps the executable executable where the OS has a bit for it.
            #[cfg(unix)]
            if let Some(mode) = entry.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&target, fs::Permissions::from_mode(mode))
                    .map_err(at(&target))?;
            }
        }
    }
    let exe = release_exe();
    if !into.join(&exe).is_file() {
        return Err(Error::BadArchive(format!("it has no {exe}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;
    use crate::install::tests::scratch;

    fn zip_of(path: &Path, files: &[(&str, &str)]) {
        let mut zip = ZipWriter::new(File::create(path).unwrap());
        for (name, text) in files {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(text.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn unpacks_the_folder_inside() {
        let root = scratch("unpack");
        let archive = root.join("release.zip");
        let exe = format!("charpaper-v0.7.0-windows-x64/{}", release_exe());
        zip_of(
            &archive,
            &[
                (&exe, "new app"),
                ("charpaper-v0.7.0-windows-x64/locales/ru-RU/charpaper_ui.ftl", "ru"),
            ],
        );
        let into = root.join("new");
        unpack_zip(&archive, &into).unwrap();
        assert_eq!(fs::read_to_string(into.join(release_exe())).unwrap(), "new app");
        assert!(into.join("locales/ru-RU/charpaper_ui.ftl").is_file());
    }

    #[test]
    fn refuses_an_archive_without_the_app() {
        let root = scratch("no-app");
        let archive = root.join("release.zip");
        zip_of(&archive, &[("charpaper-v0.7.0-windows-x64/LICENSE-MIT", "mit")]);
        assert!(matches!(unpack_zip(&archive, &root.join("new")), Err(Error::BadArchive(_))));
    }
}
