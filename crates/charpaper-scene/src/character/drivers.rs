// Shape keys driven by bones, as Blender's drivers drive them. The exporter writes a skin's
// drivers next to it, in <skin>.drivers.json (charpaper-drivers reads and runs them); every
// frame, once the animation and gaze have posed the bones, each driver reads its bones and sets
// its shape key. So correctives follow whatever moves the bones: clips, blends between them,
// and gaze turning the neck and chest.
//
// A driver may read a bone the export left out, such as a pose reader's helper bone. It reads a
// stand-in instead: the nearest exported node above it, turned as the left-out bone sits from it
// in the rest pose, which is why the armature's rest pose is kept as it spawns. The rest pose
// also gives each bone's pose against its rest, which local-space variables read.
//
// A driver reading another shape key runs after the driver of that key, if it has one, so it
// reads this frame's value.
//
// On binding, each driver is also worked out on the rest pose and compared with what Blender
// gave there, so a mismatch shows in the log once rather than as a subtly wrong shape.

use std::collections::HashMap;
use std::collections::HashSet;

use bevy::math::Affine3A;
use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_drivers::Axes;
use charpaper_drivers::Channel;
use charpaper_drivers::Driver;
use charpaper_drivers::DriverEntry;
use charpaper_drivers::DriversFile;
use charpaper_drivers::RotationMode;
use charpaper_drivers::Source;
use charpaper_drivers::Space;
use charpaper_drivers::channel;
use charpaper_drivers::rotation_difference;
use charpaper_suite::MeshShapeKeys;
use charpaper_suite::Skin;

use crate::character::Armature;
use crate::character::ShownSkin;
use crate::character::SkinObjects;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

// How far a value may be from Blender's at rest before it's reported: radians for angles,
// meters for lengths.
const TOLERANCE: f32 = 0.01;

// Every node of the armature by name, in the rest pose: its bones, and the armature object's
// own node above them.
#[derive(Component, Default)]
pub(crate) struct BoneRest(HashMap<String, RestNode>);

#[derive(Clone, Copy)]
struct RestNode {
    entity: Entity,
    // Against its parent, and in the armature's space.
    local: Affine3A,
    root: Affine3A,
}

// On a skin's root, so its drivers go when it does. In the order they run.
#[derive(Component)]
pub(crate) struct ShapeKeyDrivers {
    armature: Entity,
    drivers: Vec<Bound>,
}

struct Bound {
    // The node holding the object's shape key weights, and the key's index among them.
    target: (Entity, usize),
    driver: Driver,
    variables: Vec<BoundVariable>,
}

enum BoundVariable {
    RotationDifference([Node; 2]),
    TransformChannel { node: Node, channel: Channel, space: Space, rotation: RotationMode },
    Distance([(Node, Space); 2]),
    ShapeKey { target: (Entity, usize) },
}

// A bone a driver reads: one of the armature's, or a stand-in hanging still from one.
struct Node {
    entity: Entity,
    // Where the stand-in sits on `entity`; none for one of the armature's own bones.
    stand_in: Option<Affine3A>,
    rest: RestNode,
    // Ties the app's axes of the bone to Blender's, for variables that read numbers off it.
    axes: Option<Axes>,
}

// What the system reading the pose reaches.
struct Pose<'a, 'w, 's> {
    root: Affine3A,
    globals: &'a Query<'w, 's, &'static GlobalTransform>,
    transforms: &'a Query<'w, 's, &'static Transform>,
}

impl Node {
    // In the armature's space.
    fn now(&self, pose: &Pose) -> Option<Affine3A> {
        let global = pose.globals.get(self.entity).ok()?.affine();
        Some(pose.root * global * self.stand_in.unwrap_or(Affine3A::IDENTITY))
    }

    // Against its rest. A stand-in never moves against its parent.
    fn local_now(&self, pose: &Pose) -> Option<Affine3A> {
        if self.stand_in.is_some() {
            return Some(Affine3A::IDENTITY);
        }
        let local = pose.transforms.get(self.entity).ok()?.compute_affine();
        Some(self.rest.local.inverse() * local)
    }

    // As Blender has it, in the space asked for.
    fn in_blender(&self, space: Space, matrix: Affine3A) -> Option<Affine3A> {
        let axes = self.axes?;
        Some(match space {
            Space::World => axes.world(matrix),
            Space::Local => axes.local(matrix),
        })
    }

