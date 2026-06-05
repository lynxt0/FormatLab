//! Tauri commands exposed to the TypeScript frontend.

use std::path::{Path, PathBuf};

use anyhow::Context;
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
///
/// When `output_dir` is `Some`, the converted file is written there;
/// otherwise it lands next to the source file. Either way, name clashes
/// get a ` (n)` suffix so nothing is overwritten.
#[tauri::command]
pub fn convert_file(
    input_path: String,
    target_ext: String,
    output_dir: Option<String>,
) -> ConversionResult {
    let input = PathBuf::from(&input_path);
    let source_ext = match input.ext_lower() {
        Some(e) => e,
        None => return ConversionResult::err("Input file has no extension."),
    };
    let target_ext = target_ext.to_lowercase();

    if !input.exists() {
        return ConversionResult::err("Input file no longer exists.");
    }

    let output = match pick_output_path(&input, &target_ext, output_dir.as_deref()) {
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

fn pick_output_path(
    input: &Path,
    target_ext: &str,
    output_dir: Option<&str>,
) -> anyhow::Result<PathBuf> {
    let stem = input
        .file_stem()
        .ok_or_else(|| anyhow::anyhow!("Input has no filename stem"))?
        .to_string_lossy()
        .into_owned();

    let dir = match output_dir {
        Some(d) => {
            let dir = PathBuf::from(d);
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create output folder: {}", dir.display()))?;
            dir
        }
        None => input
            .parent()
            .ok_or_else(|| {
                anyhow::anyhow!("Input path has no parent directory: {}", input.display())
            })?
            .to_path_buf(),
    };

    let candidate = dir.join(format!("{stem}.{target_ext}"));
    Ok(unique_sibling_path(&candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_defaults_next_to_source() {
        let input = PathBuf::from("/tmp/photos/IMG_1234.dng");
        let out = pick_output_path(&input, "jpg", None).unwrap();
        assert_eq!(out, PathBuf::from("/tmp/photos/IMG_1234.jpg"));
    }

    #[test]
    fn output_honours_destination_dir_and_creates_it() {
        let dir = std::env::temp_dir().join(format!("formatlab-dest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let input = PathBuf::from("/somewhere/else/IMG_1234.dng");

        let out = pick_output_path(&input, "jpg", Some(dir.to_str().unwrap())).unwrap();

        assert_eq!(out, dir.join("IMG_1234.jpg"));
        assert!(dir.exists(), "destination folder should be created if missing");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
