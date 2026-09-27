mod configs;
mod fileio;
mod processor;

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;

const CONFIG_FILE_NAME: &str = "objmf_config.json";

struct AppState {
    config: Result<configs::Config, String>,
}

/* Debug builds may override it with OBJMF_DIR. */
fn app_dir() -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        if let Ok(dir) = std::env::var("OBJMF_DIR") {
            return Ok(PathBuf::from(dir));
        }
    }
    std::env::current_exe()
        .map_err(|e| format!("Cannot locate executable: {e}"))?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Executable has no parent directory".to_string())
}

fn load() -> Result<configs::Config, String> {
    let dir = app_dir()?;
    configs::load_config_with_base(&dir.join(CONFIG_FILE_NAME), &dir)
}

#[derive(Serialize)]
struct TypeInfo {
    key: String,
    path: String,
}

#[derive(Serialize)]
struct AppInfo {
    mod_id: String,
    model_types: Vec<TypeInfo>,
}

#[derive(Serialize)]
struct ExportResult {
    obj_target: String,
    mtl_target: String,
    png_target: String,
}

#[tauri::command]
fn get_app_info(state: State<'_, AppState>) -> Result<AppInfo, String> {
    let cfg = state.config.as_ref().map_err(|e| e.clone())?;
    Ok(AppInfo {
        mod_id: cfg.mod_id.clone(),
        model_types: cfg
            .model_types
            .iter()
            .map(|(k, v)| TypeInfo { key: k.clone(), path: v.clone() })
            .collect(),
    })
}

#[tauri::command(async)]
fn export_model(
    state: State<'_, AppState>,
    model_name: String,
    model_type: String,
    obj: String,
    mtl: String,
    png: String,
) -> Result<ExportResult, String> {
    let cfg = state.config.as_ref().map_err(|e| e.clone())?;
    let r = processor::execute_for(
        cfg,
        &model_name,
        &model_type,
        Path::new(&obj),
        Path::new(&mtl),
        Path::new(&png),
    )?;
    Ok(ExportResult {
        obj_target: r.obj_target,
        mtl_target: r.mtl_target,
        png_target: r.png_target,
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { config: load() })
        .invoke_handler(tauri::generate_handler![get_app_info, export_model])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}