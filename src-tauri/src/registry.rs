//! Central dispatch — given (source_ext, target_ext), pick the right converter.
//!
//! This mirrors the CONVERSIONS table in src/formats.ts. When you add a new
//! conversion here, you MUST also add it there, otherwise the UI will never
//! offer it.

use std::path::Path;

use anyhow::{anyhow, Result};

use crate::convert::{heic, images, office, pdf, text};

pub fn convert(input: &Path, src: &str, tgt: &str, output: &Path) -> Result<()> {
    let src = src.to_lowercase();
    let tgt = tgt.to_lowercase();

    // Normalise aliases so match arms stay short.
    let src = normalise_ext(&src);
    let tgt = normalise_ext(&tgt);

    match (src.as_str(), tgt.as_str()) {
        // HEIC / HEIF / AVIF → raster or PDF. These all decode through
        // the same libheif pipeline; the source extension is only a hint
        // about container + codec.
        (s, t) if is_heic(s) && is_raster(t) => heic::heic_to_raster(input, output, t),
        (s, "pdf") if is_heic(s) => heic::heic_to_pdf(input, output),

        // Raster image → raster image (any combination among these)
        (s, t)
            if is_raster(s)
                && is_raster(t)
                && s != t =>
        {
            images::raster_to_raster(input, output, t)
        }

        // Raster image → PDF
        (s, "pdf") if is_raster(s) => pdf::image_to_pdf(input, output),

        // SVG → raster
        ("svg", t) if is_raster(t) => images::svg_to_raster(input, output, t),

        // SVG → PDF (rasterise at 2x then embed)
        ("svg", "pdf") => pdf::svg_to_pdf(input, output),

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
        // HEIC/HEIF/AVIF all share the same HEIF container and decode
        // through the same libheif pipeline. We keep the three distinct
        // in the registry because users expect to see "AVIF → PNG" as a
        // separate row from "HEIC → PNG" in the UI, and because libheif
        // picks different decoder plugins per content.
        other => other.to_string(),
    }
}

fn is_raster(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "webp" | "gif" | "bmp" | "tiff" | "ico"
    )
}

fn is_heic(ext: &str) -> bool {
    matches!(ext, "heic" | "heif" | "avif")
}
