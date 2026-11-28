//! The Blender add-on's Python module, `charpaper_pack`: suite packs and sky baking, through
//! the same code the app runs. Every call lets go of Python's lock while it works, so
//! Blender keeps drawing.

use std::collections::HashMap;
use std::fs;
use std::ops::ControlFlow;
use std::path::Path;
use std::path::PathBuf;

use charpaper_bake::BakeError;
use charpaper_bake::BakeSettings;
use charpaper_gltf as gltf;
use charpaper_suite::MANIFEST_FILE;
use chr::PACK_FILE;
use chr::Pack;
use chr::PackError;
use chr::PackWriter;
use glam::Mat4;
use glam::Vec2;
use glam::Vec3;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedBytes;
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

/// An armature's bones as the add-on reads them out of Blender, in the armature's order.
#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct BonesArg {
    // Each bone's exported parent by index, -1 for none, and whether it has any in Blender.
    parents: Vec<i64>,
    has_parent: Vec<bool>,
    // Float32 matrices, column by column as Blender's foreach_get gives them: `rest` one per
    // bone, `poses` one per bone for every sample in turn.
    rest: PyBackedBytes,
    poses: PyBackedBytes,
    // The bones to write, by index, and their names.
    written: Vec<(usize, String)>,
}

/// Writes a clip's .glb: the bones and objects it moves, sampled `samples` times
/// `step_seconds` apart. `objects` are (name, float32 matrices relative to the parent in
/// Blender, one per sample). `channels` tells, by node, how the exporter treats translation,
/// rotation and scale: "keyed", "stepped" or "baked"; a node left out is keyed. Lets go of
/// Python's lock while it works.
#[pyfunction]
#[pyo3(signature = (path, name, samples, step_seconds, optimize, bones = None, objects = Vec::new(), channels = HashMap::new()))]
fn write_clip(
    py: Python<'_>,
    path: PathBuf,
    name: String,
    samples: usize,
    step_seconds: f32,
    optimize: bool,
    bones: Option<BonesArg>,
    objects: Vec<(String, PyBackedBytes)>,
    channels: HashMap<String, [String; 3]>,
) -> PyResult<()> {
    py.detach(|| {
        let mut nodes = Vec::new();
        if let Some(bones) = bones {
            let count = bones.parents.len();
            let rest = matrices(&bones.rest, count, "rest")?;
            let poses = matrices(&bones.poses, count * samples, "poses")?;
            let armature = gltf::blender::Bones {
                rest,
                parents: bones.parents.iter().map(|&p| usize::try_from(p).ok()).collect(),
                has_parent: bones.has_parent,
            };
            let mut tracks: Vec<Vec<gltf::Trs>> = vec![Vec::with_capacity(samples); count];
            for frame in poses.chunks_exact(count) {
                for (track, local) in tracks.iter_mut().zip(armature.locals(frame)) {
                    track.push(local);
                }
            }
            for (index, bone) in bones.written {
                let samples = tracks
                    .get(index)
                    .cloned()
                    .ok_or_else(|| PyValueError::new_err("no such bone"))?;
                let channels = channel_kinds(&channels, &bone)?;
                nodes.push(gltf::ClipNode { name: bone, samples, channels });
            }
        }
        nodes.extend(object_nodes(objects, samples, &[], &channels)?);
        let clip = gltf::Clip { name, step_seconds, optimize, drop_still_baked: false, nodes };
        write_file(&path, &clip.to_glb()).map_err(pack_error)
    })
}

fn object_nodes(
    objects: Vec<(String, PyBackedBytes)>,
    samples: usize,
    turned: &[String],
    channels: &HashMap<String, [String; 3]>,
) -> PyResult<Vec<gltf::ClipNode>> {
    objects
        .into_iter()
        .map(|(object, locals)| {
            let local = match turned.contains(&object) {
                true => gltf::blender::turned_local,
                false => gltf::blender::object_local,
            };
            let samples = matrices(&locals, samples, &object)?.into_iter().map(local).collect();
            let channels = channel_kinds(channels, &object)?;
            Ok(gltf::ClipNode { name: object, samples, channels })
        })
        .collect()
}

