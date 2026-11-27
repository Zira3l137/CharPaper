use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use crate::error::PackError;
use crate::format::ContentHash;
use crate::format::Entry;
use crate::format::HEADER_LEN;
use crate::format::Header;
use crate::format::HeaderProblem;
use crate::format::Method;
use crate::format::PackId;
use crate::format::decode_index;

/// A pack's index, kept in memory. The file itself is only open while something is read
/// from it: Windows won't let the add-on replace a file the app keeps open or mapped, and
/// Rust opens files sharing delete, so a pack can be swapped while a read is under way.
#[derive(Debug, Clone)]
pub struct Pack {
    path: PathBuf,
    id: PackId,
    entries: Vec<Entry>,
}

impl Pack {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PackError> {
        let path = path.as_ref().to_path_buf();
        let mut file = open(&path)?;
        let header = read_header(&mut file, &path)?;
        let damaged = |problem: String| PackError::Damaged { path: path.clone(), problem };

        let file_len = file.metadata().map_err(|source| read_error(&path, source))?.len();
        let index_end = header.index_offset.checked_add(header.index_len);
        if header.index_offset < HEADER_LEN || index_end != Some(file_len) {
            return Err(damaged("its index isn't where the header says".into()));
        }
        // The length matches the file's, so this can't ask for more memory than the file has.
        let mut index = vec![0; header.index_len as usize];
        file.seek(SeekFrom::Start(header.index_offset))
            .and_then(|_| file.read_exact(&mut index))
            .map_err(|source| read_error(&path, source))?;
        if PackId::of_index(&index) != header.id {
            return Err(damaged("its index doesn't match its id".into()));
        }
        let entries = decode_index(&index, &header).map_err(damaged)?;
        Ok(Self { path, id: header.id, entries })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn id(&self) -> PackId {
        self.id
    }

    /// Sorted by path.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn entry(&self, path: &str) -> Option<&Entry> {
        let at = self.entries.binary_search_by(|e| e.path.as_str().cmp(path)).ok()?;
        Some(&self.entries[at])
    }

    pub fn is_file(&self, path: &str) -> bool {
        self.entry(path).is_some()
    }

    /// Folders aren't stored: one exists while a file is under it. `""` is the suite folder.
    pub fn is_dir(&self, dir: &str) -> bool {
        dir.is_empty() || !self.under(dir).is_empty()
    }

    /// The files right in `dir`, not in its sub-folders, sorted.
    pub fn files_in<'a>(&'a self, dir: &str) -> impl Iterator<Item = &'a Entry> + 'a {
        let skip = prefix_len(dir);
        self.under(dir).iter().filter(move |e| !e.path[skip..].contains('/'))
    }

    /// The names of the folders right in `dir`, sorted.
    pub fn folders_in<'a>(&'a self, dir: &str) -> Vec<&'a str> {
        let skip = prefix_len(dir);
        let mut names: Vec<&str> = self
            .under(dir)
            .iter()
            .filter_map(|e| e.path[skip..].split_once('/').map(|(folder, _)| folder))
            .collect();
        // Sorted paths keep each folder's files together, so its repeats are neighbours.
        names.dedup();
        names
    }

    // Everything below `dir`: sorted paths that share a prefix form one run.
    fn under(&self, dir: &str) -> &[Entry] {
        if dir.is_empty() {
            return &self.entries;
        }
        let prefix = format!("{dir}/");
        let start = self.entries.partition_point(|e| e.path < prefix);
        let len = self.entries[start..].partition_point(|e| e.path.starts_with(&prefix));
        &self.entries[start..start + len]
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>, PackError> {
        let entry = self.find(path)?;
        self.file()?.read(entry)
    }

    /// Streams a file out, for reading only its start: a .glb's JSON without its meshes.
    /// A damaged file shows as an `io::Error` from the reader.
    pub fn reader(&self, path: &str) -> Result<Box<dyn Read + Send>, PackError> {
        let entry = self.find(path)?;
        let PackFile { mut file, .. } = self.file()?;
        file.seek(SeekFrom::Start(entry.offset)).map_err(|e| read_error(&self.path, e))?;
        let stored = file.take(entry.stored);
        Ok(match entry.method {
            Method::Stored => Box::new(stored),
            Method::Zstd => Box::new(
                zstd::stream::read::Decoder::new(stored).map_err(|e| read_error(&self.path, e))?,
            ),
        })
    }

    fn find(&self, path: &str) -> Result<&Entry, PackError> {
        self.entry(path)
            .ok_or_else(|| PackError::Missing { pack: self.path.clone(), file: path.into() })
    }

    /// Opens the file for several reads, after checking it's still this pack.
    pub fn file(&self) -> Result<PackFile<'_>, PackError> {
        let mut file = open(&self.path)?;
        if read_header(&mut file, &self.path)?.id != self.id {
            return Err(PackError::Replaced(self.path.clone()));
        }
        Ok(PackFile { pack: self, file })
    }

    /// Reads every file and checks it against its hash.
    pub fn verify(&self) -> Result<(), PackError> {
        let mut file = self.file()?;
        for entry in &self.entries {
            let bytes = file.read(entry)?;
            if ContentHash::of(&bytes) != entry.hash {
                return Err(self.damaged(format!("{:?} doesn't match its hash", entry.path)));
            }
        }
        Ok(())
    }

    fn damaged(&self, problem: String) -> PackError {
        PackError::Damaged { path: self.path.clone(), problem }
    }
}