    fn blender_now(&self, space: Space, pose: &Pose) -> Option<Affine3A> {
        let matrix = match space {
            Space::World => self.now(pose)?,
            Space::Local => self.local_now(pose)?,
        };
        self.in_blender(space, matrix)
    }

    fn blender_at_rest(&self, space: Space) -> Option<Affine3A> {
        let matrix = match space {
            Space::World => self.rest.root,
            Space::Local => Affine3A::IDENTITY,
        };
        self.in_blender(space, matrix)
    }
}

fn rotation_of(matrix: Affine3A) -> Quat {
    matrix.to_scale_rotation_translation().1
}

impl BoundVariable {
    fn now(&self, pose: &Pose, weights: &Query<&mut MorphWeights>) -> Option<f32> {
        match self {
            BoundVariable::RotationDifference([a, b]) => {
                Some(rotation_difference(rotation_of(a.now(pose)?), rotation_of(b.now(pose)?)))
            }
            BoundVariable::TransformChannel { node, channel: which, space, rotation } => {
                Some(channel(node.blender_now(*space, pose)?, *which, *rotation))
            }
            BoundVariable::Distance([(a, a_space), (b, b_space)]) => {
                let a = a.blender_now(*a_space, pose)?.translation;
                let b = b.blender_now(*b_space, pose)?.translation;
                Some(a.distance(b))
            }
            BoundVariable::ShapeKey { target: (entity, index) } => {
                weights.get(*entity).ok()?.weights().get(*index).copied()
            }
        }
    }

    fn at_rest(&self) -> Option<f32> {
        match self {
            BoundVariable::RotationDifference([a, b]) => {
                Some(rotation_difference(rotation_of(a.rest.root), rotation_of(b.rest.root)))
            }
            BoundVariable::TransformChannel { node, channel: which, space, rotation } => {
                Some(channel(node.blender_at_rest(*space)?, *which, *rotation))
            }
            BoundVariable::Distance([(a, a_space), (b, b_space)]) => {
                let a = a.blender_at_rest(*a_space)?.translation;
                let b = b.blender_at_rest(*b_space)?.translation;
                Some(a.distance(b))
            }
            BoundVariable::ShapeKey { .. } => None,
        }
    }

    fn reads_key(&self) -> Option<(Entity, usize)> {
        match self {
            BoundVariable::ShapeKey { target } => Some(*target),
            _ => None,
        }
    }
}

// Runs as the armature spawns, before any animation has touched it. Descendants come parents
// first, so each pose builds on its parent's.
pub(crate) fn keep_rest_pose(
    ready: On<WorldInstanceReady>,
    armatures: Query<(), With<Armature>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
    mut commands: Commands,
) {
    let armature = ready.entity;
    if !armatures.contains(armature) {
        return;
    }
    let mut poses: HashMap<Entity, Affine3A> = HashMap::new();
    let mut rest = BoneRest::default();
    for node in children.iter_descendants(armature) {
        let local = transforms.get(node).map_or(Affine3A::IDENTITY, |t| t.compute_affine());
        let parent = parents.get(node).ok().and_then(|p| poses.get(&p.parent()).copied());
        let root = parent.unwrap_or(Affine3A::IDENTITY) * local;
        poses.insert(node, root);
        if let Ok(name) = names.get(node) {
            rest.0.insert(name.to_string(), RestNode { entity: node, local, root });
        }
    }
    commands.entity(armature).insert(rest);
}

// Once the worn skin's objects are known, which is after it has been bound to the armature.
pub(crate) fn bind_drivers(
    mut commands: Commands,
    stages: Query<(Entity, Option<&ActiveSuite>, &ShownSkin, &SkinObjects), Changed<SkinObjects>>,
    armatures: Query<(Entity, &BoneRest), With<Armature>>,
    stage_of: StageOf,
) {
    for (stage, suite, shown, objects) in &stages {
        let (Some(suite), Some(root), Some(name)) = (suite, shown.root, &shown.name) else {
            continue;
        };
        if objects.0.is_empty() {
            continue;
        }
        let Some(skin) = suite.skins.iter().find(|s| &s.name == name) else {
            continue;
        };
        let Some((armature, rest)) =
            armatures.iter().find(|(a, _)| stage_of.get(*a) == Some(stage))
        else {
            continue;
        };
        let drivers = load(suite, skin, objects, rest);
        if !drivers.is_empty() {
            commands.entity(root).insert(ShapeKeyDrivers { armature, drivers });
        }
    }
}

