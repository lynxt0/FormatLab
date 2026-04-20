# FormatLab

A clean, fully-offline file converter for images, PDFs, and documents.
Built with Tauri 2 (Rust + web), runs natively on Linux and Windows.

- **Private by design** — every conversion happens on your own machine. Nothing is ever uploaded.
- **Simple** — drag files in, pick the output format, hit convert. Results saved next to the originals.
- **Fast** — native Rust converters, no browser tax.
- **Cross-platform** — Linux (`.AppImage` / `.deb`) and Windows (`.exe`).

## Supported conversions (v0.1)

| From            | To                                                     |
| --------------- | ------------------------------------------------------ |
| PNG / JPG / WebP / GIF / BMP / TIFF / ICO | any other raster format, plus PDF    |
| SVG             | PNG / JPG / WebP / BMP / TIFF / PDF                    |
| **HEIC / HEIF / AVIF** *(Linux only for now)* | any raster format, plus PDF |
| Markdown (`.md`) | HTML, TXT                                              |
| HTML            | Markdown, TXT                                          |
| TXT             | Markdown, HTML                                         |
| XLSX            | CSV                                                    |

> **HEIC on Windows** — decoding HEIC / HEIF / AVIF links against the
> system `libheif`, which isn't bundled into the Windows `.exe` yet.
> Windows friends will see these formats in the UI but conversions will
> fail with a clear message until v0.1.2, which will ship libheif via
> vcpkg. Linux users (via `.deb` / `.AppImage` / source build) have full
> support now.

Planned for future releases:
- Multi-image → multi-page PDF
- PDF → images and PDF → text
- PDF merge / split
- HEIC support on Windows (vcpkg-bundled libheif)
- DOCX ↔ HTML / Markdown / TXT
- Audio and video formats

## Using the pre-built app (for friends)

Grab the latest release from the
[Releases page](https://github.com/lynxt0/FormatLab/releases/latest):

- **Windows:** download `FormatLab_*_x64-setup.exe`, run it, done.
- **Linux:** download `FormatLab_*_amd64.AppImage`, `chmod +x`, double-click.
- **Linux (Debian / Ubuntu / Mint):** download the `.deb`, double-click or
  run `sudo dpkg -i FormatLab_*_amd64.deb`.

Your files stay on your computer. FormatLab does not contact any server
other than GitHub Releases for update checks.

### Updates

FormatLab checks for a new release on startup and shows a subtle banner
if one is available. One click installs and relaunches. No manual
downloads needed after the first install.

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
  patchelf \
  libheif-dev \
  libheif-plugin-libde265 \
  libheif-plugin-aomdec
```

The last three packages enable HEIC / HEIF / AVIF decoding. To build
without them, pass `--no-default-features` to Cargo:

```bash
npm run tauri:build -- --no-default-features
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

### One-click launcher on Linux (Cinnamon / GNOME / KDE / XFCE)

```bash
./scripts/install-desktop-entry.sh
```

This installs a `.desktop` entry into `~/.local/share/applications/` so
FormatLab shows up in your application menu with a proper icon, can be
pinned to the taskbar / panel, and can be dragged to your desktop as a
shortcut. By default the icon is `~/Pictures/format_lab_logo.png` if
present, otherwise the built-in app icon.

Pass a custom icon path as the first argument to override:

```bash
./scripts/install-desktop-entry.sh /path/to/my-icon.png
```

## Cutting a release

Releases are fully automated via GitHub Actions. To ship a new version:

```bash
# 1. Bump the version in src-tauri/tauri.conf.json, src-tauri/Cargo.toml
#    and package.json (keep them in sync).
# 2. Commit the bump.
git commit -am "v0.1.1"
# 3. Tag and push.
git tag v0.1.1
git push origin main --tags
```

The `Release` workflow (see `.github/workflows/release.yml`) then:

1. Builds Linux (AppImage + deb) and Windows (NSIS exe) installers.
2. Signs the installers with the private updater key.
3. Creates a draft GitHub Release containing all assets plus
   `latest.json` — the manifest the in-app updater reads.

Review the draft, tweak the release notes, and hit **Publish**. Everyone
running an older version will see the update banner on their next launch.

### One-time secrets setup

Before the first release, add these two repository secrets at
`Settings → Secrets and variables → Actions`:

- `TAURI_SIGNING_PRIVATE_KEY` — contents of `~/.tauri/formatlab.key`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — empty string if you used no
  password when generating the key.

Keep the private key safe. If you lose it, you'll have to ship a new
release with a new public key and existing users will need to reinstall
manually.

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
