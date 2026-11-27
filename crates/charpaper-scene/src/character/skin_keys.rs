// The worn skin's shape keys, set by name from the character's mixer. Clips and expressions
// know shape keys only by name, so every mesh of the skin with a key of that name follows,
// and a skin without it just doesn't. A key no clip names is left alone, at its slider.

use std::collections::BTreeSet;

use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;

use crate::character::animation::CharacterActor;
use crate::character::binding::InstanceReady;
use crate::character::skin::SkinRoot;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

#[derive(Component)]
pub(crate) struct SkinKeys {
    meshes: Vec<SkinMesh>,
    pub names: BTreeSet<String>,
    // How many shape keys the rig had when `rig` below was last looked up: the rig grows as
    // clips load.
    bound: usize,
}

struct SkinMesh {
    entity: Entity,
    keys: Vec<Key>,
}

struct Key {
    name: String,
    morph: usize,
    // Its slider, as exported: where it rests while nothing keys it.
    rest: f32,
    rig: Option<usize>,
}

// Runs before binding moves the skin's branches onto the armature, while all its meshes are
// still under its root.
pub(crate) fn list_skin_keys(
    mut commands: Commands,
    skins: Query<(Entity, &SkinRoot), (With<InstanceReady>, Without<SkinKeys>)>,
    stage_of: StageOf,
    suites: Query<&ActiveSuite>,
    children: Query<&Children>,
    nodes: Query<(&Name, &MorphWeights)>,
) {
    for (root, skin) in &skins {
        let suite = stage_of.get(root).and_then(|stage| suites.get(stage).ok());
        let file = suite.and_then(|suite| {
            let skin = suite.skins.iter().find(|s| s.name == skin.name)?;
            Some((suite, &skin.file))
        });
        // Bevy drops shape key names with the mesh's RAM copy, so they come from the file.
        let found = match file.map(|(suite, file)| suite.shape_keys(file)) {
            Some(Ok(found)) => found,
            Some(Err(err)) => {
                warn!("skin {:?}: cannot read its shape key names: {err}", skin.name);
                Vec::new()
            }
            None => Vec::new(),
        };

        let mut meshes = Vec::new();
        for entity in children.iter_descendants(root) {
            let Ok((name, morph)) = nodes.get(entity) else {
                continue;
            };
            let Some(mesh) = found.iter().find(|mesh| mesh.node == name.as_str()) else {
                continue;
            };
            let keys = (mesh.keys.iter().enumerate())
                .filter(|(_, key)| !key.is_empty())
                .map(|(morph_at, key)| Key {
                    name: key.clone(),
                    morph: morph_at,
                    rest: morph.weights().get(morph_at).copied().unwrap_or(0.0),
                    rig: None,
                })
                .collect();
            meshes.push(SkinMesh { entity, keys });
        }
        let names = meshes.iter().flat_map(|m| m.keys.iter().map(|k| k.name.clone())).collect();
        commands.entity(root).insert(SkinKeys { meshes, names, bound: 0 });
    }
}

// After the character's mix, and before drivers and gaze, which win over keys.
pub(crate) fn write_skin_keys(
    armatures: Query<(Entity, &CharacterActor)>,
    stage_of: StageOf,
    mut skins: Query<(Entity, &mut SkinKeys)>,
    mut morphs: Query<&mut MorphWeights>,
) {
    for (skin, mut keys) in &mut skins {
        let stage = stage_of.get(skin);
        let Some((_, actor)) = armatures.iter().find(|(a, _)| stage_of.get(*a) == stage) else {
            continue;
        };
        let (rig, pose) = (&actor.mixer().rig().shape_keys, actor.mixer().pose());
        let keys = &mut *keys;
        if keys.bound != rig.len() {
            for key in keys.meshes.iter_mut().flat_map(|mesh| mesh.keys.iter_mut()) {
                key.rig = rig.iter().position(|(name, _)| *name == key.name);
            }
            keys.bound = rig.len();
        }

        for mesh in &keys.meshes {
            let Ok(mut morph) = morphs.get_mut(mesh.entity) else {
                continue;
            };
            let wanted = |key: &Key| {
                let at = key.rig?;
                Some((key.morph, pose.shape_keys[at] + key.rest * pose.open[at]))
            };
            // Read first, so a mesh whose keys are already right isn't marked changed.
            let differs = mesh.keys.iter().filter_map(wanted).any(|(morph_at, weight)| {
                morph.weights().get(morph_at).is_some_and(|&current| current != weight)
            });
            if differs {
                let weights = morph.weights_mut();
                for (morph_at, weight) in mesh.keys.iter().filter_map(wanted) {
                    if let Some(slot) = weights.get_mut(morph_at) {
                        *slot = weight;
                    }
                }
            }
        }
    }
}
