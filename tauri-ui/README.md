# IDM-RS Tauri UI

A real desktop shell around the Rust downloader core.

## Prerequisites (Windows)

- Rust stable (`rustup`)
- Visual Studio Build Tools (Desktop C++)
- Node.js LTS
- WebView2 runtime (normally preinstalled on Windows 11)

## Run GUI (dev)

```bash
cd tauri-ui
npm install
npm run dev
npm run tauri dev
```

## Build installer (MSI)

```bash
cd tauri-ui
npm install
npm run build
npm run tauri build
```

Output MSI is created under:

`tauri-ui/src-tauri/target/release/bundle/msi/`

## UI capabilities

- Queue a new URL
- Trigger run of next queued task
- Refresh task list (status, priority, size, path)

The UI invokes Tauri commands wired directly to the core Rust downloader crate.
