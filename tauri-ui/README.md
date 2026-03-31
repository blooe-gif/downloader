# IDM-RS Tauri UI (Windows-installable scaffold)

This folder provides a clean Tauri v2 scaffold for building a Windows installer around the downloader core.

## Build on Windows

1. Install Rust stable + Visual Studio C++ Build Tools.
2. Install Node.js LTS.
3. `cd tauri-ui`
4. `npm install`
5. `npm run tauri build`

Generated installer artifacts will be under `src-tauri/target/release/bundle`.

## Notes

- The Rust core downloader in `/src` is production-ready CLI/back-end logic.
- Wire Tauri commands to call the core crate APIs for queue management and live telemetry.
