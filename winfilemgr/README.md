# WinFileMgr — Compact Dual-Pane File Manager

A small, clean dual-pane file manager written in **Rust** using **egui / eframe**.  
Designed for a modern Windows 11 look: compact, keyboard-friendly, and portable.

## Features

- Two independent panes with their own current directory
- Navigate by double-clicking folders
- Parent directory button + editable path bar
- Columns: Name, Extension, Size, Modified date
- Multi-selection (Ctrl+Click, Shift+Click)
- Copy (F5) and Move (F6) between panes
- Delete with confirmation (Del / F8)
- Create new folder (F7)
- Refresh (Ctrl+R)
- Keyboard navigation (arrows, Enter, Backspace, Tab to switch panes)
- Clean accent highlighting on the active pane

## How to get the Windows EXE (easiest way)

### Option A – Using GitHub Actions (recommended, no local Rust needed)

1. Create a new repository on GitHub (public is free).
2. Upload **all** the files from this folder into the repository root (or push via git).
3. Go to the **Actions** tab of your repository.
4. You will see a workflow called **“Build Windows EXE”**.
5. Click **Run workflow** (or just make any small push) and wait 2–5 minutes.
6. When the run finishes successfully, open it → scroll down to **Artifacts**.
7. Download **`winfilemgr-windows`** → unzip → you get `winfilemgr.exe`.

The workflow file is already included at `.github/workflows/build.yml`.

### Option B – Build locally on Windows

1. Install Rust from https://rustup.rs/
2. Install Visual Studio Build Tools (C++ workload) if prompted.
3. Open a terminal in this folder and run:

```bash
cargo build --release
```

The executable will be at: `target\release\winfilemgr.exe`

## Keyboard Shortcuts

| Key            | Action                        |
|----------------|-------------------------------|
| Tab            | Switch active pane            |
| ↑ / ↓          | Navigate list                 |
| Enter          | Open folder / open file       |
| Backspace      | Parent directory              |
| Alt + ↑        | Parent directory              |
| F5             | Copy selected to other pane   |
| F6             | Move selected to other pane   |
| Del / F8       | Delete (with confirmation)    |
| F7             | New folder                    |
| Ctrl + R       | Refresh                       |
| Ctrl + Click   | Toggle selection              |
| Shift + Click  | Range selection               |
| Double-click   | Open folder or file           |

## Project Structure

```
winfilemgr/
├── .github/
│   └── workflows/
│       └── build.yml          ← GitHub Actions (builds Windows EXE)
├── src/
│   └── main.rs                ← Full application source
├── Cargo.toml
├── README.md
└── .gitignore
```

## Notes

- Release profile is optimized for size and speed.
- The resulting `.exe` is largely portable (copy and run).
- Also works on Linux and macOS.

Enjoy!
