English | [简体中文](README_CN.md)

# Calliope

[![CI](https://github.com/LanceHuang245/Calliope/actions/workflows/ci.yml/badge.svg)](https://github.com/LanceHuang245/Calliope/actions/workflows/ci.yml)

Calliope is a local-first, cross-platform Markdown knowledge base written in Rust. It renders with each platform's native UI through [Native Shell (nsh)](https://nsh.dev) and runs on Windows, macOS, Linux, Android and iOS.\
Notes stay as plain Markdown files in a folder you own, connected through wiki links, backlinks and a graph of the whole vault. Sync goes directly to your own WebDAV or S3 storage, with no intermediary server.
> **Note**
>
> Calliope is in early development, and things will change. Keep a backup of the notes you open with it, and of the ones you sync.
---
## Features
- Plain files: any folder of Markdown notes is a vault; Calliope only adds a hidden `.calliope/` folder
- Three modes: Edit, Read, and Split side by side, with find and replace in the source
- Markdown rendering: GitHub-flavored Markdown (tables, task lists, footnotes), code highlighting, LaTeX formulas, images and GitHub's five callouts
- Wiki links: `[[Note]]`, `[[Note|alias]]`, `[[Note#Heading]]` and `[[Folder/Note]]`, resolved by name across the vault; a link to a missing note offers to create it
- Backlinks: every note linking to the open one, with the line of the link
- Link completion: type `[[` for the notes' titles
- Quick switcher: find a note by its title, or create one
- Mermaid diagrams: flowcharts, sequence, class, state, Gantt and pie charts, drawn natively in Rust by [merman](https://crates.io/crates/merman) (no WebView)
- Graph view: every note and link, with pan, zoom, hover highlighting, and optional orphans and not-yet-created notes
- Sync with WebDAV and S3: a three-way comparison with the last sync, conflict copies instead of overwrites, a question before deleting many files, and deleted files in the vault's trash for 15 days; passwords stay in the system's keyring (on Android, in the app's private storage for now)
- Safe editing: autosave within a second, atomic writes, deleted files go to the trash, and changes made by other apps are picked up without overwriting yours
- Themes: light, dark, or following the system
- Native on five platforms: Windows, macOS, Linux, Android and iOS

## Roadmap
Calliope is built in stages; links, the graph and sync are done. Next:
- Mobile: the system's tab bar and navigation, Material 3 on Android and Liquid Glass on iOS
- End-to-end encryption of synced notes, file names and folder names
- Polish: tabs, full-text search, a command palette, a local graph and an outline

## Usage
1. Open a folder of Markdown notes, or create a new vault. On a phone, Calliope creates a vault in its own storage
2. Create a note with `Ctrl+N`; its title is its file name. Folders, renaming and deleting are in the file tree's context menu
3. Link notes with `[[Note]]`, then follow the links in Read mode and see them in the Backlinks panel
4. Open the graph with `Ctrl+G` or the button under the file tree; drag to move it, `Ctrl` + scroll (or pinch) to zoom
5. Set up sync in Settings → Sync (WebDAV or S3). It syncs on opening the vault, 30 seconds after you stop editing and every few minutes, or with the sync button

| Shortcut | Action |
|---|---|
| `Ctrl+N` | New note |
| `Ctrl+E` | Switch between Edit and Read |
| `Ctrl+F` | Find in the note |
| `Ctrl+O` | Quick switcher |
| `Ctrl+G` | Graph |
| `Ctrl+\` | Hide or show the sidebar |

On macOS, use `Cmd` in place of `Ctrl`.

## Contributing
We welcome community users to contribute! Feel free to fork this repository, submit Pull Requests, and make suggestions and report bugs through [Issues](https://github.com/LanceHuang245/Calliope/issues).

### Building
You need:
- Rust stable (`rust-toolchain.toml` installs the targets it needs)
- Windows: Visual Studio Build Tools (Desktop development with C++, Windows SDK)
- Linux: GTK 4.14 or later, and GStreamer, with their development packages
- Mobile builds and packages: the [nsh CLI](https://nsh.dev/docs/installation); Android needs its SDK and NDK, iOS needs Xcode

```sh
cargo run -p calliope            # run on this desktop
cargo test --workspace           # every test, the headless UI tests included
nsh run app --target android     # run on an Android emulator or device
nsh package app --target windows # build an installer
```

The UI lives in `app/`; the vault, links, index and graph in `crates/calliope-core`, and sync in `crates/calliope-sync`. Neither crate depends on the UI, so both are tested on their own.

### Translation
The interface is English only for now. Every text of it is in [`app/src/strings.rs`](app/src/strings.rs), so other languages can be added later.

## Download
There is no release yet. Every push builds installers for all five platforms: [Calliope Action](https://github.com/LanceHuang245/Calliope/actions/workflows/ci.yml) (sign in to GitHub to download them).

| Platform | Minimum version | Notes |
|---|---|---|
| Windows | Windows 10 1809 | |
| macOS | macOS 14 | Built and tested in CI only |
| Linux | GTK 4.14 | |
| Android | Android 9 | APK for 64-bit (arm64-v8a) or 32-bit (armeabi-v7a) phones, or universal (emulators too) |
| iOS | iOS 15 | Unsigned IPA: sign it yourself to install |

## License
[Apache License 2.0](LICENSE) © Linxing Huang
