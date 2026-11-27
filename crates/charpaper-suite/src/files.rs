use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use charpaper_pack::FollowedPack;
use charpaper_pack::PACK_FILE;
use charpaper_pack::pack_path;

use crate::error::SuiteError;

/// Where a suite's files are: in the suite.chr beside its suite.toml, or else loose in its
/// folder. Paths are relative to the suite folder either way.
#[derive(Debug, Clone)]
pub struct SuiteFiles {
    root: PathBuf,
    pack: Option<Arc<FollowedPack>>,
}

impl SuiteFiles {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, SuiteError> {
        let root = root.as_ref().to_path_buf();
        let path = root.join(PACK_FILE);
        let pack = match path.is_file() {
            true => Some(Arc::new(FollowedPack::open(&path)?)),
            false => None,
        };
        Ok(Self { root, pack })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn pack(&self) -> Option<&FollowedPack> {
        self.pack.as_deref()
    }

    pub fn is_packed(&self) -> bool {
        self.pack.is_some()
    }

    pub fn is_file(&self, relative: &Path) -> bool {
        match &self.pack {
            Some(pack) => key(relative).is_some_and(|key| pack.current().is_file(&key)),
            None => self.root.join(relative).is_file(),
        }
    }

    /// The files right in `dir`, as paths relative to the suite folder, sorted. A folder
    /// that isn't there is empty: every sub-folder of a suite is optional.
    pub fn files_in(&self, dir: &Path) -> Result<Vec<PathBuf>, SuiteError> {
        let names: Vec<String> = match &self.pack {
            Some(pack) => {
                let Some(key) = key(dir) else {
                    return Ok(Vec::new());
                };
                let pack = pack.current();
                pack.files_in(&key).map(|e| name_of(&e.path).to_string()).collect()
            }
            None => {
                let entries = disk_entries(&self.root.join(dir))?;
                entries.into_iter().filter(|(_, is_dir)| !is_dir).map(|(name, _)| name).collect()
            }
        };
        let mut files: Vec<PathBuf> = names.into_iter().map(|name| dir.join(name)).collect();
        files.sort();
        Ok(files)
    }

    /// The names of the folders right in `dir`, sorted.
    pub fn folders_in(&self, dir: &Path) -> Result<Vec<String>, SuiteError> {
        let mut names: Vec<String> = match &self.pack {
            Some(pack) => match key(dir) {
                Some(key) => {
                    pack.current().folders_in(&key).into_iter().map(String::from).collect()
                }
                None => Vec::new(),
            },
            None => {
                let entries = disk_entries(&self.root.join(dir))?;
                entries.into_iter().filter(|(_, is_dir)| *is_dir).map(|(name, _)| name).collect()
            }
        };
        names.sort();
        Ok(names)
    }

    pub fn read(&self, relative: &Path) -> Result<Vec<u8>, SuiteError> {
        match &self.pack {
            Some(pack) => Ok(pack.read(&pack_key(relative)?)?),
            None => {
                let path = self.root.join(relative);
                fs::read(&path).map_err(|source| SuiteError::Io { path, source })
            }
        }
    }

    pub fn reader(&self, relative: &Path) -> Result<Box<dyn Read + Send>, SuiteError> {
        match &self.pack {
            Some(pack) => Ok(pack.reader(&pack_key(relative)?)?),
            None => {
                let path = self.root.join(relative);
                let file =
                    fs::File::open(&path).map_err(|source| SuiteError::Io { path, source })?;
                Ok(Box::new(io::BufReader::new(file)))
            }
        }
    }

    /// Changes when the file does: its content hash in a pack, its size and time on disk.
    pub fn stamp(&self, relative: &Path) -> Option<String> {
        match &self.pack {
            Some(pack) => Some(pack.current().entry(&key(relative)?)?.hash.to_string()),
            None => {
                let meta = fs::metadata(self.root.join(relative)).ok()?;
                let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
                Some(format!("{}.{modified}", meta.len()))
            }
        }
    }
}

// `""` is the suite folder itself; a path that can't be in a pack is in no folder of it.
fn key(relative: &Path) -> Option<String> {
    if relative.as_os_str().is_empty() {
        return Some(String::new());
    }
    pack_path(relative).ok()
}

fn pack_key(relative: &Path) -> Result<String, SuiteError> {
    pack_path(relative).map_err(|_| SuiteError::UnsafePath(relative.to_path_buf()))
}

fn name_of(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

fn disk_entries(dir: &Path) -> Result<Vec<(String, bool)>, SuiteError> {
    let io_error = |source| SuiteError::Io { path: dir.to_path_buf(), source };
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io_error(err)),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            found.push((name, true));
        } else if path.is_file() {
            found.push((name, false));
        }
    }
    Ok(found)
}
