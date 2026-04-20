//! Text / markup conversions.
//!
//! Keeping these simple and dependency-light. If someone hands us a very
//! fancy HTML file, we'll extract a reasonable plain-text or markdown
//! approximation — we're not a full web rendering engine.

use std::path::Path;

use anyhow::{Context, Result};
use pulldown_cmark::{html, Options, Parser};

pub fn markdown_to_html(input: &Path, output: &Path) -> Result<()> {
    let md = read_text(input)?;
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(&md, opts);
    let mut body = String::new();
    html::push_html(&mut body, parser);

    let title = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Document".into());

    let out = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n<style>body{{font-family:system-ui,sans-serif;max-width:720px;margin:2rem auto;padding:0 1rem;line-height:1.6;color:#222}}pre{{background:#f4f4f4;padding:0.75rem 1rem;border-radius:6px;overflow:auto}}code{{font-family:ui-monospace,Menlo,Consolas,monospace}}blockquote{{border-left:3px solid #ccc;padding-left:1rem;color:#555}}</style>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        html_escape(&title),
        body
    );
    write_text(output, &out)
}

pub fn markdown_to_txt(input: &Path, output: &Path) -> Result<()> {
    let md = read_text(input)?;
    // Strip a few obvious markdown markers, leaving content intact.
    let mut buf = String::with_capacity(md.len());
    for line in md.lines() {
        let trimmed = line.trim_start_matches('#').trim_start();
        let trimmed = trimmed.trim_start_matches(['>', ' ']);
        buf.push_str(trimmed);
        buf.push('\n');
    }
    write_text(output, &buf)
}

pub fn html_to_markdown(input: &Path, output: &Path) -> Result<()> {
    let html = read_text(input)?;
    let md = naive_html_to_md(&html);
    write_text(output, &md)
}

pub fn html_to_txt(input: &Path, output: &Path) -> Result<()> {
    let html = read_text(input)?;
    let txt = strip_html(&html);
    write_text(output, &txt)
}

pub fn txt_to_markdown(input: &Path, output: &Path) -> Result<()> {
    // Plain text is already valid markdown; we just copy it.
    let txt = read_text(input)?;
    write_text(output, &txt)
}

pub fn txt_to_html(input: &Path, output: &Path) -> Result<()> {
    let txt = read_text(input)?;
    let escaped = html_escape(&txt);
    let body = escaped.replace('\n', "<br>\n");
    let out = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\"><title>Text</title></head>\n<body><pre>{}</pre></body>\n</html>\n",
        body
    );
    write_text(output, &out)
}

// ------------ helpers ------------

fn read_text(input: &Path) -> Result<String> {
    std::fs::read_to_string(input)
        .with_context(|| format!("Failed to read text file: {}", input.display()))
}

fn write_text(output: &Path, data: &str) -> Result<()> {
    std::fs::write(output, data)
        .with_context(|| format!("Failed to write output: {}", output.display()))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Strip HTML tags and decode the most common entities. Good enough for a
/// "save as text" convenience button, not a full sanitiser.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Very light-weight HTML → markdown. Keeps headings, paragraphs, lists,
/// and inline emphasis. Anything fancier is flattened to text.
fn naive_html_to_md(html: &str) -> String {
    let mut s = html.to_string();

    // Normalise line endings first.
    s = s.replace("\r\n", "\n");

    // Block-level replacements. Order matters — do headings before <p>.
    for (tag, prefix) in [
        ("h1", "# "),
        ("h2", "## "),
        ("h3", "### "),
        ("h4", "#### "),
        ("h5", "##### "),
        ("h6", "###### "),
    ] {
        s = replace_block(&s, tag, prefix, "\n\n");
    }
    s = replace_block(&s, "p", "", "\n\n");
    s = replace_block(&s, "li", "- ", "\n");
    s = replace_block(&s, "blockquote", "> ", "\n\n");
    s = s.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");
    s = s.replace("<hr>", "\n---\n").replace("<hr/>", "\n---\n");

    // Inline replacements.
    for (tag, mark) in [("strong", "**"), ("b", "**"), ("em", "*"), ("i", "*"), ("code", "`")] {
        s = replace_inline(&s, tag, mark);
    }

    // Drop anything else.
    s = strip_html(&s);

    // Collapse 3+ blank lines.
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }

    s.trim().to_string() + "\n"
}

fn replace_block(haystack: &str, tag: &str, prefix: &str, suffix: &str) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(haystack.len());
    let mut rest = haystack;
    loop {
        match rest.find(&open) {
            None => {
                out.push_str(rest);
                break;
            }
            Some(start) => {
                out.push_str(&rest[..start]);
                // Skip past the '>' of the opening tag.
                let after_open_rel = match rest[start..].find('>') {
                    Some(i) => start + i + 1,
                    None => {
                        out.push_str(&rest[start..]);
                        break;
                    }
                };
                let end_rel = match rest[after_open_rel..].find(&close) {
                    Some(i) => after_open_rel + i,
                    None => {
                        out.push_str(&rest[start..]);
                        break;
                    }
                };
                out.push_str(prefix);
                out.push_str(&rest[after_open_rel..end_rel]);
                out.push_str(suffix);
                rest = &rest[end_rel + close.len()..];
            }
        }
    }
    out
}

fn replace_inline(haystack: &str, tag: &str, mark: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    haystack.replace(&open, mark).replace(&close, mark)
}
