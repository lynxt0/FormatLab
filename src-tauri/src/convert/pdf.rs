//! PDF generation — one raster or SVG input becomes a single-page PDF
//! sized to the image's physical dimensions at 300 DPI.
//!
//! Uses `printpdf` 0.9's declarative op-list API. PDF ingest (PDF → images
//! and PDF → text) is planned for a later release and will need pdfium.

use std::io::Cursor;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use image::{codecs::png::PngEncoder, DynamicImage, ImageEncoder};
use printpdf::{
    Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, RawImage, XObjectTransform,
};

use crate::convert::images;

const DPI: f32 = 300.0;
/// Points per millimetre (PDF's native unit is points; 1 pt = 1/72 inch).
const PX_PER_MM: f32 = DPI / 25.4;

/// Embed a raster image as a single-page PDF. The page is sized so the
/// image appears at 300 DPI — a PDF printed at native size matches the
/// image's actual resolution.
pub fn image_to_pdf(input: &Path, output: &Path) -> Result<()> {
    let bytes = std::fs::read(input)
        .with_context(|| format!("Failed to read image: {}", input.display()))?;

    // printpdf needs a known encoded format (PNG/JPEG/etc). If the input
    // extension happens to be one it doesn't ingest directly, re-encode
    // via the `image` crate first.
    let bytes = if supported_by_printpdf(input) {
        bytes
    } else {
        reencode_as_png(&image::load_from_memory(&bytes).with_context(|| {
            format!("Failed to decode image: {}", input.display())
        })?)?
    };

    let (w_px, h_px) = peek_dimensions(&bytes)?;
    write_single_page_pdf(&bytes, w_px, h_px, output)
}

/// Rasterise an SVG at 2x its intrinsic size for crispness, then embed
/// it on a single-page PDF sized for the *intrinsic* dimensions (so the
/// image still prints at the correct physical size).
pub fn svg_to_pdf(input: &Path, output: &Path) -> Result<()> {
    let pixmap = images::rasterise_svg(input, Some(2.0))?;
    let (w_px_2x, h_px_2x) = (pixmap.width(), pixmap.height());

    let rgba = image::RgbaImage::from_raw(w_px_2x, h_px_2x, pixmap.data().to_vec())
        .ok_or_else(|| anyhow!("Failed to convert SVG pixmap to RGBA buffer"))?;
    let dyn_img = DynamicImage::ImageRgba8(rgba);
    let png_bytes = reencode_as_png(&dyn_img)?;

    // Page uses the 1x dimensions so the PDF shows the SVG at its natural
    // physical size, just rendered with more pixel detail.
    write_single_page_pdf(&png_bytes, w_px_2x / 2, h_px_2x / 2, output)
}

fn write_single_page_pdf(
    png_or_jpeg_bytes: &[u8],
    w_px: u32,
    h_px: u32,
    output: &Path,
) -> Result<()> {
    let w_mm = (w_px as f32 / PX_PER_MM).max(1.0);
    let h_mm = (h_px as f32 / PX_PER_MM).max(1.0);

    let mut doc = PdfDocument::new("FormatLab export");

    let mut warnings = Vec::new();
    let image = RawImage::decode_from_bytes(png_or_jpeg_bytes, &mut warnings)
        .map_err(|e| anyhow!("Failed to decode image for PDF: {e}"))?;
    for w in warnings {
        log::debug!("printpdf decode warning: {w:?}");
    }

    let image_id = doc.add_image(&image);

    let ops = vec![Op::UseXobject {
        id: image_id,
        transform: XObjectTransform {
            dpi: Some(DPI),
            ..Default::default()
        },
    }];

    let page = PdfPage::new(Mm(w_mm), Mm(h_mm), ops);

    let mut save_warnings = Vec::new();
    let bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut save_warnings);
    for w in save_warnings {
        log::debug!("printpdf save warning: {w:?}");
    }

    std::fs::write(output, &bytes)
        .with_context(|| format!("Failed to write PDF: {}", output.display()))?;
    Ok(())
}

fn supported_by_printpdf(input: &Path) -> bool {
    matches!(
        input
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .as_deref(),
        Some("png") | Some("jpg") | Some("jpeg")
    )
}

/// Encode any DynamicImage back to PNG bytes in memory.
fn reencode_as_png(img: &DynamicImage) -> Result<Vec<u8>> {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize));
    PngEncoder::new(&mut buf)
        .write_image(rgba.as_raw(), w, h, image::ColorType::Rgba8.into())
        .context("Failed to encode image as PNG for PDF embedding")?;
    Ok(buf.into_inner())
}

/// Peek at image dimensions without fully decoding when possible.
fn peek_dimensions(bytes: &[u8]) -> Result<(u32, u32)> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("Failed to guess image format")?;
    let (w, h) = reader
        .into_dimensions()
        .context("Failed to read image dimensions")?;
    Ok((w, h))
}
