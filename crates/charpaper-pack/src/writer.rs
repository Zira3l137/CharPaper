use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::btree_map;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use zstd::bulk::Compressor;
use zstd::zstd_safe::CParameter;

use crate::error::PackError;
use crate::format::ALIGN;
use crate::format::ContentHash;
use crate::format::Entry;
use crate::format::HEADER_LEN;
use crate::format::Header;
use crate::format::Method;
use crate::format::PackId;
use crate::format::encode_index;
use crate::path::check_path;
use crate::path::pack_path;
use crate::reader::Pack;

/// On mesh data it squeezes about as well as 9 at twice the speed: over 100 MB a second a
/// core.
pub const DEFAULT_LEVEL: i32 = 6;

// Already compressed: trying zstd on them only costs time.
const INCOMPRESSIBLE: &[&str] = &["ogg", "gif", "png", "jpg", "jpeg", "webp"];
// Smaller ones gain too little for zstd's frame header to pay.
const SMALLEST_TO_COMPRESS: usize = 64;

enum Source {
    Disk(PathBuf),
    Bytes(Vec<u8>),
    Kept(Entry),
}

/// Collects files, then writes them as one pack in path order, so the same files always
/// make the same pack.
pub struct PackWriter {
    files: BTreeMap<String, Source>,
    previous: Option<Pack>,
    level: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Progress<'a> {
    /// The file being packed now, or `None` once all are.
    pub path: Option<&'a str>,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub files: usize,
    /// The files' bytes, unpacked.
    pub size: u64,
    /// The pack's bytes.
    pub packed: u64,
    /// Files copied from the previous pack as they were, not compressed again.
    pub reused: usize,
    /// Files whose contents another file in the pack already has, sharing its data.
    pub shared: usize,
}

impl Default for PackWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl PackWriter {
    pub fn new() -> Self {
        Self { files: BTreeMap::new(), previous: None, level: DEFAULT_LEVEL }
    }

    /// Zstd's level, 1 to 22.
    pub fn level(mut self, level: i32) -> Self {
        self.level = level.clamp(1, 22);
        self
    }

    /// Files whose contents are already in `previous` are copied from it as they are there,
    /// rather than compressed again: an export that changed one skin repacks one skin.
    pub fn reuse(&mut self, previous: Pack) {
        self.previous = Some(previous);
    }

    /// Carries over the files of the pack given to `reuse` that `keep` picks, unless the
    /// same path is added. For an export of some parts of a suite only.
    pub fn keep(&mut self, keep: impl Fn(&str) -> bool) {
        let Some(previous) = &self.previous else {
            return;
        };
        for entry in previous.entries().iter().filter(|e| keep(&e.path)) {
            if let btree_map::Entry::Vacant(slot) = self.files.entry(entry.path.clone()) {
                slot.insert(Source::Kept(entry.clone()));
            }
        }
    }

    /// The file on disk is read when the pack is written, not now.
    pub fn add_file(&mut self, path: &str, from: impl Into<PathBuf>) -> Result<(), PackError> {
        self.add(path, Source::Disk(from.into()))
    }

    pub fn add_bytes(&mut self, path: &str, bytes: Vec<u8>) -> Result<(), PackError> {
        self.add(path, Source::Bytes(bytes))
    }

    /// Every file under `folder` that `keep` picks by its pack path.
    pub fn add_folder(
        &mut self,
        folder: impl AsRef<Path>,
        keep: impl Fn(&str) -> bool,
    ) -> Result<(), PackError> {
        let folder = folder.as_ref();
        let mut pending = vec![folder.to_path_buf()];
        while let Some(dir) = pending.pop() {
            let read_error = |source| PackError::Read { path: dir.clone(), source };
            for item in fs::read_dir(&dir).map_err(read_error)? {
                let item = item.map_err(read_error)?;
                let path = item.path();
                if item.file_type().map_err(read_error)?.is_dir() {
                    pending.push(path);
                    continue;
                }
                let relative = path.strip_prefix(folder).unwrap_or(&path);
                let name = pack_path(relative)?;
                if keep(&name) {
                    self.add_file(&name, path)?;
                }
            }
        }
        Ok(())
    }

    fn add(&mut self, path: &str, source: Source) -> Result<(), PackError> {
        check_path(path)?;
        match self.files.entry(path.to_string()) {
            btree_map::Entry::Vacant(slot) => {
                slot.insert(source);
            }
            btree_map::Entry::Occupied(mut slot) if matches!(slot.get(), Source::Kept(_)) => {
                slot.insert(source);
            }
            btree_map::Entry::Occupied(_) => return Err(PackError::Duplicate(path.into())),
        }
        Ok(())
    }

    /// Writes beside `out` first and swaps it in at the end, so a failed or killed export
    /// leaves the old pack as it was.
    pub fn write(
        self,
        out: impl AsRef<Path>,
        mut progress: impl FnMut(Progress),
    ) -> Result<Summary, PackError> {
        let out = out.as_ref();
        check_case(&self.files)?;
        let mut partial = out.as_os_str().to_owned();
        partial.push(".part");
        let partial = PathBuf::from(partial);
        match self.write_to(&partial, &mut progress) {
            Ok(summary) => {
                fs::rename(&partial, out)
                    .map_err(|source| PackError::Write { path: out.into(), source })?;
                Ok(summary)
            }
            Err(err) => {
                let _ = fs::remove_file(&partial);
                Err(err)
            }
        }
    }

