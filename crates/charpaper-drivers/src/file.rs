use std::collections::BTreeMap;

use glam::Quat;
use serde::Deserialize;

use crate::curve::Curve;
use crate::transform::Channel;
use crate::transform::RotationMode;
use crate::transform::Space;

/// What the exporter writes as `<skin>.drivers.json`. Rotations are in glTF's axes, the way the
/// model is exported: Blender's world, with +Y up.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DriversFile {
    pub version: u32,
    #[serde(default)]
    pub drivers: Vec<DriverEntry>,
    /// Bones the drivers read that the export leaves out, by name.
    #[serde(default)]
    pub stand_ins: BTreeMap<String, StandIn>,
    /// The bones Transform Channel and Distance variables read, by name, with their rest
    /// rotation as Blender has it, which ties the app's axes of each to Blender's.
    #[serde(default)]
    pub bones: BTreeMap<String, BoneAxes>,
}

impl DriversFile {
    pub const VERSION: u32 = 1;

    pub fn parse(text: &str) -> Result<Self, String> {
        let file: Self = serde_json::from_str(text).map_err(|err| err.to_string())?;
        if file.version != Self::VERSION {
            return Err(format!(
                "written for version {} of the format; this app reads version {}",
                file.version,
                Self::VERSION
            ));
        }
        Ok(file)
    }
}

/// One driven shape key of one object.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DriverEntry {
    /// The object's name, as glTF names its node.
    pub object: String,
    pub key: String,
    pub kind: Kind,
    /// A scripted driver's formula; empty for the others.
    #[serde(default)]
    pub expression: String,
    pub variables: Vec<Variable>,
    /// Maps the driver's value to the shape key's; none passes it on as it is.
    pub curve: Option<Curve>,
    /// The shape key's slider range, which Blender clamps the value to.
    pub range: [f32; 2],
    /// What the driver gives in the rest pose in Blender, for the app to check itself against.
    pub rest: Option<RestCheck>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Scripted,
    Average,
    Sum,
    Min,
    Max,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Variable {
    pub name: String,
    #[serde(flatten)]
    pub source: Source,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// The angle between two bones' rotations, in radians from 0 to π.
    RotationDifference { bones: [String; 2] },
    /// One number of a bone's location, rotation or scale. A Single Property variable reading
    /// a bone's location, rotation or scale is one of these too, in local space.
    TransformChannel { bone: String, channel: Channel, space: Space, rotation: RotationMode },
    /// How far apart two bones' heads are, each in its own space.
    Distance { bones: [String; 2], spaces: [Space; 2] },
    /// Another shape key's value, which a Single Property variable reads.
    ShapeKey { object: String, key: String },
}

/// A bone the export leaves out, such as a pose reader's helper bone, which keeps still
/// relative to `parent`, the nearest exported node above it.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct StandIn {
    pub parent: String,
    /// The turn from the parent's rest rotation to the bone's, in world axes, as x, y, z, w.
    pub turn: [f32; 4],
    /// Where the bone's head is in the rest pose, for variables that read where it is.
    #[serde(default)]
    pub head: Option<[f32; 3]>,
}

impl StandIn {
    /// The bone's rest rotation, from its parent's.
    pub fn rest(&self, parent_rest: Quat) -> Quat {
        (Quat::from_array(self.turn).normalize() * parent_rest).normalize()
    }

