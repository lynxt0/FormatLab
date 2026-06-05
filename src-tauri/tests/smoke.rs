//! End-to-end smoke tests for every converter shipped in v0.1.
//!
//! These tests generate their own inputs (no fixture files checked in),
//! run the same `registry::convert` entry point the Tauri command uses,
//! and verify the outputs are non-empty files of a plausible shape.
//!
//! Run with: `cargo test --release --test smoke -- --nocapture`.

use std::fs;
use std::path::{Path, PathBuf};

use formatlab_lib::registry::convert;

// -------- test fixtures --------

/// Create a small coloured PNG (32x32) and return its path.
fn write_test_png(dir: &Path) -> PathBuf {
    let path = dir.join("test.png");
    let mut img = image::RgbaImage::new(32, 32);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let r = ((x * 8) & 0xFF) as u8;
        let g = ((y * 8) & 0xFF) as u8;
        let b = 128;
        *px = image::Rgba([r, g, b, 255]);
    }
    img.save(&path).expect("save test png");
    path
}

fn write_test_svg(dir: &Path) -> PathBuf {
    let path = dir.join("test.svg");
    fs::write(
        &path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
            <rect width="64" height="64" fill="#2d6cdf"/>
            <circle cx="32" cy="32" r="20" fill="#ffffff"/>
        </svg>"##,
    )
    .expect("write svg");
    path
}

/// Build a minimal but realistic DNG: a TIFF whose IFD0 holds fake raw
/// CFA data (PhotometricInterpretation = CFA) and whose SubIFD holds a
/// real 16x12 JPEG preview. Returns the file path and the exact preview
/// bytes so tests can assert a lossless passthrough.
fn write_test_dng(dir: &Path) -> (PathBuf, Vec<u8>) {
    use std::io::Cursor;

    let mut prev = image::RgbImage::new(16, 12);
    for (x, y, px) in prev.enumerate_pixels_mut() {
        *px = image::Rgb([((x * 16) & 0xFF) as u8, ((y * 16) & 0xFF) as u8, 200]);
    }
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(prev)
        .write_to(&mut Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .expect("encode preview jpeg");
    let raw = vec![0xABu8; 64]; // fake CFA strip — deliberately not a JPEG

    // Fixed-size little-endian IFDs let us compute offsets up front. Every
    // entry is a single LONG so values sit inline.
    let ifd0_off = 8usize;
    let ifd0_size = 2 + 7 * 12 + 4;
    let subifd_off = ifd0_off + ifd0_size;
    let subifd_size = 2 + 6 * 12 + 4;
    let jpeg_off = subifd_off + subifd_size;
    let raw_off = jpeg_off + jpeg.len();

    fn put_ifd(buf: &mut Vec<u8>, entries: &[(u16, u32)], next: u32) {
        buf.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        for &(tag, val) in entries {
            buf.extend_from_slice(&tag.to_le_bytes());
            buf.extend_from_slice(&4u16.to_le_bytes()); // type LONG
            buf.extend_from_slice(&1u32.to_le_bytes()); // count
            buf.extend_from_slice(&val.to_le_bytes());
        }
        buf.extend_from_slice(&next.to_le_bytes());
    }

    let mut buf = Vec::new();
    buf.extend_from_slice(b"II");
    buf.extend_from_slice(&42u16.to_le_bytes());
    buf.extend_from_slice(&(ifd0_off as u32).to_le_bytes());

    // IFD0: the raw image (Compression=JPEG, Photometric=CFA=32803) plus a
    // SubIFDs pointer. The CFA tag must keep the extractor away from it.
    put_ifd(
        &mut buf,
        &[
            (256, 100),
            (257, 100),
            (259, 7),
            (262, 32803),
            (273, raw_off as u32),
            (279, raw.len() as u32),
            (330, subifd_off as u32),
        ],
        0,
    );
    // SubIFD: the JPEG preview (Compression=JPEG, Photometric=YCbCr=6).
    put_ifd(
        &mut buf,
        &[
            (256, 16),
            (257, 12),
            (259, 7),
            (262, 6),
            (273, jpeg_off as u32),
            (279, jpeg.len() as u32),
        ],
        0,
    );

    assert_eq!(buf.len(), jpeg_off, "jpeg data must follow the IFDs");
    buf.extend_from_slice(&jpeg);
    assert_eq!(buf.len(), raw_off, "raw data must follow the jpeg");
    buf.extend_from_slice(&raw);

    let path = dir.join("test.dng");
    fs::write(&path, &buf).expect("write dng");
    (path, jpeg)
}

fn write_test_markdown(dir: &Path) -> PathBuf {
    let path = dir.join("test.md");
    fs::write(
        &path,
        "# FormatLab\n\nA **local** file converter.\n\n- images\n- PDFs\n- text\n",
    )
    .expect("write md");
    path
}

fn tmpdir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "formatlab-smoke-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn assert_valid_output(path: &Path, min_bytes: u64) {
    let meta = fs::metadata(path)
        .unwrap_or_else(|e| panic!("output missing at {}: {e}", path.display()));
    assert!(
        meta.len() >= min_bytes,
        "output {} too small: {} bytes (expected >= {})",
        path.display(),
        meta.len(),
        min_bytes
    );
}

