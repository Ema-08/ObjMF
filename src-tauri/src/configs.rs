/*
Config JSON schema:

ROOT_OBJECT:
 - mod_id (String, required): the mod's ID (must match non-empty [a-z])
 - resources_path (Path, required): the mod's resources folder (usually points to the equivalent of gradle's ./src/main/resources)
 - obj_rootf (String, required): the folder in which all the objs go (relative to the mod's namespaced assets dir, must be a valid INNER_PATH)
 - mtl_rootf (String, required): the folder in which all the mtls go (relative to the mod's namespaced assets dir, must be a valid INNER_PATH)
 - png_rootf (String, required): the folder in which all the textures go (relative to the mod's namespaced assets dir joined with "textures" inner folder, must be a valid INNER_PATH)
 - model_types (Object, required, can be empty)
    Keys must be valid model names (must match non-empty [A-Z] || "_", with no trailing or leading "_")
    Values must be valid INNER_PATH, used relatively to *_rootf paths

INNER_PATH specification: must match [a-z] || "/", with no trailing, leading, or double "/".
*/

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use crate::fileio::read_bin;

pub struct Config {
    pub mod_id: String,
    pub resources_path: Box<Path>,
    pub obj_rootf: String,
    pub mtl_rootf: String,
    pub png_rootf: String,
    pub model_types: BTreeMap<String, String>,
}

impl Config {

    pub fn assets_dir(&self) -> PathBuf {
        self.resources_path.join("assets").join(&self.mod_id)
    }

    pub fn objs_dir(&self) -> PathBuf {
        join_inner(self.assets_dir(), &self.obj_rootf)
    }

    pub fn mtls_dir(&self) -> PathBuf {
        join_inner(self.assets_dir(), &self.mtl_rootf)
    }

    pub fn pngs_dir(&self) -> PathBuf {
        join_inner(self.assets_dir().join("textures"), &self.png_rootf)
    }

}

pub fn join_inner(mut base: PathBuf, inner: &str) -> PathBuf {
    base.extend(inner.split('/'));
    base
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    mod_id: String,
    resources_path: String,
    obj_rootf: String,
    mtl_rootf: String,
    png_rootf: String,
    model_types: BTreeMap<String, String>,
}

fn is_lower_word(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase())
}

pub fn validate_word(field: &str, s: &str) -> Result<(), String> {
    if is_lower_word(s) {
        Ok(())
    } else {
        Err(format!("{field}: {s:?} must be non-empty and contain only a-z"))
    }
}

fn is_type_key(s: &str) -> bool {
    let b = s.as_bytes();
    match (b.first(), b.last()) {
        (Some(f), Some(l)) if f.is_ascii_uppercase() && l.is_ascii_uppercase() => {
            b.iter().all(|c| c.is_ascii_uppercase() || *c == b'_')
        }
        _ => false,
    }
}

fn validate_type_key(field: &str, s: &str) -> Result<(), String> {
    if is_type_key(s) {
        Ok(())
    } else {
        Err(format!(
            "{field}: {s:?} must start and end with A-Z and contain only A-Z and _"
        ))
    }
}

fn validate_inner_path(field: &str, s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(format!("{field}: path must not be empty"));
    }
    for seg in s.split('/') {
        if seg.is_empty() {
            return Err(format!("{field}: {s:?} has a leading, trailing or double '/'"));
        }
        if !is_lower_word(seg) {
            return Err(format!("{field}: segment {seg:?} in {s:?} must contain only a-z"));
        }
    }
    Ok(())
}

fn normalize_dir(base: &Path, configured: &str) -> Result<PathBuf, String> {
    if configured.trim().is_empty() {
        return Err("resources_path must not be empty".into());
    }
    let joined = base.join(configured);
    let canonical = dunce::canonicalize(&joined).map_err(|e| {
        format!(
            "resources_path {configured:?} (resolved to {}) is not accessible: {e}",
            joined.display()
        )
    })?;
    if !canonical.is_dir() {
        return Err(format!("resources_path is not a directory: {}", canonical.display()));
    }
    Ok(canonical)
}

pub fn load_config_with_base(path: &Path, base: &Path) -> Result<Config, String> {
    let mut bytes = read_bin(path)?;
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(..3);
    }

    let raw: RawConfig = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Invalid config {}: {e}", path.display()))?;

    validate_word("mod_id", &raw.mod_id)?;
    validate_inner_path("obj_rootf", &raw.obj_rootf)?;
    validate_inner_path("mtl_rootf", &raw.mtl_rootf)?;
    validate_inner_path("png_rootf", &raw.png_rootf)?;
    for (name, inner) in &raw.model_types {
        validate_type_key(&format!("model_types key {name:?}"), name)?;
        validate_inner_path(&format!("model_types[{name:?}]"), inner)?;
    }

    let resources_path = normalize_dir(base, &raw.resources_path)?.into_boxed_path();

    Ok(Config {
        mod_id: raw.mod_id,
        resources_path,
        obj_rootf: raw.obj_rootf,
        mtl_rootf: raw.mtl_rootf,
        png_rootf: raw.png_rootf,
        model_types: raw.model_types,
    })
}