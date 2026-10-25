// Shape keys driven by bones, as Blender's drivers drive them. The exporter writes a skin's
// drivers next to it, in <skin>.drivers.json (charpaper-drivers reads and runs them); every
// frame, once the animation and gaze have posed the bones, each driver reads its bones and sets
// its shape key. So correctives follow whatever moves the bones: clips, blends between them,
// and gaze turning the neck and chest.
//
// A driver may read a bone the export left out, such as a pose reader's helper bone. It reads a
// stand-in instead: the nearest exported node above it, turned as the left-out bone sits from it
// in the rest pose, which is why the armature's rest pose is kept as it spawns.
//
// On binding, each driver is also worked out on the rest pose and compared with what Blender
// gave there, so a mismatch shows in the log once rather than as a subtly wrong shape.

use std::collections::HashMap;
use std::collections::HashSet;

use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_drivers::Driver;
use charpaper_drivers::DriverEntry;
use charpaper_drivers::DriversFile;
use charpaper_drivers::Source;
use charpaper_drivers::rotation_difference;
use charpaper_suite::MeshShapeKeys;
use charpaper_suite::Skin;

use crate::character::Armature;
use crate::character::ShownSkin;
use crate::character::SkinObjects;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

// How far a value may be from Blender's at rest, in radians for angles, before it's reported.
const TOLERANCE: f32 = 0.01;

// Every node of the armature by name, with its rest rotation in the armature's space: its bones,
// and the armature object's own node above them.
#[derive(Component, Default)]
pub(crate) struct BoneRest(HashMap<String, (Entity, Quat)>);

// On a skin's root, so its drivers go when it does.
#[derive(Component)]
pub(crate) struct ShapeKeyDrivers(Vec<Bound>);

struct Bound {
    // The node holding the object's shape key weights, and the key's index among them.
    target: (Entity, usize),
    driver: Driver,
    variables: Vec<BoundVariable>,
}

enum BoundVariable {
    RotationDifference([BoneSource; 2]),
}

// A bone the driver reads: one of the armature's, or a stand-in hanging from one at `offset`.
struct BoneSource {
    entity: Entity,
    offset: Quat,
    rest: Quat,
}

impl BoneSource {
    fn now(&self, globals: &Query<&GlobalTransform>) -> Option<Quat> {
        globals.get(self.entity).ok().map(|global| global.rotation() * self.offset)
    }
}

impl BoundVariable {
    fn now(&self, globals: &Query<&GlobalTransform>) -> Option<f32> {
        match self {
            BoundVariable::RotationDifference([a, b]) => {
                Some(rotation_difference(a.now(globals)?, b.now(globals)?))
            }
        }
    }

    fn at_rest(&self) -> f32 {
        match self {
            BoundVariable::RotationDifference([a, b]) => rotation_difference(a.rest, b.rest),
        }
    }
}

// Runs as the armature spawns, before any animation has touched it. Descendants come parents
// first, so each rotation builds on its parent's.
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
    let mut rotations: HashMap<Entity, Quat> = HashMap::new();
    let mut rest = BoneRest::default();
    for node in children.iter_descendants(armature) {
        let local = transforms.get(node).map_or(Quat::IDENTITY, |t| t.rotation);
        let parent = parents.get(node).ok().and_then(|p| rotations.get(&p.parent()).copied());
        let rotation = parent.unwrap_or(Quat::IDENTITY) * local;
        rotations.insert(node, rotation);
        if let Ok(name) = names.get(node) {
            rest.0.insert(name.to_string(), (node, rotation));
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
        let Some((_, rest)) = armatures.iter().find(|(a, _)| stage_of.get(*a) == Some(stage))
        else {
            continue;
        };
        let drivers = load(suite, skin, objects, rest);
        if !drivers.is_empty() {
            commands.entity(root).insert(ShapeKeyDrivers(drivers));
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

    let mut bound = Vec::new();
    let mut differ = 0;
    for entry in &file.drivers {
        let driven = format!("{} on {}", entry.key, entry.object);
        match bind(entry, &file, objects, &meshes, rest) {
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

fn bind(
    entry: &DriverEntry,
    file: &DriversFile,
    objects: &SkinObjects,
    meshes: &[MeshShapeKeys],
    rest: &BoneRest,
) -> Result<Bound, String> {
    let driver = Driver::new(entry)?;
    let entity = objects
        .0
        .iter()
        .find(|(name, _)| *name == entry.object)
        .map(|(_, entity)| *entity)
        .ok_or_else(|| format!("the skin has no object {:?}", entry.object))?;
    let index = meshes
        .iter()
        .find(|mesh| mesh.node == entry.object)
        .and_then(|mesh| mesh.keys.iter().position(|key| *key == entry.key))
        .ok_or_else(|| format!("{} has no shape key {:?}", entry.object, entry.key))?;
    let variables = entry
        .variables
        .iter()
        .map(|variable| match &variable.source {
            Source::RotationDifference { bones: [a, b] } => {
                Ok(BoundVariable::RotationDifference([
                    source(a, file, rest)?,
                    source(b, file, rest)?,
                ]))
            }
        })
        .collect::<Result<_, String>>()?;
    Ok(Bound { target: (entity, index), driver, variables })
}

fn source(name: &str, file: &DriversFile, rest: &BoneRest) -> Result<BoneSource, String> {
    if let Some(&(entity, rotation)) = rest.0.get(name) {
        return Ok(BoneSource { entity, offset: Quat::IDENTITY, rest: rotation });
    }
    let stand_in =
        file.stand_ins.get(name).ok_or_else(|| format!("the armature has no bone {name:?}"))?;
    let &(entity, parent_rest) = rest.0.get(&stand_in.parent).ok_or_else(|| {
        format!("{name:?} hangs from {:?}, which the armature lacks", stand_in.parent)
    })?;
    Ok(BoneSource {
        entity,
        offset: stand_in.offset(parent_rest),
        rest: stand_in.rest(parent_rest),
    })
}

// What differs from Blender in the rest pose, if anything.
fn check(bound: &Bound, entry: &DriverEntry) -> Option<String> {
    let expected = entry.rest.as_ref()?;
    let values: Vec<f32> = bound.variables.iter().map(BoundVariable::at_rest).collect();
    for ((variable, ours), blender) in entry.variables.iter().zip(&values).zip(&expected.variables)
    {
        if (ours - blender).abs() > TOLERANCE {
            let name = &variable.name;
            return Some(format!(
                "reads {name} = {ours:.3} at rest, where Blender reads {blender:.3}"
            ));
        }
    }
    let ours = bound.driver.evaluate(&values)?;
    ((ours - expected.value).abs() > TOLERANCE)
        .then(|| format!("gives {ours:.3} at rest, where Blender gives {:.3}", expected.value))
}

// After the transforms are propagated, so every bone's global rotation is this frame's final
// pose, gaze included. The renderer reads the weights later, while extracting.
pub(crate) fn drive_shape_keys(
    skins: Query<&ShapeKeyDrivers>,
    globals: Query<&GlobalTransform>,
    mut weights: Query<&mut MorphWeights>,
    mut values: Local<Vec<f32>>,
) {
    for drivers in &skins {
        'driver: for bound in &drivers.0 {
            values.clear();
            for variable in &bound.variables {
                let Some(value) = variable.now(&globals) else {
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
