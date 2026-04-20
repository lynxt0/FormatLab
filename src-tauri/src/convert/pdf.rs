//! PDF generation — reserved for v0.1.1.
//!
//! v0.1 ships without PDF output. The next release will wire up:
//!   - Raster → PDF (single-page, 300 DPI, fit-to-image)
//!   - SVG → PDF (rasterise + embed)
//!   - PDF → images (via pdfium)
//!   - PDF → text
//!   - PDF merge / split
//!
//! Keeping the module here so the registry can continue to reference it
//! as we bring these back online one at a time.

use std::path::Path;

use anyhow::{bail, Result};

pub fn image_to_pdf(_input: &Path, _output: &Path) -> Result<()> {
    bail!("PDF output is coming in v0.1.1")
}

pub fn svg_to_pdf(_input: &Path, _output: &Path) -> Result<()> {
    bail!("PDF output is coming in v0.1.1")
}