    fn write_to(
        self,
        path: &Path,
        progress: &mut impl FnMut(Progress),
    ) -> Result<Summary, PackError> {
        let write_error = |source| PackError::Write { path: path.into(), source };
        let mut previous = self.previous.as_ref().map(Pack::file).transpose()?;
        let in_previous: HashMap<ContentHash, &Entry> =
            self.previous.iter().flat_map(Pack::entries).map(|e| (e.hash, e)).collect();
        let mut compressor = compressor(self.level).map_err(write_error)?;

        let mut out = BufWriter::new(File::create(path).map_err(write_error)?);
        out.write_all(&[0; HEADER_LEN as usize]).map_err(write_error)?;
        let mut at = HEADER_LEN;
        let mut placed: HashMap<ContentHash, (u64, u64, Method)> = HashMap::new();
        let mut entries = Vec::with_capacity(self.files.len());
        let mut summary = Summary { files: self.files.len(), ..Summary::default() };

        for (done, (name, source)) in self.files.iter().enumerate() {
            progress(Progress { path: Some(name), done, total: summary.files });
            let contents: Option<Cow<[u8]>> = match source {
                Source::Kept(_) => None,
                Source::Bytes(bytes) => Some(Cow::Borrowed(bytes)),
                Source::Disk(file) => Some(Cow::Owned(
                    fs::read(file)
                        .map_err(|source| PackError::Read { path: file.clone(), source })?,
                )),
            };
            let (hash, size) = match (source, &contents) {
                (Source::Kept(entry), _) => (entry.hash, entry.size),
                (_, Some(bytes)) => (ContentHash::of(bytes), bytes.len() as u64),
                (_, None) => unreachable!("only kept files come without contents"),
            };
            summary.size += size;

            let (offset, stored, method) = match placed.get(&hash) {
                Some(&same) => {
                    summary.shared += 1;
                    same
                }
                None => {
                    // A kept file is always found here: its contents come from that pack.
                    let (data, method) = match (in_previous.get(&hash), &mut previous, contents) {
                        (Some(old), Some(file), _) => {
                            summary.reused += 1;
                            (Cow::Owned(file.read_stored(old)?), old.method)
                        }
                        (_, _, Some(bytes)) => {
                            squeeze(&mut compressor, name, bytes).map_err(write_error)?
                        }
                        _ => unreachable!("a kept file's contents are in the previous pack"),
                    };
                    let start = at.next_multiple_of(ALIGN);
                    let padding = [0; ALIGN as usize];
                    out.write_all(&padding[..(start - at) as usize]).map_err(write_error)?;
                    out.write_all(&data).map_err(write_error)?;
                    at = start + data.len() as u64;
                    let spot = (start, data.len() as u64, method);
                    placed.insert(hash, spot);
                    spot
                }
            };
            entries.push(Entry { path: name.clone(), offset, stored, size, method, hash });
        }

        let index = encode_index(&entries);
        let header = Header {
            entries: entries.len() as u32,
            index_offset: at,
            index_len: index.len() as u64,
            id: PackId::of_index(&index),
        };
        out.write_all(&index).map_err(write_error)?;
        let mut file = out.into_inner().map_err(|e| write_error(e.into_error()))?;
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.write_all(&header.encode()))
            .and_then(|()| file.sync_all())
            .map_err(write_error)?;
        summary.packed = at + index.len() as u64;
        progress(Progress { path: None, done: summary.files, total: summary.files });
        Ok(summary)
    }
}

fn compressor(level: i32) -> std::io::Result<Compressor<'static>> {
    let mut compressor = Compressor::new(level)?;
    compressor.include_checksum(true)?;
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    if cores > 1 {
        compressor.set_parameter(CParameter::NbWorkers(cores as u32))?;
    }
    Ok(compressor)
}

// Zstd only where it saves at least 5%: below that, unpacking costs more than it saves.
fn squeeze<'a>(
    compressor: &mut Compressor,
    path: &str,
    bytes: Cow<'a, [u8]>,
) -> std::io::Result<(Cow<'a, [u8]>, Method)> {
    let extension = path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase());
    let skip = extension.is_some_and(|ext| INCOMPRESSIBLE.contains(&ext.as_str()));
    if skip || bytes.len() < SMALLEST_TO_COMPRESS {
        return Ok((bytes, Method::Stored));
    }
    let packed = compressor.compress(&bytes)?;
    if (packed.len() as u64) * 20 <= (bytes.len() as u64) * 19 {
        Ok((Cow::Owned(packed), Method::Zstd))
    } else {
        Ok((bytes, Method::Stored))
    }
}

// A pack made on Linux must unpack on Windows, where `Skin.glb` and `skin.glb` are one file.
fn check_case(files: &BTreeMap<String, Source>) -> Result<(), PackError> {
    let mut seen: HashMap<String, &str> = HashMap::with_capacity(files.len());
    for path in files.keys() {
        if let Some(other) = seen.insert(path.to_lowercase(), path) {
            return Err(PackError::CaseClash(other.into(), path.clone()));
        }
    }
    Ok(())
}