pub struct PackFile<'a> {
    pack: &'a Pack,
    file: File,
}

impl PackFile<'_> {
    /// The file's contents, unpacked. Zstd checks its own checksum; stored files are only
    /// checked by `Pack::verify`.
    pub fn read(&mut self, entry: &Entry) -> Result<Vec<u8>, PackError> {
        let stored = self.read_stored(entry)?;
        match entry.method {
            Method::Stored => Ok(stored),
            Method::Zstd => {
                let damaged = || self.pack.damaged(format!("{:?} doesn't unpack", entry.path));
                let size = usize::try_from(entry.size).map_err(|_| damaged())?;
                let bytes = zstd::bulk::decompress(&stored, size).map_err(|_| damaged())?;
                if bytes.len() != size {
                    return Err(damaged());
                }
                Ok(bytes)
            }
        }
    }

    /// The bytes as they are in the pack, still compressed if they were.
    pub fn read_stored(&mut self, entry: &Entry) -> Result<Vec<u8>, PackError> {
        let path = &self.pack.path;
        let mut bytes = vec![0; entry.stored as usize];
        self.file
            .seek(SeekFrom::Start(entry.offset))
            .and_then(|_| self.file.read_exact(&mut bytes))
            .map_err(|source| read_error(path, source))?;
        Ok(bytes)
    }
}

fn open(path: &Path) -> Result<File, PackError> {
    File::open(path).map_err(|source| read_error(path, source))
}

fn read_header(file: &mut File, path: &Path) -> Result<Header, PackError> {
    let mut bytes = [0; HEADER_LEN as usize];
    match file.read_exact(&mut bytes) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(PackError::NotAPack(path.into()));
        }
        Err(source) => return Err(read_error(path, source)),
    }
    Header::decode(&bytes).map_err(|problem| match problem {
        HeaderProblem::NotAPack => PackError::NotAPack(path.into()),
        HeaderProblem::Version(found) => PackError::Version { path: path.into(), found },
    })
}

fn prefix_len(dir: &str) -> usize {
    if dir.is_empty() { 0 } else { dir.len() + 1 }
}

fn read_error(path: &Path, source: std::io::Error) -> PackError {
    PackError::Read { path: path.into(), source }
}

/// A pack that follows its file: when the add-on replaces it, the next read opens the new
/// one instead of failing. What was listed from the old one may be gone from the new one.
#[derive(Debug)]
pub struct FollowedPack {
    path: PathBuf,
    current: RwLock<Arc<Pack>>,
}

impl FollowedPack {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PackError> {
        let pack = Pack::open(path)?;
        Ok(Self { path: pack.path.clone(), current: RwLock::new(Arc::new(pack)) })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The pack as it was last opened.
    pub fn current(&self) -> Arc<Pack> {
        self.current.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>, PackError> {
        self.follow(|pack| pack.read(path))
    }

    pub fn reader(&self, path: &str) -> Result<Box<dyn Read + Send>, PackError> {
        self.follow(|pack| pack.reader(path))
    }

    fn follow<T>(&self, read: impl Fn(&Pack) -> Result<T, PackError>) -> Result<T, PackError> {
        match read(&self.current()) {
            Err(PackError::Replaced(_)) => {
                let new = Arc::new(Pack::open(&self.path)?);
                *self.current.write().unwrap_or_else(|e| e.into_inner()) = new.clone();
                read(&new)
            }
            result => result,
        }
    }
}
