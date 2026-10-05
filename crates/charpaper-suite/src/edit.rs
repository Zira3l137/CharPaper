use std::fs;
use std::path::Path;

use toml_edit::DocumentMut;
use toml_edit::Item;
use toml_edit::Table;
use toml_edit::Value;

use crate::error::SuiteError;
use crate::layout::Look;
use crate::manifest::MANIFEST_FILE;

// The author's suite.toml is copied here before the first edit and never overwritten, so
// restoring always brings back what the author shipped. Delete it when updating a suite.
pub const BACKUP_FILE: &str = "suite.toml.bak";

pub fn has_backup(root: &Path) -> bool {
    root.join(BACKUP_FILE).is_file()
}

pub fn save_look(root: &Path, look: &Look) -> Result<bool, SuiteError> {
    let path = root.join(MANIFEST_FILE);
    let text = fs::read_to_string(&path).map_err(|source| io(&path, source))?;
    let mut doc: DocumentMut =
        text.parse().map_err(|source| SuiteError::Edit { path: path.clone(), source })?;

    let post = &look.post;
    let mut changed = false;
    changed |= set(&mut doc, &["post"], "tonemapping", post.tonemapping.map(|t| t.as_str().into()));
    changed |= set(&mut doc, &["post"], "exposure", post.exposure.map(float));
    changed |= set(&mut doc, &["post"], "bloom", post.bloom.map(float));
    changed |=
        set(&mut doc, &["post"], "chromatic_aberration", post.chromatic_aberration.map(float));
    let vignette = &post.vignette;
    changed |= set(&mut doc, &["post", "vignette"], "intensity", vignette.intensity.map(float));
    changed |= set(&mut doc, &["post", "vignette"], "size", vignette.size.map(float));
    changed |= set(&mut doc, &["post", "grain"], "intensity", post.grain.intensity.map(float));
    changed |= set(&mut doc, &["post", "grain"], "size", post.grain.size.map(float));
    let grading = &post.grading;
    for (key, value) in [
        ("warmth", grading.warmth),
        ("tint", grading.tint),
        ("saturation", grading.saturation),
        ("contrast", grading.contrast),
    ] {
        changed |= set(&mut doc, &["post", "grading"], key, value.map(float));
    }
    let lut = &post.lut;
    changed |= set(&mut doc, &["post", "lut"], "name", lut.name.as_deref().map(Value::from));
    changed |= set(&mut doc, &["post", "lut"], "strength", lut.strength.map(float));
    for (name, entry) in &look.environments {
        let table = ["environments", name.as_str()];
        changed |= set(&mut doc, &table, "brightness", entry.brightness.map(float));
        changed |= set(&mut doc, &table, "shadows", entry.shadows.map(Value::from));
        changed |= set(&mut doc, &table, "exposure", entry.exposure.map(float));
    }
    if !changed {
        return Ok(false);
    }

    let backup = root.join(BACKUP_FILE);
    if !backup.is_file() {
        fs::copy(&path, &backup).map_err(|source| io(&backup, source))?;
    }
    write(&path, &doc.to_string())?;
    Ok(true)
}

pub fn restore_look(root: &Path) -> Result<bool, SuiteError> {
    let backup = root.join(BACKUP_FILE);
    if !backup.is_file() {
        return Ok(false);
    }
    let path = root.join(MANIFEST_FILE);
    fs::rename(&backup, &path).map_err(|source| io(&path, source))?;
    Ok(true)
}

fn set(doc: &mut DocumentMut, tables: &[&str], key: &str, value: Option<Value>) -> bool {
    let Some(mut value) = value else {
        return false;
    };
    let mut table = doc.as_table_mut() as &mut dyn toml_edit::TableLike;
    for (depth, name) in tables.iter().enumerate() {
        let item = table.entry(name).or_insert_with(|| {
            let mut new = Table::new();
            // Only the innermost table gets a header; an empty [environments] would be noise.
            new.set_implicit(depth + 1 < tables.len());
            Item::Table(new)
        });
        let Some(next) = item.as_table_like_mut() else {
            return false;
        };
        table = next;
    }

    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(old) if same(old, &value) => false,
        Some(old) => {
            // Keeps a trailing comment on the line.
            *value.decor_mut() = old.decor().clone();
            *old = value;
            true
        }
        None => {
            table.insert(key, Item::Value(value));
            true
        }
    }
}

// An author may write 1000 where the app writes 1000.0.
fn same(old: &Value, new: &Value) -> bool {
    let number = |v: &Value| v.as_float().or_else(|| v.as_integer().map(|i| i as f64));
    match (number(old), number(new)) {
        (Some(a), Some(b)) => (a - b).abs() < 1e-6,
        _ => old.to_string().trim() == new.to_string().trim(),
    }
}

// Via the shortest decimal form, so 0.3 isn't written as 0.30000001192092896.
fn float(value: f32) -> Value {
    Value::from(value.to_string().parse::<f64>().unwrap_or(value as f64))
}

// Written to a temporary file and renamed, so a crash mid-write can't truncate it.
fn write(path: &Path, text: &str) -> Result<(), SuiteError> {
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, text).map_err(|source| io(&temporary, source))?;
    fs::rename(&temporary, path).map_err(|source| io(path, source))
}

fn io(path: &Path, source: std::io::Error) -> SuiteError {
    SuiteError::Io { path: path.to_path_buf(), source }
}
