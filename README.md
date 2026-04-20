# FormatLab

A clean, fully-offline file converter for images, PDFs, and documents.
Built with Tauri 2 (Rust + web), runs natively on Linux and Windows.

- **Private by design** — every conversion happens on your own machine. Nothing is ever uploaded.
- **Simple** — drag files in, pick the output format, hit convert. Results saved next to the originals.
- **Fast** — native Rust converters, no browser tax.
- **Cross-platform** — Linux (`.AppImage` / `.deb`) and Windows (`.exe`).

## Supported conversions (v0.1)

| From            | To                                               |
| --------------- | ------------------------------------------------ |
| PNG / JPG / WebP / GIF / BMP / TIFF / ICO | any of the other raster formats |
| SVG             | PNG / JPG / WebP / BMP / TIFF                    |
| Markdown (`.md`) | HTML, TXT                                        |
| HTML            | Markdown, TXT                                    |
| TXT             | Markdown, HTML                                   |
| XLSX            | CSV                                              |

Planned for future releases (v0.1.1 +):
- Images ↔ PDF (single and multi-page)
- PDF → images and PDF → text
- PDF merge / split
- HEIC / AVIF decoding
- DOCX ↔ HTML / Markdown / TXT
- Audio and video formats

## Using the pre-built app (for friends)

Grab the latest release from the [Releases page](#) (coming once v0.1 is tagged):

- **Windows:** download `FormatLab_x64-setup.exe`, run it, done.
- **Linux:** download `FormatLab_*.AppImage`, `chmod +x`, double-click.
- **Linux (Debian/Ubuntu/Mint):** download the `.deb`, double-click or run `sudo dpkg -i`.

Your files stay on your computer. FormatLab does not contact any server.

## Building from source

### Prerequisites (all platforms)

- [Rust](https://rustup.rs) ≥ 1.77
- [Node.js](https://nodejs.org) ≥ 20
- A C toolchain (`build-essential` on Linux, Visual Studio Build Tools on Windows)

### Linux (Ubuntu / Mint / Debian)

```bash
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl wget file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev \
  patchelf
```

### Windows

Install [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/downloads/)
with the **"Desktop development with C++"** workload. That's it — the rest is handled by
`rustup` and `npm`.

### macOS

```bash
xcode-select --install
```

Then:

```bash
git clone https://github.com/<you>/FormatLab.git
cd FormatLab
npm install
npm run tauri:dev    # dev mode with hot reload
npm run tauri:build  # produces a native installer in src-tauri/target/release/bundle/
```

Built installers land in `src-tauri/target/release/bundle/` — e.g.
`bundle/appimage/*.AppImage`, `bundle/deb/*.deb`, `bundle/nsis/*.exe`.

## Project layout

```
FormatLab/
├── app-icon.svg            # master icon, regenerate with `npx tauri icon`
├── index.html              # app shell
├── src/                    # frontend (TypeScript, no framework)
│   ├── main.ts             # wires up UI, drag-drop, invokes Rust
│   ├── formats.ts          # which source→target conversions exist
│   ├── queue.ts            # tiny observable store for the file queue
│   ├── theme.ts            # light/dark theme toggle
│   └── styles.css          # neutral modern UI, CSS variables for theming
└── src-tauri/              # backend (Rust)
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/
    │   └── default.json    # Tauri 2 permission capabilities
    └── src/
        ├── main.rs
        ├── lib.rs
        ├── commands.rs     # #[tauri::command] functions the frontend calls
        ├── registry.rs     # dispatch: (source_ext, target_ext) → converter
        ├── util.rs         # path helpers (unique filenames, ext parsing)
        └── convert/
            ├── images.rs   # raster↔raster, SVG→raster
            ├── pdf.rs      # images→PDF
            ├── text.rs     # Markdown ↔ HTML ↔ TXT
            └── office.rs   # XLSX→CSV
```

## Adding a new conversion

Conversions are declared in two mirrored places:

1. `src/formats.ts` — so the UI offers the option.
2. `src-tauri/src/registry.rs` — so Rust actually performs it.

Steps:

1. Add your target extension to the `FORMATS` record and the `CONVERSIONS`
   map in `src/formats.ts`.
2. Add a match arm in `src-tauri/src/registry.rs` that calls your converter.
3. Implement the converter in `src-tauri/src/convert/<category>.rs` as a
   `fn converter(input: &Path, output: &Path) -> Result<()>`.
4. `npm run tauri:dev` — the Rust side hot-rebuilds on save.

## Why Tauri (and not Electron / a web app)

- Installers are ~10–15 MB instead of 80–150 MB.
- Conversions run in native Rust — an order of magnitude faster than WASM
  for images and PDFs at scale.
- Full filesystem access means output files land right next to their inputs
  instead of through a download prompt.
- Future format support (HEIC, PDF rendering, office docs) can bundle
  native libraries instead of hoping for WASM ports.

## License

MIT — see [LICENSE](LICENSE).