/// Magic-byte check on the first few bytes of an output file.
fn assert_magic(path: &Path, expected: &[u8], label: &str) {
    let bytes = fs::read(path).expect("read output");
    assert!(
        bytes.starts_with(expected),
        "{} at {}: got {:?}, expected prefix {:?}",
        label,
        path.display(),
        &bytes[..expected.len().min(bytes.len())],
        expected
    );
}

// -------- tests --------

#[test]
fn png_to_jpg() {
    let dir = tmpdir();
    let src = write_test_png(&dir);
    let out = dir.join("test.jpg");
    convert(&src, "png", "jpg", &out).expect("png -> jpg failed");
    assert_valid_output(&out, 200);
    assert_magic(&out, &[0xFF, 0xD8, 0xFF], "JPEG magic");
}

#[test]
fn png_to_webp() {
    let dir = tmpdir();
    let src = write_test_png(&dir);
    let out = dir.join("test.webp");
    convert(&src, "png", "webp", &out).expect("png -> webp failed");
    assert_valid_output(&out, 80);
    assert_magic(&out, b"RIFF", "WebP RIFF header");
}

#[test]
fn png_to_ico_downscales() {
    let dir = tmpdir();
    let src = write_test_png(&dir);
    let out = dir.join("test.ico");
    convert(&src, "png", "ico", &out).expect("png -> ico failed");
    assert_valid_output(&out, 100);
    assert_magic(&out, &[0x00, 0x00, 0x01, 0x00], "ICO header");
}

#[test]
fn png_to_pdf_produces_valid_pdf() {
    let dir = tmpdir();
    let src = write_test_png(&dir);
    let out = dir.join("test.pdf");
    convert(&src, "png", "pdf", &out).expect("png -> pdf failed");
    assert_valid_output(&out, 500);
    assert_magic(&out, b"%PDF-", "PDF header");

    // Cheap sanity check: the file must also contain the xref / trailer
    // markers that every PDF reader looks for.
    let bytes = fs::read(&out).unwrap();
    let tail = String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(64)..]).into_owned();
    assert!(
        tail.contains("%%EOF"),
        "PDF trailer missing at tail: {tail:?}"
    );
}

#[test]
fn svg_to_png_rasterises() {
    let dir = tmpdir();
    let src = write_test_svg(&dir);
    let out = dir.join("test.png");
    convert(&src, "svg", "png", &out).expect("svg -> png failed");
    assert_valid_output(&out, 200);
    assert_magic(&out, &[0x89, b'P', b'N', b'G'], "PNG signature");
}

#[test]
fn svg_to_pdf_embeds() {
    let dir = tmpdir();
    let src = write_test_svg(&dir);
    let out = dir.join("test.pdf");
    convert(&src, "svg", "pdf", &out).expect("svg -> pdf failed");
    assert_valid_output(&out, 500);
    assert_magic(&out, b"%PDF-", "PDF header");
}

#[test]
fn markdown_to_html() {
    let dir = tmpdir();
    let src = write_test_markdown(&dir);
    let out = dir.join("test.html");
    convert(&src, "md", "html", &out).expect("md -> html failed");
    assert_valid_output(&out, 100);
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("<h1>"), "expected <h1> tag in HTML output");
    assert!(body.contains("<strong>local</strong>"), "expected inline markdown rendered to <strong>");
    assert!(body.contains("<li>"), "expected bullet list rendering");
}

#[test]
fn markdown_to_txt_strips_marks() {
    let dir = tmpdir();
    let src = write_test_markdown(&dir);
    let out = dir.join("test.txt");
    convert(&src, "md", "txt", &out).expect("md -> txt failed");
    let body = fs::read_to_string(&out).unwrap();
    assert!(!body.starts_with("#"), "expected heading marker stripped");
    assert!(body.contains("FormatLab"), "expected original content preserved");
}