    /// The bone's rotation relative to its parent's, which stays as it is in the rest pose.
    pub fn offset(&self, parent_rest: Quat) -> Quat {
        (parent_rest.inverse() * self.rest(parent_rest)).normalize()
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct BoneAxes {
    /// In Blender's own axes, as x, y, z, w.
    pub rest: [f32; 4],
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct RestCheck {
    /// Each variable's value, in order; none for one the exporter can't work out at rest, such
    /// as another shape key's value.
    pub variables: Vec<Option<f32>>,
    pub value: f32,
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use glam::EulerRot;

    use super::*;

    #[test]
    fn reads_what_the_exporter_writes() {
        let text = r#"{
            "version": 1,
            "drivers": [{
                "object": "Body", "key": "C_ArmUp_L", "kind": "scripted",
                "expression": "max(0.0, 1.0 - d / 2.07445)",
                "variables": [{"name": "d", "type": "rotation_difference",
                               "bones": ["upper_arm.L", "MCH-reader_C_ArmUp_L"]}],
                "curve": {"from": 0.0, "to": 1.0, "values": [0.0, 1.0],
                          "slope_before": 0.0, "slope_after": 0.0},
                "range": [0.0, 1.0],
                "rest": {"variables": [2.07445, null], "value": 0.0}
            }],
            "stand_ins": {"MCH-reader_C_ArmUp_L": {"parent": "shoulder.L", "turn": [0, 0, 0, 1]}}
        }"#;
        let file = DriversFile::parse(text).unwrap();
        assert_eq!(file.drivers[0].kind, Kind::Scripted);
        assert_eq!(file.stand_ins["MCH-reader_C_ArmUp_L"].parent, "shoulder.L");
    }

    #[test]
    fn reads_every_variable_type() {
        let text = r#"{
            "version": 1,
            "drivers": [{
                "object": "Body", "key": "K", "kind": "sum", "curve": null, "range": [0, 1],
                "variables": [
                    {"name": "a", "type": "transform_channel", "bone": "shin.L",
                     "channel": "rot_x", "space": "local", "rotation": "swing_twist_y"},
                    {"name": "b", "type": "distance", "bones": ["hand.L", "hip.L"],
                     "spaces": ["world", "world"]},
                    {"name": "c", "type": "shape_key", "object": "Body", "key": "Blink"}
                ]
            }],
            "bones": {"shin.L": {"rest": [0, 0, 0, 1]}}
        }"#;
        let file = DriversFile::parse(text).unwrap();
        let sources: Vec<&Source> = file.drivers[0].variables.iter().map(|v| &v.source).collect();
        assert!(matches!(
            sources[0],
            Source::TransformChannel {
                channel: Channel::RotX,
                rotation: RotationMode::SwingTwistY,
                ..
            }
        ));
        assert!(matches!(
            sources[1],
            Source::Distance { spaces: [Space::World, Space::World], .. }
        ));
        assert!(matches!(sources[2], Source::ShapeKey { .. }));
        assert_eq!(file.bones["shin.L"].rest, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn refuses_another_version() {
        assert!(DriversFile::parse(r#"{"version": 2}"#).is_err());
    }

    // The exporter works out a stand-in's turn in Blender's world and writes it in glTF's axes
    // by swapping its components: (x, y, z) becomes (x, z, -y). The app's bones may also differ
    // from Blender's by how the glTF exporter turns each bone's own axes.
    #[test]
    fn stand_in_matches_the_bone_it_replaces() {
        let parent = Quat::from_euler(EulerRot::XYZ, 0.3, -0.8, 1.2);
        let reader = Quat::from_euler(EulerRot::XYZ, -1.1, 0.4, 0.2);
        let turn = reader * parent.inverse();
        let to_gltf = Quat::from_rotation_x(-FRAC_PI_2);
        let own_axes = Quat::from_euler(EulerRot::XYZ, 0.7, 0.1, -0.5);
        let app = |q: Quat| to_gltf * q * own_axes;

        let stand_in =
            StandIn { parent: String::new(), turn: [turn.x, turn.z, -turn.y, turn.w], head: None };
        assert!(stand_in.rest(app(parent)).angle_between(app(reader)) < 1e-4);

        // Hanging still from its parent, it follows wherever the parent turns.
        let moved = Quat::from_euler(EulerRot::XYZ, 0.9, 0.2, -0.3) * parent;
        let followed = app(moved) * stand_in.offset(app(parent));
        let expected = app(moved * parent.inverse() * reader);
        assert!(followed.angle_between(expected) < 1e-4);
    }
}
