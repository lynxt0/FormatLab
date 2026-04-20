//! Office conversions. v1 covers XLSX → CSV (first worksheet only).
//! Multi-sheet export and DOCX handling are planned for v1.1.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook, Data, Reader, Xlsx};

pub fn xlsx_to_csv(input: &Path, output: &Path) -> Result<()> {
    let mut wb: Xlsx<_> = open_workbook(input)
        .with_context(|| format!("Failed to open workbook: {}", input.display()))?;

    let sheet_name = wb
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("Workbook has no sheets"))?;

    let range = wb
        .worksheet_range(&sheet_name)
        .with_context(|| format!("Failed to read sheet '{sheet_name}'"))?;

    let file = File::create(output)
        .with_context(|| format!("Failed to create CSV: {}", output.display()))?;
    let mut w = BufWriter::new(file);

    for row in range.rows() {
        let mut first = true;
        for cell in row {
            if !first {
                w.write_all(b",")?;
            }
            first = false;
            write_cell(&mut w, cell)?;
        }
        w.write_all(b"\n")?;
    }
    w.flush()?;
    Ok(())
}

fn write_cell<W: Write>(w: &mut W, cell: &Data) -> Result<()> {
    let raw = match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => format_float(*f),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => d.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERR({:?})", e),
    };
    if needs_quoting(&raw) {
        let escaped = raw.replace('"', "\"\"");
        write!(w, "\"{escaped}\"")?;
    } else {
        w.write_all(raw.as_bytes())?;
    }
    Ok(())
}

fn needs_quoting(s: &str) -> bool {
    s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r')
}

fn format_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e16 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}
