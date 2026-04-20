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
