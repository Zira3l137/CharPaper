use std::collections::BTreeMap;
use std::thread;

use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use gltf::json;
use gltf::json::accessor::ComponentType;
use gltf::json::accessor::Type;
use gltf::json::mesh::Semantic;
use gltf::json::validation::Checked::Valid;

use crate::build::Glb;
use crate::clip::Clip;
use crate::clip::Trs;
use crate::mesh::MeshData;
use crate::mesh::Primitive;

/// A file of nodes, meshes and an armature: the model, a skin. Its materials, textures and
/// images come from `materials`, a .glb Blender's own exporter wrote of the meshes' materials
/// alone, so they are Blender's to the last setting.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub name: String,
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
    pub meshes: Vec<MeshData>,
    pub skins: Vec<Skin>,
    pub lights: Vec<Light>,
    pub cameras: Vec<Camera>,
    /// What plays in the file: a moving camera's motion. Its nodes are the scene's, by name.
    pub animation: Option<Clip>,
    /// In it, mesh `i` here is the mesh named `proxy_name(i)`, with one primitive for each of
    /// its material slots in order.
    pub materials: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub trs: Trs,
    pub children: Vec<usize>,
    pub mesh: Option<usize>,
    pub skin: Option<usize>,
    pub light: Option<usize>,
    pub camera: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Skin {
    pub name: String,
    pub joints: Vec<usize>,
    pub inverse_binds: Vec<Mat4>,
}

