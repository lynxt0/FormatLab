//! DNG (Adobe Digital Negative) support.
//!
//! DNG is a raw camera format built on the TIFF container. Rather than
//! demosaicing the sensor data ourselves — which would need a heavy,
//! copyleft raw-development pipeline (rawloader/imagepipe are LGPL) — we
//! extract the full-resolution JPEG *preview* that virtually every DNG
//! writer embeds. Phone cameras (Pixel, iPhone ProRAW, Samsung), drones
//! (DJI), and Lightroom / Camera Raw all store one. That preview is the
//! camera's own rendering, so it both looks good and keeps FormatLab
//! MIT-licensed and dependency-light, and unlike the HEIC pipeline it
//! works on every platform including Windows.
//!
//! Files that genuinely contain no embedded preview (rare "raw-only"
//! archival DNGs) produce a clear error instead of a wrong image.

use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};

use crate::convert::images;

/// Convert a DNG to a raster image via its embedded JPEG preview.
///
/// For JPEG targets the preview bytes are written through untouched —
/// lossless, and it preserves the camera's own encoding and EXIF. Other
/// raster targets decode the preview and re-encode through the shared
/// image pipeline.
pub fn dng_to_raster(input: &Path, output: &Path, target_ext: &str) -> Result<()> {
    let jpeg = extract_preview_jpeg(input)?;

    if matches!(target_ext, "jpg" | "jpeg") {
        std::fs::write(output, &jpeg)
            .with_context(|| format!("Failed to write image: {}", output.display()))?;
        return Ok(());
    }

    let img = image::load_from_memory(&jpeg)
        .context("Failed to decode the JPEG preview embedded in the DNG")?;
    images::save_dynamic_image(&img, output, target_ext)
}

/// Convert a DNG to a single-page PDF via its embedded JPEG preview.
pub fn dng_to_pdf(input: &Path, output: &Path) -> Result<()> {
    let jpeg = extract_preview_jpeg(input)?;
    let img = image::load_from_memory(&jpeg)
        .context("Failed to decode the JPEG preview embedded in the DNG")?;
    crate::convert::pdf::dynamic_image_to_pdf(img, output)
}

// ---------------- embedded-preview extraction ----------------
//
// A DNG is a TIFF: a header pointing at a chain of IFDs (image file
// directories), each a table of tagged entries. The raw sensor data and
// one or more JPEG previews live in separate IFDs (often reached via the
// SubIFDs tag). We walk every IFD and pick the largest entry that is a
// real photographic JPEG, deliberately skipping the raw CFA data — which
// in "lossy DNG" files is *also* JPEG-compressed but would decode to a
// useless Bayer mosaic. The PhotometricInterpretation tag is what tells
// the two apart.

const TAG_IMAGE_WIDTH: u16 = 256;
const TAG_IMAGE_LENGTH: u16 = 257;
const TAG_COMPRESSION: u16 = 259;
const TAG_PHOTOMETRIC: u16 = 262;
const TAG_STRIP_OFFSETS: u16 = 273;
const TAG_STRIP_BYTE_COUNTS: u16 = 279;
const TAG_SUB_IFDS: u16 = 330;
const TAG_JPEG_IF_OFFSET: u16 = 513; // JPEGInterchangeFormat
const TAG_JPEG_IF_LENGTH: u16 = 514; // JPEGInterchangeFormatLength

const COMPRESSION_OLD_JPEG: u64 = 6;
const COMPRESSION_JPEG: u64 = 7;
const PHOTOMETRIC_RGB: u64 = 2;
const PHOTOMETRIC_YCBCR: u64 = 6;

const JPEG_SOI: [u8; 2] = [0xFF, 0xD8];

/// Guard against malformed or hostile files with cyclic / huge IFD graphs.
const MAX_IFDS: usize = 512;

struct Tiff<'a> {
    buf: &'a [u8],
    le: bool,
}

impl Tiff<'_> {
    fn u16(&self, off: usize) -> Result<u16> {
        let b = self
            .buf
            .get(off..off + 2)
            .ok_or_else(|| anyhow!("TIFF read out of bounds"))?;
        Ok(if self.le {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        })
    }

    fn u32(&self, off: usize) -> Result<u32> {
        let b = self
            .buf
            .get(off..off + 4)
            .ok_or_else(|| anyhow!("TIFF read out of bounds"))?;
        Ok(if self.le {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        })
    }
}

/// Byte width of a TIFF field type. Returns 0 for types we don't read.
fn type_size(ty: u16) -> usize {
    match ty {
        1 | 2 | 6 | 7 => 1,  // BYTE, ASCII, SBYTE, UNDEFINED
        3 | 8 => 2,          // SHORT, SSHORT
        4 | 9 | 11 => 4,     // LONG, SLONG, FLOAT
        5 | 10 | 12 => 8,    // RATIONAL, SRATIONAL, DOUBLE
        _ => 0,
    }
}

/// Read every value of one IFD entry, widened to u64. Handles inline
/// values (count fits in the 4-byte slot) and out-of-line value arrays.
fn entry_values(tiff: &Tiff, entry_off: usize) -> Result<Vec<u64>> {
    let ty = tiff.u16(entry_off + 2)?;
    let count = tiff.u32(entry_off + 4)? as usize;
    let ts = type_size(ty);
    if ts == 0 || count == 0 {
        return Ok(Vec::new());
    }

    let total = ts
        .checked_mul(count)
        .ok_or_else(|| anyhow!("TIFF entry value count overflows"))?;
    let base = if total <= 4 {
        entry_off + 8
    } else {
        tiff.u32(entry_off + 8)? as usize
    };

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = base + i * ts;
        let v = match ts {
            1 => *tiff.buf.get(o).ok_or_else(|| anyhow!("TIFF read out of bounds"))? as u64,
            2 => tiff.u16(o)? as u64,
            4 => tiff.u32(o)? as u64,
            8 => {
                let a = tiff.u32(o)? as u64;
                let b = tiff.u32(o + 4)? as u64;
                if tiff.le {
                    a | (b << 32)
                } else {
                    (a << 32) | b
                }
            }
            _ => unreachable!("type_size only returns 0,1,2,4,8"),
        };
        out.push(v);
    }
    Ok(out)
}

