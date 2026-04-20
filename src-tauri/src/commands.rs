//! Tauri commands exposed to the TypeScript frontend.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::registry::convert;
use crate::util::{unique_sibling_path, ExtExt};

#[derive(Serialize)]
pub struct FileMeta {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Serialize)]
pub struct ConversionResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ConversionResult {
    fn ok(path: PathBuf) -> Self {
        Self {
            ok: true,
            output_path: Some(path.to_string_lossy().into_owned()),
            error: None,
        }
    }

    fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            output_path: None,
            error: Some(msg.into()),
        }
    }
}

/// Read basic metadata for the given paths. Missing files are skipped
/// silently so a partial drop still gets registered.
#[tauri::command]
pub fn get_file_meta(paths: Vec<String>) -> Vec<FileMeta> {
    paths
        .into_iter()
        .filter_map(|p| {
            let path = PathBuf::from(&p);
            let meta = std::fs::metadata(&path).ok()?;
            if !meta.is_file() {
                return None;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.clone());
            Some(FileMeta {
                path: path.to_string_lossy().into_owned(),
                name,
                size_bytes: meta.len(),
            })
        })
        .collect()
}

/// Convert a single file to the requested target extension.
#[tauri::command]
pub fn convert_file(input_path: String, target_ext: String) -> ConversionResult {
    let input = PathBuf::from(&input_path);
    let source_ext = match input.ext_lower() {
        Some(e) => e,
        None => return ConversionResult::err("Input file has no extension."),
    };
    let target_ext = target_ext.to_lowercase();

    if !input.exists() {
        return ConversionResult::err("Input file no longer exists.");
    }

    let output = match pick_output_path(&input, &target_ext) {
        Ok(o) => o,
        Err(e) => return ConversionResult::err(e.to_string()),
    };

    match convert(&input, &source_ext, &target_ext, &output) {
        Ok(()) => ConversionResult::ok(output),
        Err(e) => {
            log::error!(
                "Conversion failed: {} ({} -> {}): {:#}",
                input.display(),
                source_ext,
                target_ext,
                e
            );
            // Clean up a partial file if the converter left one behind.
            let _ = std::fs::remove_file(&output);
            ConversionResult::err(format!("{e:#}"))
        }
    }
}

/// Open the platform file manager and highlight the given path.
/// Falls back to opening the containing directory if the platform
/// doesn't support a "reveal" operation.
#[tauri::command]
pub fn reveal_in_file_manager(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let parent = p.parent().unwrap_or(Path::new("."));

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", p.display()))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&p)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // There's no universal "reveal" on Linux; open the folder instead.
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

fn pick_output_path(input: &Path, target_ext: &str) -> anyhow::Result<PathBuf> {
    let parent = input.parent().ok_or_else(|| {
        anyhow::anyhow!("Input path has no parent directory: {}", input.display())
    })?;
    let stem = input
        .file_stem()
        .ok_or_else(|| anyhow::anyhow!("Input has no filename stem"))?
        .to_string_lossy()
        .into_owned();
    let candidate = parent.join(format!("{stem}.{target_ext}"));
    Ok(unique_sibling_path(&candidate))
}