/// A light as KHR_lights_punctual has it, its numbers as Blender's exporter works them out.
#[derive(Debug, Clone)]
pub struct Light {
    pub name: String,
    pub kind: LightKind,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: Option<f32>,
    /// A spot's inner and outer cone angles.
    pub cone: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightKind {
    Point,
    Spot,
    Directional,
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub name: String,
    pub projection: Projection,
}

#[derive(Debug, Clone, Copy)]
pub enum Projection {
    Perspective { aspect_ratio: f32, yfov: f32, znear: f32, zfar: f32 },
    Orthographic { xmag: f32, ymag: f32, znear: f32, zfar: f32 },
}

/// The name of mesh `index`'s stand-in in the materials file.
pub fn proxy_name(index: usize) -> String {
    format!("charpaper~proxy~{index}")
}

impl Scene {
    /// The .glb, or what this writer doesn't do that the file needs, for Blender's exporter to
    /// write it instead.
    pub fn to_glb(&self) -> Result<Vec<u8>, String> {
        if let Some(mesh) = self.meshes.iter().find(|m| m.needs_neutral_bone()) {
            return Err(format!("{} has vertices on no bone", mesh.name));
        }
        let mut glb = Glb::new();
        let slots = match &self.materials {
            Some(bytes) => take_materials(&mut glb, bytes, self.meshes.len())?,
            None => vec![Vec::new(); self.meshes.len()],
        };

        // The heavy part, and each mesh is its own: one thread for each.
        let primitives: Vec<Vec<Primitive>> = thread::scope(|scope| {
            let jobs: Vec<_> = self.meshes.iter().map(|m| scope.spawn(|| m.primitives())).collect();
            jobs.into_iter().map(|job| job.join().expect("building a mesh doesn't panic")).collect()
        });
        for ((mesh, primitives), slots) in self.meshes.iter().zip(primitives).zip(&slots) {
            let mesh = write_mesh(&mut glb, mesh, primitives, slots);
            glb.root.push(mesh);
        }

        for skin in &self.skins {
            let matrices: Vec<f32> =
                skin.inverse_binds.iter().flat_map(|m| m.to_cols_array()).collect();
            let inverse_binds = glb.floats(&matrices, Type::Mat4, false);
            glb.root.push(json::Skin {
                extensions: None,
                extras: Default::default(),
                inverse_bind_matrices: Some(inverse_binds),
                joints: skin.joints.iter().map(|&j| json::Index::new(j as u32)).collect(),
                name: Some(skin.name.clone()),
                skeleton: None,
            });
        }

        let lights: Vec<_> = self.lights.iter().map(light).collect();
        if !lights.is_empty() {
            let extensions = glb.root.extensions.get_or_insert_with(Default::default);
            extensions.khr_lights_punctual =
                Some(json::extensions::root::KhrLightsPunctual { lights });
            used(&mut glb.root.extensions_used, "KHR_lights_punctual");
        }
        for camera in &self.cameras {
            glb.root.push(write_camera(camera));
        }

        for node in &self.nodes {
            let Trs { translation, rotation, scale } = near(node.trs);
            let children =
                node.children.iter().map(|&c| json::Index::new(c as u32)).collect::<Vec<_>>();
            glb.root.push(json::Node {
                name: Some(node.name.clone()),
                children: (!children.is_empty()).then_some(children),
                mesh: node.mesh.map(|m| json::Index::new(m as u32)),
                skin: node.skin.map(|s| json::Index::new(s as u32)),
                translation: (translation != Vec3::ZERO).then(|| translation.into()),
                rotation: (rotation != Quat::IDENTITY)
                    .then(|| json::scene::UnitQuaternion(rotation.to_array())),
                scale: (scale != Vec3::ONE).then(|| scale.into()),
                camera: node.camera.map(|c| json::Index::new(c as u32)),
                // Built up from the default: Bevy's build adds fields to it.
                extensions: node.light.map(|l| {
                    let mut extensions = json::extensions::scene::Node::default();
                    extensions.khr_lights_punctual =
                        Some(json::extensions::scene::khr_lights_punctual::KhrLightsPunctual {
                            light: json::Index::new(l as u32),
                        });
                    extensions
                }),
                ..Default::default()
            });
        }
        if let Some(clip) = &self.animation {
            let nodes = clip
                .nodes
                .iter()
                .map(|moved| {
                    let index = self.nodes.iter().position(|n| n.name == moved.name);
                    index.map(|i| json::Index::new(i as u32)).ok_or_else(|| {
                        format!("{} moves in the animation but isn't in the file", moved.name)
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(animation) = clip.animation(&mut glb, &nodes) {
                glb.root.push(animation);
            }
        }
        let scene = glb.root.push(json::Scene {
            extensions: None,
            extras: Default::default(),
            name: Some(self.name.clone()),
            nodes: self.roots.iter().map(|&r| json::Index::new(r as u32)).collect(),
        });
        glb.root.scene = Some(scene);
        Ok(glb.into_vec())
    }
}

// Copies the materials file's materials, textures, samplers and images over as its JSON has
// them, defaults left out as Blender leaves them, and returns each mesh's material per slot.
// They go in first, so their indices stay as they were.
fn take_materials(
    glb: &mut Glb,
    bytes: &[u8],
    meshes: usize,
) -> Result<Vec<Vec<Option<json::Index<json::Material>>>>, String> {
    let wrong = |e: &dyn std::fmt::Display| format!("the materials file: {e}");
    let file = gltf::binary::Glb::from_slice(bytes).map_err(|e| wrong(&e))?;
    let blob = file.bin.as_deref().unwrap_or_default();
    let mut raw: serde_json::Value = serde_json::from_slice(&file.json).map_err(|e| wrong(&e))?;
    let proxy = json::Root::from_slice(&file.json).map_err(|e| wrong(&e))?;

    let mut slots = vec![Vec::new(); meshes];
    for mesh in &proxy.meshes {
        let Some(index) =
            (0..meshes).find(|&i| mesh.name.as_deref() == Some(proxy_name(i).as_str()))
        else {
            continue;
        };
        for primitive in &mesh.primitives {
            // Blender's exporter picks the color attribute a material reads; telling which,
            // this writer leaves to it.
            if primitive.attributes.keys().any(|s| matches!(s, Valid(Semantic::Colors(_)))) {
                return Err("a material reads a color attribute".into());
            }
        }
        slots[index] = mesh.primitives.iter().map(|p| p.material).collect();
    }

    let object = raw.as_object_mut().ok_or("the materials file isn't a glTF document")?;
    if let Some(serde_json::Value::Array(images)) = object.get_mut("images") {
        for image in images {
            let Some(view) = image.get("bufferView").and_then(|v| v.as_u64()) else {
                continue;
            };
            let view = proxy.buffer_views.get(view as usize).ok_or("an image has no data")?;
            let start = view.byte_offset.map_or(0, |o| o.0 as usize);
            let data = blob
                .get(start..start + view.byte_length.0 as usize)
                .ok_or("the materials file's image lies outside it")?;
            image["bufferView"] = serde_json::json!(glb.view(data.to_vec()).value());
        }
    }
    for key in ["materials", "textures", "samplers", "images"] {
        if let Some(value) = object.remove(key) {
            glb.raw.insert(key.into(), value);
        }
    }
    for name in &proxy.extensions_used {
        used(&mut glb.root.extensions_used, name);
    }
    for name in &proxy.extensions_required {
        used(&mut glb.root.extensions_required, name);
    }
    Ok(slots)
}

fn write_mesh(
    glb: &mut Glb,
    mesh: &MeshData,
    primitives: Vec<Primitive>,
    slots: &[Option<json::Index<json::Material>>],
) -> json::Mesh {
    let mut written = Vec::with_capacity(primitives.len());
    for primitive in primitives {
        let mut attributes = BTreeMap::new();
        let floats =
            |glb: &mut Glb, values: Vec<f32>, kind, bounds| glb.floats(&values, kind, bounds);
        let positions = primitive.positions.iter().flatten().copied().collect();
        attributes.insert(Valid(Semantic::Positions), floats(glb, positions, Type::Vec3, true));
        let normals = primitive.normals.iter().flatten().copied().collect();
        attributes.insert(Valid(Semantic::Normals), floats(glb, normals, Type::Vec3, false));
        for (set, uvs) in primitive.uvs.iter().enumerate() {
            let uvs = uvs.iter().flatten().copied().collect();
            attributes.insert(
                Valid(Semantic::TexCoords(set as u32)),
                floats(glb, uvs, Type::Vec2, false),
            );
        }
        if !primitive.tangents.is_empty() {
            let tangents = primitive.tangents.iter().flatten().copied().collect();
            attributes.insert(Valid(Semantic::Tangents), floats(glb, tangents, Type::Vec4, false));
        }
        if !primitive.joints.is_empty() {
            let joints: Vec<u32> = primitive.joints.iter().flatten().map(|&j| j as u32).collect();
            let component = if joints.iter().all(|&j| j < 256) {
                ComponentType::U8
            } else {
                ComponentType::U16
            };
            let joints = glb.integers(&joints, Type::Vec4, component);
            attributes.insert(Valid(Semantic::Joints(0)), joints);
            let weights = primitive.weights.iter().flatten().copied().collect();
            attributes.insert(Valid(Semantic::Weights(0)), floats(glb, weights, Type::Vec4, false));
        }
        let targets: Vec<json::mesh::MorphTarget> = primitive
            .targets
            .iter()
            .map(|target| {
                let positions = target.positions.iter().flatten().copied().collect();
                let normals: Vec<f32> = target.normals.iter().flatten().copied().collect();
                json::mesh::MorphTarget {
                    positions: Some(floats(glb, positions, Type::Vec3, true)),
                    normals: (!normals.is_empty()).then(|| floats(glb, normals, Type::Vec3, false)),
                    tangents: None,
                }
            })
            .collect();
        // 65535 and 4294967295 mean "no vertex" in glTF.
        let big = primitive.indices.iter().any(|&i| i >= 65535);
        let component = if big { ComponentType::U32 } else { ComponentType::U16 };
        let indices = glb.integers(&primitive.indices, Type::Scalar, component);
        written.push(json::mesh::Primitive {
            attributes,
            extensions: None,
            extras: Default::default(),
            indices: Some(indices),
            material: slots.get(primitive.slot as usize).copied().flatten(),
            mode: Valid(json::mesh::Mode::Triangles),
            targets: (!targets.is_empty()).then_some(targets),
        });
    }
    let names: Vec<&str> = mesh.shape_keys.iter().map(|k| k.name.as_str()).collect();
    let extras = (!names.is_empty()).then(|| {
        let names = serde_json::json!({ "targetNames": names });
        serde_json::value::to_raw_value(&names).expect("names always serialize")
    });
    json::Mesh {
        extensions: None,
        extras,
        name: Some(mesh.name.clone()),
        primitives: written,
        weights: (!names.is_empty()).then(|| mesh.shape_keys.iter().map(|k| k.weight).collect()),
    }
}

fn used(list: &mut Vec<String>, extension: &str) {
    if !list.iter().any(|e| e == extension) {
        list.push(extension.to_owned());
    }
}

// As the exporter's round_if_near: what is within float noise of the identity is the identity,
// and left out.
fn near(trs: Trs) -> Trs {
    let snap = |v: f32, to: f32| if (v - to).abs() > 2.0e-6 { v } else { to };
    let Trs { translation: t, rotation: r, scale: s } = trs;
    Trs {
        translation: t.map(|v| snap(v, 0.0)),
        rotation: Quat::from_xyzw(snap(r.x, 0.0), snap(r.y, 0.0), snap(r.z, 0.0), snap(r.w, 1.0)),
        scale: s.map(|v| snap(v, 1.0)),
    }
}

fn light(light: &Light) -> json::extensions::scene::khr_lights_punctual::Light {
    use json::extensions::scene::khr_lights_punctual as khr;
    let kind = match light.kind {
        LightKind::Point => khr::Type::Point,
        LightKind::Spot => khr::Type::Spot,
        LightKind::Directional => khr::Type::Directional,
    };
    khr::Light {
        color: light.color,
        extensions: None,
        extras: Default::default(),
        intensity: light.intensity,
        name: Some(light.name.clone()),
        range: light.range,
        spot: light
            .cone
            .map(|(inner, outer)| khr::Spot { inner_cone_angle: inner, outer_cone_angle: outer }),
        type_: Valid(kind),
    }
}

fn write_camera(camera: &Camera) -> json::Camera {
    let (perspective, orthographic, kind) = match camera.projection {
        Projection::Perspective { aspect_ratio, yfov, znear, zfar } => (
            Some(json::camera::Perspective {
                aspect_ratio: Some(aspect_ratio),
                yfov,
                zfar: Some(zfar),
                znear,
                extensions: None,
                extras: Default::default(),
            }),
            None,
            json::camera::Type::Perspective,
        ),
        Projection::Orthographic { xmag, ymag, znear, zfar } => (
            None,
            Some(json::camera::Orthographic {
                xmag,
                ymag,
                zfar,
                znear,
                extensions: None,
                extras: Default::default(),
            }),
            json::camera::Type::Orthographic,
        ),
    };
    json::Camera {
        name: Some(camera.name.clone()),
        orthographic,
        perspective,
        type_: Valid(kind),
        extensions: None,
        extras: Default::default(),
    }
}
