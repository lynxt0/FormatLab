/**
 * Format registry for the frontend.
 * Each source format lists the target formats it can be converted to.
 * The Rust backend has the matching registry and actually performs conversion.
 *
 * To add a new conversion you must update BOTH this file and src-tauri/src/registry.rs.
 */

export type Category = "image" | "pdf" | "text" | "office";

export interface FormatInfo {
  /** Lowercase extension without the leading dot, e.g. "png". */
  ext: string;
  /** Human-friendly label shown in the UI. */
  label: string;
  category: Category;
}

/**
 * Lookup by lowercase extension.
 */
export const FORMATS: Record<string, FormatInfo> = {
  // Images
  png:  { ext: "png",  label: "PNG",  category: "image" },
  jpg:  { ext: "jpg",  label: "JPG",  category: "image" },
  jpeg: { ext: "jpeg", label: "JPEG", category: "image" },
  webp: { ext: "webp", label: "WebP", category: "image" },
  gif:  { ext: "gif",  label: "GIF",  category: "image" },
  bmp:  { ext: "bmp",  label: "BMP",  category: "image" },
  tiff: { ext: "tiff", label: "TIFF", category: "image" },
  tif:  { ext: "tif",  label: "TIFF", category: "image" },
  ico:  { ext: "ico",  label: "ICO",  category: "image" },
  svg:  { ext: "svg",  label: "SVG",  category: "image" },

  // HEIF family (decode-only in v0.1.1 — no one wants to produce HEIC)
  heic: { ext: "heic", label: "HEIC", category: "image" },
  heif: { ext: "heif", label: "HEIF", category: "image" },
  avif: { ext: "avif", label: "AVIF", category: "image" },

  // PDF
  pdf:  { ext: "pdf",  label: "PDF",  category: "pdf" },

  // Text / markup
  md:       { ext: "md",       label: "Markdown", category: "text" },
  markdown: { ext: "markdown", label: "Markdown", category: "text" },
  html:     { ext: "html",     label: "HTML",     category: "text" },
  htm:      { ext: "htm",      label: "HTML",     category: "text" },
  txt:      { ext: "txt",      label: "Plain text", category: "text" },

  // Office
  xlsx: { ext: "xlsx", label: "Excel (XLSX)", category: "office" },
  csv:  { ext: "csv",  label: "CSV",          category: "office" },
};

/**
 * For a given source extension, which target extensions are supported?
 * Keep this mirrored with the Rust registry.
 */
export const CONVERSIONS: Record<string, string[]> = {
  // Raster images can all go to each other, and to PDF.
  png:  ["jpg", "webp", "gif", "bmp", "tiff", "ico", "pdf"],
  jpg:  ["png", "webp", "gif", "bmp", "tiff", "ico", "pdf"],
  jpeg: ["png", "webp", "gif", "bmp", "tiff", "ico", "pdf"],
  webp: ["png", "jpg", "gif", "bmp", "tiff", "ico", "pdf"],
  gif:  ["png", "jpg", "webp", "bmp", "tiff", "ico", "pdf"],
  bmp:  ["png", "jpg", "webp", "gif", "tiff", "ico", "pdf"],
  tiff: ["png", "jpg", "webp", "gif", "bmp", "ico", "pdf"],
  tif:  ["png", "jpg", "webp", "gif", "bmp", "ico", "pdf"],
  ico:  ["png", "jpg", "webp", "gif", "bmp", "tiff", "pdf"],

  // SVG → raster or PDF (SVG passthrough isn't a useful "conversion")
  svg:  ["png", "jpg", "webp", "bmp", "tiff", "pdf"],

  // HEIC / HEIF / AVIF are decode-only for now — export to any raster
  // format or PDF. Encoding TO HEIC/AVIF requires patented encoders we
  // haven't bundled (planned for v0.2).
  heic: ["png", "jpg", "webp", "gif", "bmp", "tiff", "pdf"],
  heif: ["png", "jpg", "webp", "gif", "bmp", "tiff", "pdf"],
  avif: ["png", "jpg", "webp", "gif", "bmp", "tiff", "pdf"],

  // Text / markup
  md:       ["html", "txt"],
  markdown: ["html", "txt"],
  html:     ["md", "txt"],
  htm:      ["md", "txt"],
  txt:      ["md", "html"],

  // Office (v1: xlsx → csv only)
  xlsx: ["csv"],
};

/**
 * Given a set of source extensions from the queue, compute the target
 * formats that every file supports. Returns empty if the queue has no
 * common target.
 */
export function commonTargets(sourceExts: string[]): string[] {
  if (sourceExts.length === 0) return [];
  const perFile = sourceExts.map((ext) => CONVERSIONS[ext.toLowerCase()] ?? []);
  if (perFile.some((arr) => arr.length === 0)) return [];
  const [first, ...rest] = perFile;
  return first.filter((t) => rest.every((arr) => arr.includes(t)));
}

/**
 * Parse the extension from a filename (lowercased, no leading dot).
 * Returns empty string if no extension.
 */
export function extOf(filename: string): string {
  const dot = filename.lastIndexOf(".");
  if (dot < 0 || dot === filename.length - 1) return "";
  return filename.slice(dot + 1).toLowerCase();
}
