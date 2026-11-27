//! The Blender add-on's Python module, `charpaper_pack`: suite packs and sky baking, through
//! the same code the app runs. Every call lets go of Python's lock while it works, so
//! Blender keeps drawing.

use std::fs;
use std::ops::ControlFlow;
use std::path::Path;
use std::path::PathBuf;

use charpaper_bake::BakeError;
use charpaper_bake::BakeSettings;
use charpaper_suite::MANIFEST_FILE;
use chr::PACK_FILE;
use chr::Pack;
use chr::PackError;
use chr::PackWriter;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::types::PyDict;

create_exception!(charpaper_pack, PackFailed, PyException, "A pack can't be read or written.");
create_exception!(
    charpaper_pack,
    PackReplaced,
    PackFailed,
    "The pack was replaced since it was opened: open it again."
);
create_exception!(charpaper_pack, BakeFailed, PyException, "A sky can't be baked.");

// The files beside the pack in a suite folder, never in it.
const BESIDE_THE_PACK: &[&str] = &[MANIFEST_FILE, "suite.toml.bak", PACK_FILE, "suite.chr.part"];

fn pack_error(err: PackError) -> PyErr {
    let message = error_chain(&err);
    match err {
        PackError::Missing { file, .. } => PyKeyError::new_err(file),
        PackError::Replaced(_) => PackReplaced::new_err(message),
        _ => PackFailed::new_err(message),
    }
}

// "cannot read x: Access is denied", not just "cannot read x".
fn error_chain(err: &dyn std::error::Error) -> String {
    let mut message = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        message = format!("{message}: {cause}");
        source = cause.source();
    }
    message
}

/// A suite pack, opened. Only its list of files is kept; each read opens the file again.
#[pyclass(frozen, module = "charpaper_pack", name = "Pack")]
struct PyPack(Pack);

#[pymethods]
impl PyPack {
    #[new]
    fn open(py: Python<'_>, path: PathBuf) -> PyResult<Self> {
        py.detach(|| Pack::open(path)).map(Self).map_err(pack_error)
    }

    #[getter]
    fn path(&self) -> PathBuf {
        self.0.path().to_path_buf()
    }

    /// Changes whenever the pack's contents do.
    #[getter]
    fn id(&self) -> String {
        self.0.id().to_string()
    }

    /// Every file's path, sorted.
    fn files(&self) -> Vec<String> {
        self.0.entries().iter().map(|e| e.path.clone()).collect()
    }

    /// The paths of the files right in `folder`, not in its sub-folders.
    #[pyo3(signature = (folder = ""))]
    fn files_in(&self, folder: &str) -> Vec<String> {
        self.0.files_in(folder).map(|e| e.path.clone()).collect()
    }

    /// The names of the folders right in `folder`.
    #[pyo3(signature = (folder = ""))]
    fn folders_in(&self, folder: &str) -> Vec<String> {
        self.0.folders_in(folder).into_iter().map(str::to_string).collect()
    }

    fn is_file(&self, path: &str) -> bool {
        self.0.is_file(path)
    }

    fn is_dir(&self, path: &str) -> bool {
        self.0.is_dir(path)
    }

    /// `size` unpacked, `stored` in the pack, `compressed`, and `hash`, which only matches
    /// another file's when the contents are the same.
    fn info<'py>(&self, py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyDict>> {
        let entry = self.0.entry(path).ok_or_else(|| PyKeyError::new_err(path.to_string()))?;
        let info = PyDict::new(py);
        info.set_item("size", entry.size)?;
        info.set_item("stored", entry.stored)?;
        info.set_item("compressed", entry.method != chr::Method::Stored)?;
        info.set_item("hash", entry.hash.to_string())?;
        Ok(info)
    }

    fn read<'py>(&self, py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = py.detach(|| self.0.read(path)).map_err(pack_error)?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Writes one file of the pack to `to`, making its folders.
    fn extract(&self, py: Python<'_>, path: &str, to: PathBuf) -> PyResult<()> {
        py.detach(|| {
            let bytes = self.0.read(path)?;
            write_file(&to, &bytes)
        })
        .map_err(pack_error)
    }

    /// Reads every file and checks it against its hash.
    fn verify(&self, py: Python<'_>) -> PyResult<()> {
        py.detach(|| self.0.verify()).map_err(pack_error)
    }

    fn __contains__(&self, path: &str) -> bool {
        self.0.is_file(path)
    }

    fn __len__(&self) -> usize {
        self.0.entries().len()
    }

    fn __repr__(&self) -> String {
        format!("Pack({:?}, {} files)", self.0.path().display().to_string(), self.0.entries().len())
    }
}

fn write_file(to: &Path, bytes: &[u8]) -> Result<(), PackError> {
    let write_error = |source| PackError::Write { path: to.to_path_buf(), source };
    if let Some(folder) = to.parent() {
        fs::create_dir_all(folder).map_err(write_error)?;
    }
    fs::write(to, bytes).map_err(write_error)
}

