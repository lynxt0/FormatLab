//! HEIC / HEIF / AVIF decoding.
//!
//! Gated behind the `heic` cargo feature, which links against the system
//! `libheif`. Linux dev and release builds enable it by default; Windows
//! CI builds currently turn it off because we haven't bundled libheif
//! there yet (v0.1.2 item).
//!
//! When the feature is disabled the module still compiles — every
//! function returns a friendly "not available in this build" error, so
//! the UI can show HEIC files in the queue and the user gets a clear
//! message instead of a silent failure.

use std::path::Path;

use anyhow::Result;
use image::DynamicImage;

use crate::convert::images;

/// Decode any HEIC / HEIF / AVIF file and save it as one of the
/// supported raster formats.
pub fn heic_to_raster(input: &Path, output: &Path, target_ext: &str) -> Result<()> {
    let img = decode(input)?;
    images::save_dynamic_image(&img, output, target_ext)
}

/// Decode HEIC / HEIF / AVIF to a raster image, then embed in a PDF.
pub fn heic_to_pdf(input: &Path, output: &Path) -> Result<()> {
    let img = decode(input)?;
    crate::convert::pdf::dynamic_image_to_pdf(img, output)
}

// ---------------- implementations ----------------

#[cfg(feature = "heic")]
fn decode(input: &Path) -> Result<DynamicImage> {
    use anyhow::Context;
    use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};

    let heif = LibHeif::new();

    let ctx = HeifContext::read_from_file(
        input
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Input path is not valid UTF-8"))?,
    )
    .with_context(|| format!("Failed to open HEIC file: {}", input.display()))?;

    let handle = ctx
        .primary_image_handle()
        .context("HEIC file has no primary image")?;

    // Always decode as interleaved RGBA so we can hand the buffer
    // straight to the `image` crate.
    let heif_img = heif
        .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgba), None)
        .context("Failed to decode HEIC image")?;

    let planes = heif_img.planes();
    let plane = planes
        .interleaved
        .ok_or_else(|| anyhow::anyhow!("HEIC decode produced no interleaved plane"))?;

    let width = plane.width as u32;
    let height = plane.height as u32;
    let stride = plane.stride;

    // libheif may pad each row to a stride larger than width * 4. The
    // `image` crate wants tight-packed rows, so compact as we copy.
    let mut tight = Vec::with_capacity((width * height * 4) as usize);
    let expected_row_bytes = (width as usize) * 4;
    for y in 0..(height as usize) {
        let row_start = y * stride;
        let row_end = row_start + expected_row_bytes;
        tight.extend_from_slice(&plane.data[row_start..row_end]);
    }

    let rgba = image::RgbaImage::from_raw(width, height, tight)
        .ok_or_else(|| anyhow::anyhow!("HEIC decode produced an invalid RGBA buffer"))?;

    Ok(DynamicImage::ImageRgba8(rgba))
}

#[cfg(not(feature = "heic"))]
fn decode(_input: &Path) -> Result<DynamicImage> {
    anyhow::bail!(
        "HEIC/HEIF/AVIF decoding isn't available in this build of FormatLab. \
         Linux builds include it by default; Windows support is planned for v0.1.2."
    )
}