fn load(suite: &ActiveSuite, skin: &Skin, objects: &SkinObjects, rest: &BoneRest) -> Vec<Bound> {
    let file_path = suite.absolute(&skin.file);
    let path = file_path.with_extension("drivers.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            warn!("skin {:?}: cannot read {}: {err}", skin.name, path.display());
            return Vec::new();
        }
    };
    let file = match DriversFile::parse(&text) {
        Ok(file) => file,
        Err(err) => {
            warn!("skin {:?}: cannot read its drivers: {err}", skin.name);
            return Vec::new();
        }
    };
    let meshes = match charpaper_suite::mesh_shape_keys(&file_path) {
        Ok(meshes) => meshes,
        Err(err) => {
            warn!("skin {:?}: cannot read its shape keys: {err}", skin.name);
            return Vec::new();
        }
    };

    let skin_keys = SkinKeys { objects, meshes: &meshes };
    let mut bound = Vec::new();
    let mut differ = 0;
    for entry in &file.drivers {
        let driven = format!("{} on {}", entry.key, entry.object);
        match bind(entry, &file, &skin_keys, rest) {
            Ok(driver) => {
                if let Some(difference) = check(&driver, entry) {
                    warn!("skin {:?}: the driver of {driven} {difference}", skin.name);
                    differ += 1;
                }
                bound.push(driver);
            }
            Err(err) => warn!("skin {:?}: the driver of {driven} is left out: {err}", skin.name),
        }
    }
    let bound = in_running_order(bound);
    let objects: HashSet<Entity> = bound.iter().map(|b| b.target.0).collect();
    let verdict = match differ {
        0 => "all match Blender at rest".to_string(),
        n => format!("{n} differ from Blender at rest"),
    };
    info!(
        "skin {:?}: {} driven shape key(s) on {} mesh(es); {verdict}",
        skin.name,
        bound.len(),
        objects.len()
    );
    bound
}

// Where each object's shape keys are.
struct SkinKeys<'a> {
    objects: &'a SkinObjects,
    meshes: &'a [MeshShapeKeys],
}

impl SkinKeys<'_> {
    fn find(&self, object: &str, key: &str) -> Result<(Entity, usize), String> {
        let entity = self
            .objects
            .0
            .iter()
            .find(|(name, _)| name == object)
            .map(|(_, entity)| *entity)
            .ok_or_else(|| format!("the skin has no object {object:?}"))?;
        let index = self
            .meshes
            .iter()
            .find(|mesh| mesh.node == object)
            .and_then(|mesh| mesh.keys.iter().position(|k| k == key))
            .ok_or_else(|| format!("{object} has no shape key {key:?}"))?;
        Ok((entity, index))
    }
}

