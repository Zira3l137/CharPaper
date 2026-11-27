use std::fmt;

use crate::path::check_path;

pub(crate) const MAGIC: [u8; 8] = *b"CHRPACK\0";
/// The pack format this build reads and writes.
pub const VERSION: u16 = 1;
pub(crate) const HEADER_LEN: u64 = 48;
// Each file's data starts on this boundary. Nothing needs it yet; it keeps the door open to
// mapping the file and handing out aligned slices later, for the cost of a few bytes.
pub(crate) const ALIGN: u64 = 16;
const ENTRY_FIXED_LEN: usize = 2 + 8 + 8 + 8 + 1 + 16;

/// Tells two packs apart: the first 16 bytes of the BLAKE3 hash of the index. The index
/// holds every file's offset and content hash, so a pack with any other content or layout
/// gets another id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackId(pub [u8; 16]);

impl PackId {
    pub(crate) fn of_index(index: &[u8]) -> Self {
        Self(truncated(blake3::hash(index)))
    }
}

impl fmt::Display for PackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hex(&self.0, f)
    }
}

/// The first 16 bytes of the BLAKE3 hash of a file's contents, as unpacked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash(pub [u8; 16]);

impl ContentHash {
    pub fn of(bytes: &[u8]) -> Self {
        Self(truncated(blake3::hash(bytes)))
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hex(&self.0, f)
    }
}

fn truncated(hash: blake3::Hash) -> [u8; 16] {
    let mut out = [0; 16];
    out.copy_from_slice(&hash.as_bytes()[..16]);
    out
}

fn hex(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    bytes.iter().try_for_each(|b| write!(f, "{b:02x}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Stored,
    Zstd,
}

impl Method {
    fn code(self) -> u8 {
        match self {
            Self::Stored => 0,
            Self::Zstd => 1,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Stored),
            1 => Some(Self::Zstd),
            _ => None,
        }
    }
}

/// One file in the pack. Files with the same contents may share their data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub offset: u64,
    /// Bytes in the pack.
    pub stored: u64,
    /// Bytes once unpacked.
    pub size: u64,
    pub method: Method,
    pub hash: ContentHash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    pub entries: u32,
    pub index_offset: u64,
    pub index_len: u64,
    pub id: PackId,
}

pub(crate) enum HeaderProblem {
    NotAPack,
    Version(u16),
}

impl Header {
    pub fn encode(&self) -> [u8; HEADER_LEN as usize] {
        let mut out = [0; HEADER_LEN as usize];
        out[0..8].copy_from_slice(&MAGIC);
        out[8..10].copy_from_slice(&VERSION.to_le_bytes());
        // 10..12 are flags, none defined yet.
        out[12..16].copy_from_slice(&self.entries.to_le_bytes());
        out[16..24].copy_from_slice(&self.index_offset.to_le_bytes());
        out[24..32].copy_from_slice(&self.index_len.to_le_bytes());
        out[32..48].copy_from_slice(&self.id.0);
        out
    }

    pub fn decode(bytes: &[u8; HEADER_LEN as usize]) -> Result<Self, HeaderProblem> {
        if bytes[0..8] != MAGIC {
            return Err(HeaderProblem::NotAPack);
        }
        let version = u16::from_le_bytes([bytes[8], bytes[9]]);
        if version != VERSION {
            return Err(HeaderProblem::Version(version));
        }
        let u64_at = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
        let mut id = [0; 16];
        id.copy_from_slice(&bytes[32..48]);
        Ok(Self {
            entries: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            index_offset: u64_at(16),
            index_len: u64_at(24),
            id: PackId(id),
        })
    }
}

pub(crate) fn encode_index(entries: &[Entry]) -> Vec<u8> {
    let len = entries.iter().map(|e| ENTRY_FIXED_LEN + e.path.len()).sum();
    let mut out = Vec::with_capacity(len);
    for entry in entries {
        out.extend((entry.path.len() as u16).to_le_bytes());
        out.extend(entry.path.as_bytes());
        out.extend(entry.offset.to_le_bytes());
        out.extend(entry.stored.to_le_bytes());
        out.extend(entry.size.to_le_bytes());
        out.push(entry.method.code());
        out.extend(entry.hash.0);
    }
    out
}

// Checks everything a reader leans on: sorted unique paths for the binary search, and data
// that lies between the header and the index.
pub(crate) fn decode_index(bytes: &[u8], header: &Header) -> Result<Vec<Entry>, String> {
    let mut entries: Vec<Entry> = Vec::with_capacity(header.entries.min(1 << 16) as usize);
    let mut rest = bytes;
    let mut take = |n: usize| -> Result<&[u8], String> {
        if rest.len() < n {
            return Err("the index ends too soon".into());
        }
        let (head, tail) = rest.split_at(n);
        rest = tail;
        Ok(head)
    };
    for _ in 0..header.entries {
        let len = u16::from_le_bytes(take(2)?.try_into().unwrap()) as usize;
        let path = std::str::from_utf8(take(len)?)
            .map_err(|_| "a path in the index isn't UTF-8".to_string())?
            .to_string();
        check_path(&path).map_err(|e| e.to_string())?;
        let mut u64_next = || Ok::<_, String>(u64::from_le_bytes(take(8)?.try_into().unwrap()));
        let (offset, stored, size) = (u64_next()?, u64_next()?, u64_next()?);
        let method = Method::from_code(take(1)?[0])
            .ok_or_else(|| format!("{path:?} is packed in a way this CharPaper doesn't know"))?;
        let mut hash = [0; 16];
        hash.copy_from_slice(take(16)?);

        let end = offset.checked_add(stored);
        if offset < HEADER_LEN || end.is_none_or(|end| end > header.index_offset) {
            return Err(format!("{path:?} lies outside the pack's data"));
        }
        if method == Method::Stored && stored != size {
            return Err(format!("{path:?} has two different sizes"));
        }
        if entries.last().is_some_and(|last| last.path >= path) {
            return Err("the index isn't sorted".into());
        }
        entries.push(Entry { path, offset, stored, size, method, hash: ContentHash(hash) });
    }
    if !rest.is_empty() {
        return Err("the index has bytes left over".into());
    }
    Ok(entries)
}
