use std::collections::HashMap;
use std::io;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use bevy::asset::io::AssetReader;
use bevy::asset::io::AssetReaderError;
use bevy::asset::io::AssetSourceBuilder;
use bevy::asset::io::PathStream;
use bevy::asset::io::Reader;
use bevy::asset::io::VecReader;
use bevy::asset::io::file::FileAssetReader;
use bevy::log::warn;
use bevy::tasks::futures_lite::stream;
use charpaper_pack::FollowedPack;
use charpaper_pack::PACK_FILE;
use charpaper_pack::PackError;
use charpaper_pack::pack_path;

/// The asset source over the characters folder: `<suite folder>/<file>` comes out of that
/// suite's suite.chr when it has one, and off the disk when it doesn't.
pub fn suite_source(characters: PathBuf) -> AssetSourceBuilder {
    AssetSourceBuilder::new(move || Box::new(SuiteReader::new(&characters)))
}

struct SuiteReader {
    loose: FileAssetReader,
    // By suite folder. Only packs that opened: a suite without one is looked for again on
    // every read, so a suite packed while the app runs is noticed.
    packs: Mutex<HashMap<String, Arc<FollowedPack>>>,
}

impl SuiteReader {
    fn new(characters: &Path) -> Self {
        Self { loose: FileAssetReader::new(characters), packs: Mutex::default() }
    }

    // The suite's pack, if it's packed, and the path in it. `None` for a loose suite.
    fn packed(&self, path: &Path) -> Result<Option<(Arc<FollowedPack>, String)>, AssetReaderError> {
        let mut components = path.components();
        let Some(Component::Normal(folder)) = components.next() else {
            return Ok(None);
        };
        let folder = folder.to_string_lossy().into_owned();
        let rest = components.as_path();
        let key = match rest.as_os_str().is_empty() {
            true => String::new(),
            false => pack_path(rest).map_err(|_| AssetReaderError::NotFound(path.into()))?,
        };

        let mut packs = self.packs.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(pack) = packs.get(&folder) {
            return Ok(Some((pack.clone(), key)));
        }
        let file = self.loose.root_path().join(&folder).join(PACK_FILE);
        if !file.is_file() {
            return Ok(None);
        }
        // Small: only the index is read.
        let pack = Arc::new(FollowedPack::open(&file).map_err(|e| reader_error(path, e))?);
        packs.insert(folder, pack.clone());
        Ok(Some((pack, key)))
    }
}

impl AssetReader for SuiteReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let reader: Box<dyn Reader + 'a> = match self.packed(path)? {
            // Unpacking a skin takes a while, so it runs off Bevy's IO threads.
            Some((pack, key)) => {
                let bytes = blocking::unblock(move || pack.read(&key)).await;
                Box::new(VecReader::new(bytes.map_err(|e| reader_error(path, e))?))
            }
            None => Box::new(self.loose.read(path).await?),
        };
        Ok(reader)
    }

    // A pack holds no .meta files, so a packed asset always loads with its default settings.
    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let reader: Box<dyn Reader + 'a> = match self.packed(path)? {
            Some(_) => return Err(AssetReaderError::NotFound(path.into())),
            None => Box::new(self.loose.read_meta(path).await?),
        };
        Ok(reader)
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let Some((pack, key)) = self.packed(path)? else {
            return self.loose.read_directory(path).await;
        };
        let pack = pack.current();
        if !pack.is_dir(&key) {
            return Err(AssetReaderError::NotFound(path.into()));
        }
        let mut found: Vec<PathBuf> = pack.files_in(&key).map(|e| child(path, &e.path)).collect();
        found.extend(pack.folders_in(&key).into_iter().map(|name| path.join(name)));
        Ok(Box::new(stream::iter(found)))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        match self.packed(path)? {
            Some((pack, key)) => Ok(pack.current().is_dir(&key)),
            None => self.loose.is_directory(path).await,
        }
    }
}

fn child(dir: &Path, pack_path: &str) -> PathBuf {
    dir.join(pack_path.rsplit_once('/').map_or(pack_path, |(_, name)| name))
}

fn reader_error(path: &Path, err: PackError) -> AssetReaderError {
    match err {
        PackError::Missing { .. } => AssetReaderError::NotFound(path.into()),
        err => {
            warn!("cannot read {} from its suite's pack: {err}", path.display());
            AssetReaderError::Io(Arc::new(io::Error::other(err)))
        }
    }
}