/// Packs every file under `folder` into `out`, except suite.toml and its backup.
///
/// With `reuse`, files whose contents are already in the pack at `out` are copied from it
/// instead of being compressed again. `keep` lists paths and folders, such as `"skins"`,
/// to carry over from that pack where `folder` doesn't have them, for an export of only
/// some parts. `progress(path, done, total)` is called before each file, and once with
/// `None` at the end. An exception it raises comes out of `pack`; raised before the end, it
/// stops packing and leaves `out` as it was.
///
/// Returns `files`, `size` (unpacked bytes), `packed` (the pack's bytes), `reused` and
/// `shared` (files with the same contents as another).
#[pyfunction]
#[pyo3(signature = (
    folder, out, *, reuse = true, keep = Vec::new(), level = chr::DEFAULT_LEVEL, progress = None
))]
fn pack<'py>(
    py: Python<'py>,
    folder: PathBuf,
    out: PathBuf,
    reuse: bool,
    keep: Vec<String>,
    level: i32,
    progress: Option<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyDict>> {
    let progress = progress.map(Bound::unbind);
    let summary = py.detach(|| -> PyResult<_> {
        let mut writer = PackWriter::new().level(level);
        let previous = if reuse && out.is_file() { Some(Pack::open(&out)) } else { None };
        match previous {
            Some(Ok(previous)) => writer.reuse(previous),
            Some(Err(err)) if !keep.is_empty() => {
                let why = error_chain(&err);
                return Err(PackFailed::new_err(format!("nothing to keep files from: {why}")));
            }
            // A pack that can't be read is simply packed again from scratch.
            Some(Err(_)) => {}
            None if !keep.is_empty() => {
                let what = out.display();
                return Err(PackFailed::new_err(format!("nothing to keep files from: no {what}")));
            }
            None => {}
        }
        writer.add_folder(&folder, |path| !BESIDE_THE_PACK.contains(&path)).map_err(pack_error)?;
        writer.keep(|path| keep.iter().any(|k| path == k || is_under(path, k)));

        let mut raised = None;
        let result = writer.write(&out, |step| {
            let Some(callback) = &progress else {
                return ControlFlow::Continue(());
            };
            Python::attach(|py| match callback.call1(py, (step.path, step.done, step.total)) {
                Ok(_) => ControlFlow::Continue(()),
                Err(err) => {
                    raised = Some(err);
                    ControlFlow::Break(())
                }
            })
        });
        // Raised by the last call, the error comes too late to stop anything, but it still
        // shouldn't vanish.
        match (result, raised) {
            (Err(PackError::Stopped) | Ok(_), Some(err)) => Err(err),
            (result, _) => result.map_err(pack_error),
        }
    })?;
    let info = PyDict::new(py);
    info.set_item("files", summary.files)?;
    info.set_item("size", summary.size)?;
    info.set_item("packed", summary.packed)?;
    info.set_item("reused", summary.reused)?;
    info.set_item("shared", summary.shared)?;
    Ok(info)
}

fn is_under(path: &str, folder: &str) -> bool {
    path.strip_prefix(folder).is_some_and(|rest| rest.starts_with('/'))
}

/// Bakes the sky maps the app needs from an environment's .hdr or .exr panorama into
/// `folder`, as the app would on first load. Maps already in `folder` are left alone.
/// Returns the paths written.
#[pyfunction]
fn bake(py: Python<'_>, panorama: PathBuf, folder: PathBuf) -> PyResult<Vec<PathBuf>> {
    py.detach(|| {
        fs::create_dir_all(&folder)
            .map_err(|source| BakeError::Write { path: folder.clone(), source })?;
        charpaper_bake::bake(&panorama, &folder, &BakeSettings::default())
    })
    .map_err(|err| BakeFailed::new_err(error_chain(&err)))
}

/// The names of the sky maps `folder` doesn't have yet.
#[pyfunction]
fn missing_maps(folder: PathBuf) -> Vec<&'static str> {
    charpaper_bake::missing(&folder).into_iter().map(|map| map.file_name()).collect()
}

#[pymodule]
mod charpaper_pack {
    #[pymodule_export]
    use super::BakeFailed;
    #[pymodule_export]
    use super::PackFailed;
    #[pymodule_export]
    use super::PackReplaced;
    #[pymodule_export]
    use super::PyPack;
    #[pymodule_export]
    use super::bake;
    #[pymodule_export]
    use super::missing_maps;
    #[pymodule_export]
    use super::pack;

    #[pymodule_export]
    const VERSION: u16 = chr::VERSION;
    #[pymodule_export]
    const PACK_FILE: &str = chr::PACK_FILE;
    #[pymodule_export]
    const DEFAULT_LEVEL: i32 = chr::DEFAULT_LEVEL;
}
