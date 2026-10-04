mod animations;
mod cameras;
mod environments;
mod gaze;
mod skins;
mod tree;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use crate::document::read_document;
use crate::inspect::animations::check_animations;
use crate::inspect::cameras::check_camera_file;
use crate::inspect::cameras::check_lens;
use crate::inspect::cameras::check_orbit_lens;
use crate::inspect::environments::check_environments;
use crate::inspect::gaze::check_gaze;
use crate::inspect::skins::check_skin_expressions;
use crate::inspect::skins::check_skin_file;
use crate::inspect::skins::model_bones;
use crate::inspect::tree::Gltf;
use crate::inspect::tree::index;
use crate::layout::Skin;
use crate::layout::Suite;

// Opens the suite's glTF files and checks that the pieces fit together the way the scene
// will use them. Problems a suite survives are findings; SuiteError is for the rest.

const EXAMPLES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub file: Option<PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.count(Severity::Error)
    }

    pub fn warnings(&self) -> usize {
        self.count(Severity::Warning)
    }

    fn count(&self, severity: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == severity).count()
    }

    fn push(&mut self, severity: Severity, file: Option<&Path>, message: String) {
        self.findings.push(Finding { severity, file: file.map(Path::to_path_buf), message });
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        match &self.file {
            Some(file) => write!(f, "{label:<7} {}: {}", file.display(), self.message),
            None => write!(f, "{label:<7} {}", self.message),
        }
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for finding in &self.findings {
            writeln!(f, "{finding}")?;
        }
        write!(f, "{} error(s), {} warning(s)", self.errors(), self.warnings())
    }
}

pub fn inspect(suite: &Suite) -> Report {
    let mut report = Report::default();

    let Some(model) = open(suite, &suite.model, &mut report) else {
        return report;
    };
    let bones = model_bones(&model, &suite.model, &mut report);

    // A borrowed armature's meshes are its skin's own.
    let meshes = model.doc.nodes().filter(|n| n.mesh().is_some()).count();
    if !suite.model_is_skin && meshes == 0 && suite.skins.is_empty() {
        let message = "nothing to show: the model has no meshes and the suite has no skins";
        report.push(Severity::Error, None, message.to_string());
    } else if !suite.model_is_skin && meshes > 0 && !suite.skins.is_empty() {
        let message = format!(
            "{meshes} mesh(es) are shown under every skin; move them into the skins that \
             should own them"
        );
        report.push(Severity::Warning, Some(&suite.model), message);
    }

    let unnamed = model.doc.nodes().filter(|n| n.name().is_none()).count();
    if unnamed > 0 {
        let message = format!(
            "{unnamed} node(s) have no name; Bevy names them by index, which other files \
             cannot match"
        );
        report.push(Severity::Warning, Some(&suite.model), message);
    }

    let skins: Vec<(&Skin, Gltf)> = suite
        .skins
        .iter()
        .filter_map(|skin| Some((skin, open(suite, &skin.file, &mut report)?)))
        .collect();
    check_animations(suite, &model, &skins, &mut report);
    for (skin, gltf) in &skins {
        check_skin_file(&skin.file, gltf, &model, &bones, &mut report);
        check_skin_expressions(&skin.file, gltf, &mut report);
    }
    let camera_files: BTreeSet<&Path> = suite.cameras.iter().map(|c| c.file.as_path()).collect();
    let camera_files: BTreeMap<&Path, Gltf> = camera_files
        .into_iter()
        .filter_map(|file| Some((file, open(suite, file, &mut report)?)))
        .collect();
    for (file, gltf) in &camera_files {
        check_camera_file(file, gltf, &mut report);
    }
    for camera in &suite.cameras {
        let file = camera_files.get(camera.file.as_path());
        check_lens(camera, file, &bones, &skins, &mut report);
    }
    check_orbit_lens(&suite.camera, &mut report);

    check_environments(suite, &mut report);
    if let Some(gaze) = &suite.gaze {
        check_gaze(gaze, &suite.model, &bones, &skins, &mut report);
    }

    report.findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    report
}

fn open(suite: &Suite, file: &Path, report: &mut Report) -> Option<Gltf> {
    match read_document(&suite.absolute(file)) {
        Ok(doc) => Some(index(doc)),
        Err(message) => {
            report.push(Severity::Error, Some(file), message);
            None
        }
    }
}

fn describe_clips(clips: &[Option<&str>]) -> String {
    if clips.is_empty() {
        return "none".to_string();
    }
    let names: Vec<&str> = clips.iter().map(|c| c.unwrap_or("<unnamed>")).collect();
    names.join(", ")
}

fn examples(items: &[&str]) -> String {
    let shown = items.iter().take(EXAMPLES).copied().collect::<Vec<_>>().join(", ");
    match items.len().saturating_sub(EXAMPLES) {
        0 => shown,
        more => format!("{shown} and {more} more"),
    }
}