fn bind(
    entry: &DriverEntry,
    file: &DriversFile,
    keys: &SkinKeys,
    rest: &BoneRest,
) -> Result<Bound, String> {
    let driver = Driver::new(entry)?;
    let target = keys.find(&entry.object, &entry.key)?;
    let any_bone = |name: &str| node(name, file, rest);
    // Numbers read off a bone need its axes tied to Blender's.
    let with_axes = |name: &str| {
        let found = node(name, file, rest)?;
        match found.axes {
            Some(_) => Ok(found),
            None => Err(format!("the drivers file lacks {name:?}'s axes; export the skin again")),
        }
    };
    let variables = entry
        .variables
        .iter()
        .map(|variable| {
            Ok(match &variable.source {
                Source::RotationDifference { bones: [a, b] } => {
                    BoundVariable::RotationDifference([any_bone(a)?, any_bone(b)?])
                }
                Source::TransformChannel { bone, channel, space, rotation } => {
                    BoundVariable::TransformChannel {
                        node: with_axes(bone)?,
                        channel: *channel,
                        space: *space,
                        rotation: *rotation,
                    }
                }
                Source::Distance { bones: [a, b], spaces: [a_space, b_space] } => {
                    BoundVariable::Distance([(with_axes(a)?, *a_space), (with_axes(b)?, *b_space)])
                }
                Source::ShapeKey { object, key } => {
                    BoundVariable::ShapeKey { target: keys.find(object, key)? }
                }
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Bound { target, driver, variables })
}

fn node(name: &str, file: &DriversFile, rest: &BoneRest) -> Result<Node, String> {
    let (entity, stand_in, node_rest) = match rest.0.get(name) {
        Some(found) => (found.entity, None, *found),
        None => {
            let stand_in = file
                .stand_ins
                .get(name)
                .ok_or_else(|| format!("the armature has no bone {name:?}"))?;
            let parent = rest.0.get(&stand_in.parent).ok_or_else(|| {
                format!("{name:?} hangs from {:?}, which the armature lacks", stand_in.parent)
            })?;
            let (_, parent_rotation, parent_head) = parent.root.to_scale_rotation_translation();
            let head = stand_in.head.map_or(parent_head, Vec3::from_array);
            let root = Affine3A::from_rotation_translation(stand_in.rest(parent_rotation), head);
            let on_parent = parent.root.inverse() * root;
            (parent.entity, Some(on_parent), RestNode { entity: parent.entity, local: root, root })
        }
    };
    let axes = file.bones.get(name).map(|bone| {
        Axes::new(Quat::from_array(bone.rest).normalize(), rotation_of(node_rest.root))
    });
    Ok(Node { entity, stand_in, rest: node_rest, axes })
}

// Drivers that set a shape key go before those that read it. A loop keeps the file's order.
fn in_running_order(mut pending: Vec<Bound>) -> Vec<Bound> {
    let mut ordered = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let unset: HashSet<(Entity, usize)> = pending.iter().map(|b| b.target).collect();
        let ready = pending
            .iter()
            .position(|b| {
                b.variables.iter().filter_map(BoundVariable::reads_key).all(|k| !unset.contains(&k))
            })
            .unwrap_or(0);
        ordered.push(pending.remove(ready));
    }
    ordered
}

// What differs from Blender in the rest pose, if anything.
fn check(bound: &Bound, entry: &DriverEntry) -> Option<String> {
    let expected = entry.rest.as_ref()?;
    let values: Vec<Option<f32>> = bound.variables.iter().map(BoundVariable::at_rest).collect();
    for ((variable, ours), blender) in entry.variables.iter().zip(&values).zip(&expected.variables)
    {
        if let (Some(ours), Some(blender)) = (ours, blender)
            && (ours - blender).abs() > TOLERANCE
        {
            let name = &variable.name;
            return Some(format!(
                "reads {name} = {ours:.3} at rest, where Blender reads {blender:.3}"
            ));
        }
    }
    // Only when every variable could be worked out at rest.
    let values: Option<Vec<f32>> = values.into_iter().collect();
    let ours = bound.driver.evaluate(&values?)?;
    ((ours - expected.value).abs() > TOLERANCE)
        .then(|| format!("gives {ours:.3} at rest, where Blender gives {:.3}", expected.value))
}

// After the transforms are propagated, so every bone's global transform is this frame's final
// pose, gaze included. The renderer reads the weights later, while extracting.
pub(crate) fn drive_shape_keys(
    skins: Query<&ShapeKeyDrivers>,
    globals: Query<&'static GlobalTransform>,
    transforms: Query<&'static Transform>,
    mut weights: Query<&mut MorphWeights>,
    mut values: Local<Vec<f32>>,
) {
    for skin in &skins {
        let Ok(armature) = globals.get(skin.armature) else {
            continue;
        };
        let pose =
            Pose { root: armature.affine().inverse(), globals: &globals, transforms: &transforms };
        'driver: for bound in &skin.drivers {
            values.clear();
            for variable in &bound.variables {
                let Some(value) = variable.now(&pose, &weights) else {
                    continue 'driver;
                };
                values.push(value);
            }
            let Some(value) = bound.driver.evaluate(&values) else {
                continue;
            };
            let (entity, index) = bound.target;
            if let Ok(mut morphs) = weights.get_mut(entity)
                && let Some(slot) = morphs.weights_mut().get_mut(index)
            {
                *slot = value;
            }
        }
    }
}
