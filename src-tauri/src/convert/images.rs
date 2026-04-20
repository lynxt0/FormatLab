//! Image conversions — raster ↔ raster, and SVG → raster.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, ImageFormat};

/// Convert any supported raster format to any other supported raster format.
pub fn raster_to_raster(input: &Path, output: &Path, target_ext: &str) -> Result<()> {
    let img = image::open(input)
        .with_context(|| format!("Failed to decode image: {}", input.display()))?;
    save_image(&img, output, target_ext)
}

/// Rasterise an SVG and save it as one of the supported raster formats.
pub fn svg_to_raster(input: &Path, output: &Path, target_ext: &str) -> Result<()> {
    let pixmap = rasterise_svg(input, None)?;
    let img = pixmap_to_image(&pixmap);
    save_image(&img, output, target_ext)
}

/// Read an SVG file and render it to a tiny_skia::Pixmap.
///
/// `scale` scales both axes. `None` = 1.0.
pub(crate) fn rasterise_svg(input: &Path, scale: Option<f32>) -> Result<tiny_skia::Pixmap> {
    let data = std::fs::read(input)
        .with_context(|| format!("Failed to read SVG: {}", input.display()))?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&data, &opt)
        .with_context(|| format!("Failed to parse SVG: {}", input.display()))?;
    let size = tree.size();
    let scale = scale.unwrap_or(1.0);
    let w = (size.width() * scale).ceil().max(1.0) as u32;
    let h = (size.height() * scale).ceil().max(1.0) as u32;

    let mut pixmap =
        tiny_skia::Pixmap::new(w, h).ok_or_else(|| anyhow!("Failed to allocate pixmap {w}x{h}"))?;

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(pixmap)
}

fn pixmap_to_image(pixmap: &tiny_skia::Pixmap) -> DynamicImage {
    let buf = image::RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
        .expect("pixmap dimensions match buffer length by construction");
    DynamicImage::ImageRgba8(buf)
}

fn save_image(img: &DynamicImage, output: &Path, target_ext: &str) -> Result<()> {
    let fmt = ext_to_format(target_ext)?;
    let prepared = prepare_for_format(img, fmt);
    prepared
        .save_with_format(output, fmt)
        .with_context(|| format!("Failed to write image: {}", output.display()))?;
    Ok(())
}

fn ext_to_format(ext: &str) -> Result<ImageFormat> {
    Ok(match ext {
        "png" => ImageFormat::Png,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "webp" => ImageFormat::WebP,
        "gif" => ImageFormat::Gif,
        "bmp" => ImageFormat::Bmp,
        "tiff" | "tif" => ImageFormat::Tiff,
        "ico" => ImageFormat::Ico,
        other => return Err(anyhow!("Unsupported image target: {other}")),
    })
}

/// Some formats don't support alpha, or want specific size constraints.
/// Normalise the image accordingly so saving doesn't fail or produce
/// a surprising result.
fn prepare_for_format(img: &DynamicImage, fmt: ImageFormat) -> DynamicImage {
    match fmt {
        ImageFormat::Jpeg | ImageFormat::Bmp => {
            // JPEG has no alpha; BMP's alpha support is inconsistent. Flatten
            // onto white so transparency doesn't become random colours.
            flatten_on_white(img)
        }
        ImageFormat::Ico => {
            // ICO maxes out at 256x256. Resize proportionally if needed.
            let (w, h) = (img.width(), img.height());
            if w <= 256 && h <= 256 {
                img.clone()
            } else {
                img.resize(256, 256, image::imageops::FilterType::Lanczos3)
            }
        }
        _ => img.clone(),
    }
}

fn flatten_on_white(img: &DynamicImage) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut out = image::RgbImage::new(w, h);
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = p[3] as f32 / 255.0;
        let inv = 1.0 - a;
        let r = (p[0] as f32 * a + 255.0 * inv).round() as u8;
        let g = (p[1] as f32 * a + 255.0 * inv).round() as u8;
        let b = (p[2] as f32 * a + 255.0 * inv).round() as u8;
        out.put_pixel(x, y, image::Rgb([r, g, b]));
    }
    DynamicImage::ImageRgb8(out)
}
