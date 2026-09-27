use crate::configs::{self, join_inner};
use crate::fileio::{read_bin, read_textual, write_bin, write_textual};
use std::fs;
use std::path::Path;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

pub struct Report {
    pub obj_target: String,
    pub mtl_target: String,
    pub png_target: String,
}

fn is_directive(line: &str, key: &str) -> bool {
    line.trim_start()
        .strip_prefix(key)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

fn transform_obj(lines: Vec<String>, mtl_ref: &str) -> Result<Vec<String>, String> {
    match lines.iter().filter(|l| is_directive(l, "mtllib")).count() {
        0 => return Err("The OBJ has no mtllib declaration".into()),
        1 => {}
        n => return Err(format!("The OBJ has {n} mtllib declarations, expected exactly 1")),
    }
    Ok(lines
        .into_iter()
        .map(|l| if is_directive(&l, "mtllib") { format!("mtllib {mtl_ref}") } else { l })
        .collect())
}

fn transform_mtl(lines: Vec<String>, texture_ref: &str) -> Result<Vec<String>, String> {
    let unsupported: Vec<&str> = lines
        .iter()
        .filter(|l| l.trim_start().starts_with("map_") && !is_directive(l, "map_Kd"))
        .map(|l| l.trim())
        .collect();
    if !unsupported.is_empty() {
        return Err(format!(
            "The MTL has unsupported texture maps (only map_Kd is allowed):\n{}",
            unsupported.join("; ")
        ));
    }
    match lines.iter().filter(|l| is_directive(l, "map_Kd")).count() {
        0 => return Err("The MTL has no map_Kd declaration".into()),
        1 => {}
        n => return Err(format!("The MTL has {n} map_Kd declarations, expected exactly 1")),
    }
    Ok(lines
        .into_iter()
        .map(|l| if is_directive(&l, "map_Kd") { format!("map_Kd {texture_ref}") } else { l })
        .collect())
}

fn is_model_name(s: &str) -> bool {
    let b = s.as_bytes();
    match (b.first(), b.last()) {
        (Some(f), Some(l)) if f.is_ascii_lowercase() && l.is_ascii_lowercase() => {
            b.iter().all(|c| c.is_ascii_lowercase() || *c == b'_')
        }
        _ => false,
    }
}

pub fn validate_model_name(field: &str, s: &str) -> Result<(), String> {
    if is_model_name(s) {
        Ok(())
    } else {
        Err(format!(
            "{field}: {s:?} must start and end with a-z and contain only a-z and _"
        ))
    }
}

pub fn execute_for(
    config: &configs::Config,
    model_name: &str,
    model_type: &str,
    src_obj: &Path,
    src_mtl: &Path,
    src_png: &Path,
) -> Result<Report, String> {

    validate_model_name("model_name", model_name)?;
    let model_type_locp = config.model_types.get(model_type)
            .ok_or_else(|| format!("Unknown model type {}", model_type))?;
    let name = model_name;
    let png_bytes = read_bin(src_png)?;
    if !png_bytes.starts_with(&PNG_SIGNATURE) { return Err(format!("{} is not a PNG file", src_png.display())); }
    let obj_lines = read_textual(src_obj)?;
    let mtl_lines = read_textual(src_mtl)?;

    let obj_out = transform_obj(obj_lines, &format!("{name}.mtl"))?;
    let mtl_out = transform_mtl(mtl_lines,
        &format!("{}:{}/{}/{}", config.mod_id, config.png_rootf, model_type_locp, name))?;

    let obj_target = join_inner(config.objs_dir(), model_type_locp).join(format!("{name}.obj"));
    let mtl_target = join_inner(config.mtls_dir(), model_type_locp).join(format!("{name}.mtl"));
    let png_target = join_inner(config.pngs_dir(), model_type_locp).join(format!("{name}.png"));
    let targets = [&obj_target, &mtl_target, &png_target];

    let existing: Vec<String> = targets
        .iter()
        .filter(|p| p.exists())
        .map(|p| p.display().to_string())
        .collect();
    if !existing.is_empty() {
        return Err("Some target files already exist and will not be overwritten".into());
    }
    for t in targets {
        let parent = t.parent().ok_or("Target path has no parent directory")?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
    }
    let written = write_textual(&obj_target, obj_out, true)
        .and_then(|_| write_bin(&png_target, png_bytes, true))
        .and_then(|_| write_textual(&mtl_target, mtl_out, true));
    if let Err(e) = written {
        return Err(format!(
            "WARNING: the export stopped midway due to an I/O error ({e}), so the mod's assets may be in a corrupted or incomplete state. Check the target folders before continuing"
        ));
    }
    Ok(Report {
        obj_target: obj_target.display().to_string(),
        mtl_target: mtl_target.display().to_string(),
        png_target: png_target.display().to_string(),
    })
}