/// A located JPEG candidate inside the file, with a comparison score
/// (pixel area when known, else byte length).
struct Candidate {
    offset: usize,
    len: usize,
    score: u64,
}

pub(crate) fn extract_preview_jpeg(input: &Path) -> Result<Vec<u8>> {
    let buf = std::fs::read(input)
        .with_context(|| format!("Failed to read DNG: {}", input.display()))?;

    let le = if buf.starts_with(b"II") {
        true
    } else if buf.starts_with(b"MM") {
        false
    } else {
        bail!("Not a TIFF/DNG file (missing byte-order mark)");
    };
    let tiff = Tiff { buf: &buf, le };
    if tiff.u16(2)? != 42 {
        bail!("Not a TIFF/DNG file (bad magic number)");
    }

    let mut queue = vec![tiff.u32(4)? as usize];
    let mut visited = Vec::new();
    let mut best: Option<Candidate> = None;

    while let Some(off) = queue.pop() {
        if off == 0 || visited.contains(&off) {
            continue;
        }
        visited.push(off);
        if visited.len() > MAX_IFDS {
            break;
        }
        // A single unreadable IFD (e.g. a stray pointer) shouldn't abort
        // extraction when other IFDs hold a valid preview.
        if let Err(e) = scan_ifd(&tiff, off, &mut queue, &mut best) {
            log::debug!("DNG: skipping unreadable IFD at {off}: {e:#}");
        }
    }

    match best {
        Some(c) => Ok(buf[c.offset..c.offset + c.len].to_vec()),
        None => bail!(
            "This DNG has no embedded JPEG preview, so FormatLab can't convert it. \
             DNGs from phones, drones, and Lightroom include one; some raw-only \
             archival DNGs don't."
        ),
    }
}

/// Parse one IFD: enqueue any child IFDs and update `best` if this IFD
/// holds a photographic JPEG larger than what we've seen so far.
fn scan_ifd(
    tiff: &Tiff,
    off: usize,
    queue: &mut Vec<usize>,
    best: &mut Option<Candidate>,
) -> Result<()> {
    let n = tiff.u16(off)? as usize;

    let mut compression = None;
    let mut photometric = None;
    let mut width = None;
    let mut length = None;
    let mut jpeg_offset = None;
    let mut jpeg_len = None;
    let mut strip_offsets = Vec::new();
    let mut strip_byte_counts = Vec::new();

    for i in 0..n {
        let entry = off + 2 + i * 12;
        let tag = tiff.u16(entry)?;
        match tag {
            TAG_SUB_IFDS => {
                for v in entry_values(tiff, entry)? {
                    queue.push(v as usize);
                }
            }
            TAG_COMPRESSION => compression = entry_values(tiff, entry)?.first().copied(),
            TAG_PHOTOMETRIC => photometric = entry_values(tiff, entry)?.first().copied(),
            TAG_IMAGE_WIDTH => width = entry_values(tiff, entry)?.first().copied(),
            TAG_IMAGE_LENGTH => length = entry_values(tiff, entry)?.first().copied(),
            TAG_JPEG_IF_OFFSET => jpeg_offset = entry_values(tiff, entry)?.first().copied(),
            TAG_JPEG_IF_LENGTH => jpeg_len = entry_values(tiff, entry)?.first().copied(),
            TAG_STRIP_OFFSETS => strip_offsets = entry_values(tiff, entry)?,
            TAG_STRIP_BYTE_COUNTS => strip_byte_counts = entry_values(tiff, entry)?,
            _ => {}
        }
    }

    // Chain to the next IFD in the linked list.
    queue.push(tiff.u32(off + 2 + n * 12)? as usize);

    // Only photographic JPEGs qualify. This is what excludes the raw CFA
    // data, which may be JPEG-compressed but is PhotometricInterpretation
    // CFA (32803) or LinearRaw (34892), never RGB/YCbCr.
    let is_jpeg = matches!(compression, Some(COMPRESSION_JPEG | COMPRESSION_OLD_JPEG));
    let is_photographic = matches!(photometric, Some(PHOTOMETRIC_RGB | PHOTOMETRIC_YCBCR));
    if !is_jpeg || !is_photographic {
        return Ok(());
    }

    // Prefer the old-style JPEGInterchangeFormat pointer; fall back to a
    // single-strip image (how DNG previews are normally stored).
    let region = match (jpeg_offset, jpeg_len) {
        (Some(o), Some(l)) if l >= 2 => Some((o as usize, l as usize)),
        _ if strip_offsets.len() == 1 && strip_byte_counts.len() == 1 => {
            Some((strip_offsets[0] as usize, strip_byte_counts[0] as usize))
        }
        _ => None,
    };
    let Some((o, l)) = region else { return Ok(()) };

    let is_jpeg_bytes = tiff
        .buf
        .get(o..o + l)
        .is_some_and(|s| s.starts_with(&JPEG_SOI));
    if !is_jpeg_bytes {
        return Ok(());
    }

    let area = width.unwrap_or(0).saturating_mul(length.unwrap_or(0));
    let score = if area > 0 { area } else { l as u64 };
    if best.as_ref().map_or(true, |b| score > b.score) {
        *best = Some(Candidate {
            offset: o,
            len: l,
            score,
        });
    }
    Ok(())
}