fn channel_kinds(
    channels: &HashMap<String, [String; 3]>,
    node: &str,
) -> PyResult<[gltf::Channel; 3]> {
    let Some(kinds) = channels.get(node) else {
        return Ok([gltf::Channel::Keyed; 3]);
    };
    let mut found = [gltf::Channel::Keyed; 3];
    for (slot, kind) in found.iter_mut().zip(kinds) {
        *slot = match kind.as_str() {
            "keyed" => gltf::Channel::Keyed,
            "stepped" => gltf::Channel::Stepped,
            "baked" => gltf::Channel::Baked,
            other => {
                return Err(PyValueError::new_err(format!("{node}: no channel kind {other:?}")));
            }
        };
    }
    Ok(found)
}

/// What plays in a scene's file, as `write_clip` has it, its objects among the scene's nodes.
#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct AnimationArg {
    name: String,
    samples: usize,
    step_seconds: f32,
    optimize: bool,
    drop_still_baked: bool,
    objects: Vec<(String, PyBackedBytes)>,
    // Cameras and lights, whose rotation the exporter turns.
    turned: Vec<String>,
    channels: HashMap<String, [String; 3]>,
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct LightArg {
    name: String,
    // "point", "spot" or "directional".
    kind: String,
    color: [f32; 3],
    intensity: f32,
    range: Option<f32>,
    cone: Option<(f32, f32)>,
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct CameraArg {
    name: String,
    // "perspective" with aspect_ratio and yfov, or "orthographic" with xmag and ymag.
    kind: String,
    znear: f32,
    zfar: f32,
    aspect_ratio: Option<f32>,
    yfov: Option<f32>,
    xmag: Option<f32>,
    ymag: Option<f32>,
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct SceneArg {
    name: String,
    nodes: Vec<NodeArg>,
    roots: Vec<usize>,
    skins: Vec<SkinArg>,
    meshes: Vec<MeshArg>,
    lights: Vec<LightArg>,
    cameras: Vec<CameraArg>,
    animation: Option<AnimationArg>,
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct NodeArg {
    name: String,
    // Relative to the parent, in Blender's axes.
    matrix: PyBackedBytes,
    children: Vec<usize>,
    mesh: Option<usize>,
    skin: Option<usize>,
    light: Option<usize>,
    camera: Option<usize>,
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct SkinArg {
    name: String,
    joints: Vec<usize>,
    inverse_binds: PyBackedBytes,
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct MeshArg {
    name: String,
    positions: PyBackedBytes,
    corner_verts: PyBackedBytes,
    normals: PyBackedBytes,
    uvs: Vec<PyBackedBytes>,
    tangents: Option<PyBackedBytes>,
    signs: Option<PyBackedBytes>,
    triangles: PyBackedBytes,
    triangle_slots: PyBackedBytes,
    shape_keys: Vec<ShapeKeyArg>,
    skin: Option<InfluencesArg>,
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct ShapeKeyArg {
    name: String,
    weight: f32,
    positions: PyBackedBytes,
    normals: Option<PyBackedBytes>,
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
struct InfluencesArg {
    // Per vertex how many of `joints` and `weights` are its, in order.
    counts: PyBackedBytes,
    joints: PyBackedBytes,
    weights: PyBackedBytes,
    object_world: PyBackedBytes,
    armature_world: PyBackedBytes,
}

/// Writes a model or skin .glb from what the add-on read out of Blender, with the materials,
/// textures and images of `materials`, a .glb Blender's exporter wrote of the meshes' material
/// slots. Raises ValueError for what it doesn't write as Blender's exporter would, having
/// written nothing.
#[pyfunction]
#[pyo3(signature = (path, scene, materials = None))]
fn write_scene(
    py: Python<'_>,
    path: PathBuf,
    scene: SceneArg,
    materials: Option<PyBackedBytes>,
) -> PyResult<()> {
    py.detach(|| {
        let nodes = scene
            .nodes
            .into_iter()
            .map(|node| {
                let matrix = matrices(&node.matrix, 1, &node.name)?[0];
                Ok(gltf::scene::Node {
                    trs: gltf::blender::object_local(matrix),
                    name: node.name,
                    children: node.children,
                    mesh: node.mesh,
                    skin: node.skin,
                    light: node.light,
                    camera: node.camera,
                })
            })
            .collect::<PyResult<Vec<_>>>()?;
        let skins = scene
            .skins
            .into_iter()
            .map(|skin| {
                let inverse_binds = matrices(&skin.inverse_binds, skin.joints.len(), &skin.name)?;
                Ok(gltf::scene::Skin { name: skin.name, joints: skin.joints, inverse_binds })
            })
            .collect::<PyResult<Vec<_>>>()?;
        let meshes = scene.meshes.into_iter().map(mesh_data).collect::<PyResult<Vec<_>>>()?;
        let lights = scene.lights.into_iter().map(light).collect::<PyResult<Vec<_>>>()?;
        let cameras = scene.cameras.into_iter().map(camera).collect::<PyResult<Vec<_>>>()?;
        let animation = match scene.animation {
            None => None,
            Some(clip) => Some(gltf::Clip {
                nodes: object_nodes(clip.objects, clip.samples, &clip.turned, &clip.channels)?,
                name: clip.name,
                step_seconds: clip.step_seconds,
                optimize: clip.optimize,
                drop_still_baked: clip.drop_still_baked,
            }),
        };
        let scene = gltf::Scene {
            name: scene.name,
            nodes,
            roots: scene.roots,
            meshes,
            skins,
            lights,
            cameras,
            animation,
            materials: materials.map(|m| m.to_vec()),
        };
        let glb = scene.to_glb().map_err(PyValueError::new_err)?;
        write_file(&path, &glb).map_err(pack_error)
    })
}

fn light(light: LightArg) -> PyResult<gltf::scene::Light> {
    use gltf::scene::LightKind;
    let kind = match light.kind.as_str() {
        "point" => LightKind::Point,
        "spot" => LightKind::Spot,
        "directional" => LightKind::Directional,
        other => {
            return Err(PyValueError::new_err(format!("{}: no light kind {other:?}", light.name)));
        }
    };
    Ok(gltf::scene::Light {
        name: light.name,
        kind,
        color: light.color,
        intensity: light.intensity,
        range: light.range,
        cone: light.cone,
    })
}

fn camera(camera: CameraArg) -> PyResult<gltf::scene::Camera> {
    use gltf::scene::Projection;
    let missing = || PyValueError::new_err(format!("{}: the camera lacks a value", camera.name));
    let (znear, zfar) = (camera.znear, camera.zfar);
    let projection = match camera.kind.as_str() {
        "perspective" => Projection::Perspective {
            aspect_ratio: camera.aspect_ratio.ok_or_else(missing)?,
            yfov: camera.yfov.ok_or_else(missing)?,
            znear,
            zfar,
        },
        "orthographic" => Projection::Orthographic {
            xmag: camera.xmag.ok_or_else(missing)?,
            ymag: camera.ymag.ok_or_else(missing)?,
            znear,
            zfar,
        },
        other => {
            return Err(PyValueError::new_err(format!(
                "{}: no camera kind {other:?}",
                camera.name
            )));
        }
    };
    Ok(gltf::scene::Camera { name: camera.name, projection })
}

fn mesh_data(mesh: MeshArg) -> PyResult<gltf::mesh::MeshData> {
    let name = mesh.name;
    let wrong = |what: &str| PyValueError::new_err(format!("{name}: {what} don't fit the mesh"));
    let positions = vec3s(&mesh.positions);
    let corner_verts = ints(&mesh.corner_verts);
    let corners = corner_verts.len();
    let normals = vec3s(&mesh.normals);
    if normals.len() != corners || corner_verts.iter().any(|&v| v as usize >= positions.len()) {
        return Err(wrong("normals or corners"));
    }
    let uvs: Vec<Vec<Vec2>> = mesh
        .uvs
        .iter()
        .map(|layer| floats(layer).as_chunks().0.iter().map(|&uv| Vec2::from_array(uv)).collect())
        .collect();
    if uvs.iter().any(|layer: &Vec<Vec2>| layer.len() != corners) {
        return Err(wrong("UVs"));
    }
    let tangents = match (mesh.tangents, mesh.signs) {
        (Some(tangents), Some(signs)) => Some((vec3s(&tangents), floats(&signs))),
        _ => None,
    };
    if tangents.as_ref().is_some_and(|(t, s)| t.len() != corners || s.len() != corners) {
        return Err(wrong("tangents"));
    }
    let triangle_corners = ints(&mesh.triangles);
    if triangle_corners.iter().any(|&c| c as usize >= corners) {
        return Err(wrong("triangles"));
    }
    let triangles: Vec<[u32; 3]> = triangle_corners.as_chunks().0.to_vec();
    let triangle_slots = ints(&mesh.triangle_slots);
    if triangle_slots.len() != triangles.len() {
        return Err(wrong("material slots"));
    }
    let shape_keys = mesh
        .shape_keys
        .into_iter()
        .map(|key| {
            let positions = vec3s(&key.positions);
            let normals = key.normals.as_deref().map(vec3s);
            if normals.as_ref().is_some_and(|n| n.len() != corners) {
                return Err(wrong("shape keys"));
            }
            Ok(gltf::mesh::ShapeKey { name: key.name, weight: key.weight, positions, normals })
        })
        .collect::<PyResult<Vec<_>>>()?;
    if shape_keys.iter().any(|k| k.positions.len() != positions.len()) {
        return Err(wrong("shape keys"));
    }
    let skin = match mesh.skin {
        None => None,
        Some(skin) => {
            let counts = ints(&skin.counts);
            let joints = ints(&skin.joints);
            let weights = floats(&skin.weights);
            if counts.len() != positions.len() || joints.len() != weights.len() {
                return Err(wrong("vertex groups"));
            }
            let mut influences = Vec::with_capacity(counts.len());
            let mut at = 0;
            for count in counts {
                let end = at + count as usize;
                let pairs = joints
                    .get(at..end)
                    .zip(weights.get(at..end))
                    .ok_or_else(|| wrong("vertex groups"))?;
                influences
                    .push(pairs.0.iter().zip(pairs.1).map(|(&j, &w)| (j as u16, w)).collect());
                at = end;
            }
            Some(gltf::mesh::SkinData {
                influences,
                object_world: matrices(&skin.object_world, 1, &name)?[0],
                armature_world: matrices(&skin.armature_world, 1, &name)?[0],
            })
        }
    };
    Ok(gltf::mesh::MeshData {
        name,
        positions,
        corner_verts,
        normals,
        uvs,
        tangents,
        triangles,
        triangle_slots,
        shape_keys,
        skin,
    })
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes.as_chunks().0.iter().map(|&b| f32::from_le_bytes(b)).collect()
}

fn ints(bytes: &[u8]) -> Vec<u32> {
    bytes.as_chunks().0.iter().map(|&b| u32::from_le_bytes(b)).collect()
}

fn vec3s(bytes: &[u8]) -> Vec<Vec3> {
    floats(bytes).as_chunks().0.iter().map(|&v| Vec3::from_array(v)).collect()
}

fn matrices(bytes: &[u8], count: usize, what: &str) -> PyResult<Vec<Mat4>> {
    if bytes.len() != count * 64 {
        let message = format!("{what}: {} bytes for {count} matrices", bytes.len());
        return Err(PyValueError::new_err(message));
    }
    Ok(floats(bytes).as_chunks().0.iter().map(Mat4::from_cols_array).collect())
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
    use super::write_clip;
    #[pymodule_export]
    use super::write_scene;

    #[pymodule_export]
    const VERSION: u16 = chr::VERSION;
    #[pymodule_export]
    const PACK_FILE: &str = chr::PACK_FILE;
    #[pymodule_export]
    const DEFAULT_LEVEL: i32 = chr::DEFAULT_LEVEL;
}