/// End-to-end AVIF → PNG test. Skips silently on machines that don't
/// have `ffmpeg` (used to author the test AVIF); CI and any dev box
/// with ffmpeg installed get full coverage.
#[test]
#[cfg(feature = "heic")]
fn avif_to_png_round_trip() {
    if which_cmd("ffmpeg").is_none() {
        eprintln!("skipping avif_to_png_round_trip: ffmpeg not installed");
        return;
    }

    let dir = tmpdir();
    let png_seed = dir.join("seed.png");
    let avif = dir.join("seed.avif");
    let out = dir.join("seed.png.out.png");

    // Make a known seed PNG so we have something to encode.
    let mut img = image::RgbImage::new(64, 64);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = image::Rgb([((x * 4) & 0xFF) as u8, ((y * 4) & 0xFF) as u8, 128]);
    }
    img.save(&png_seed).unwrap();

    // ffmpeg: PNG → AVIF using AV1 still-picture encoding.
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-loglevel", "error",
            "-y",
            "-i", png_seed.to_str().unwrap(),
            "-c:v", "libaom-av1",
            "-still-picture", "1",
            "-cpu-used", "8",
            avif.to_str().unwrap(),
        ])
        .status()
        .expect("run ffmpeg");
    assert!(status.success(), "ffmpeg failed to produce AVIF");

    // Now the real test — decode via FormatLab and confirm PNG output.
    convert(&avif, "avif", "png", &out).expect("avif -> png failed");
    assert_valid_output(&out, 200);
    assert_magic(&out, &[0x89, b'P', b'N', b'G'], "PNG signature");

    // Decoded dimensions must match the input.
    let decoded = image::open(&out).expect("open decoded png");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
}

/// If HEIC feature is disabled, the dispatcher should still error out
/// cleanly instead of panicking.
#[test]
#[cfg(not(feature = "heic"))]
fn heic_disabled_returns_friendly_error() {
    let dir = tmpdir();
    let fake = dir.join("x.heic");
    std::fs::write(&fake, b"not a real heic").unwrap();
    let out = dir.join("x.png");
    let err = convert(&fake, "heic", "png", &out).expect_err("should fail when feature is off");
    assert!(
        err.to_string().contains("isn't available in this build")
            || err.to_string().contains("not available"),
        "error message should explain the feature gate: {err}"
    );
}

fn which_cmd(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[test]
fn dng_to_jpg_passes_preview_through() {
    let dir = tmpdir();
    let (src, preview) = write_test_dng(&dir);
    let out = dir.join("test.jpg");
    convert(&src, "dng", "jpg", &out).expect("dng -> jpg failed");
    assert_magic(&out, &[0xFF, 0xD8, 0xFF], "JPEG magic");
    let got = fs::read(&out).unwrap();
    assert_eq!(
        got, preview,
        "DNG -> JPG should write the embedded preview through untouched"
    );
}

#[test]
fn dng_to_png_decodes_preview_not_raw() {
    let dir = tmpdir();
    let (src, _) = write_test_dng(&dir);
    let out = dir.join("test.png");
    convert(&src, "dng", "png", &out).expect("dng -> png failed");
    assert_magic(&out, &[0x89, b'P', b'N', b'G'], "PNG signature");
    // 16x12 is the preview; 100x100 would mean we wrongly grabbed the raw CFA.
    let decoded = image::open(&out).expect("open decoded png");
    assert_eq!(decoded.width(), 16, "should decode the preview, not the raw");
    assert_eq!(decoded.height(), 12, "should decode the preview, not the raw");
}

#[test]
fn dng_without_preview_errors_clearly() {
    let dir = tmpdir();
    // A valid little-endian TIFF header pointing at an empty IFD: no preview.
    let mut buf = Vec::new();
    buf.extend_from_slice(b"II");
    buf.extend_from_slice(&42u16.to_le_bytes());
    buf.extend_from_slice(&8u32.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes()); // 0 entries
    buf.extend_from_slice(&0u32.to_le_bytes()); // no next IFD
    let src = dir.join("empty.dng");
    fs::write(&src, &buf).unwrap();
    let out = dir.join("empty.jpg");
    let err = convert(&src, "dng", "jpg", &out).expect_err("should fail without a preview");
    assert!(
        err.to_string().contains("no embedded JPEG preview"),
        "error should explain the missing preview: {err}"
    );
}

#[test]
fn html_to_markdown_inline() {
    let dir = tmpdir();
    let src = dir.join("in.html");
    fs::write(
        &src,
        "<h2>Hi</h2><p>This is <strong>bold</strong> and <em>italic</em>.</p><ul><li>one</li><li>two</li></ul>",
    )
    .unwrap();
    let out = dir.join("out.md");
    convert(&src, "html", "md", &out).expect("html -> md failed");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("## Hi"), "heading should become ##");
    assert!(body.contains("**bold**"), "<strong> should become **bold**");
    assert!(body.contains("*italic*"), "<em> should become *italic*");
    assert!(body.contains("- one"), "list item should become - one");
}
