//! Central dispatch — given (source_ext, target_ext), pick the right converter.
//!
//! This mirrors the CONVERSIONS table in src/formats.ts. When you add a new
//! conversion here, you MUST also add it there, otherwise the UI will never
//! offer it.

use std::path::Path;

use anyhow::{anyhow, Result};

// PDF module is registered for v0.1.1 but not exposed in v0.1.
#[allow(unused_imports)]
use crate::convert::pdf;
use crate::convert::{images, office, text};

pub fn convert(input: &Path, src: &str, tgt: &str, output: &Path) -> Result<()> {
    let src = src.to_lowercase();
    let tgt = tgt.to_lowercase();

    // Normalise aliases so match arms stay short.
    let src = normalise_ext(&src);
    let tgt = normalise_ext(&tgt);

    match (src.as_str(), tgt.as_str()) {
        // Raster image → raster image (any combination among these)
        (s, t)
            if is_raster(s)
                && is_raster(t)
                && s != t =>
        {
            images::raster_to_raster(input, output, t)
        }

        // SVG → raster
        ("svg", t) if is_raster(t) => images::svg_to_raster(input, output, t),

        // Text / markup
        ("md", "html") => text::markdown_to_html(input, output),
        ("md", "txt") => text::markdown_to_txt(input, output),
        ("html", "md") => text::html_to_markdown(input, output),
        ("html", "txt") => text::html_to_txt(input, output),
        ("txt", "md") => text::txt_to_markdown(input, output),
        ("txt", "html") => text::txt_to_html(input, output),

        // Office
        ("xlsx", "csv") => office::xlsx_to_csv(input, output),

        (s, t) => Err(anyhow!("No converter registered for {s} → {t}")),
    }
}

fn normalise_ext(ext: &str) -> String {
    match ext {
        "jpeg" => "jpg".to_string(),
        "tif" => "tiff".to_string(),
        "htm" => "html".to_string(),
        "markdown" => "md".to_string(),
        other => other.to_string(),
    }
}

fn is_raster(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "webp" | "gif" | "bmp" | "tiff" | "ico"
    )
}
