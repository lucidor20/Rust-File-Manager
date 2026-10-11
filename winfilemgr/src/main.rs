//! WinFileMgr - Compact dual-pane file manager
//! Portable, clean interface inspired by Windows 11 aesthetics.
//! Built with egui/eframe for cross-platform (focus on Windows).

use chrono::{DateTime, Local};
use eframe::egui::{self, Color32, Key, Modifiers, RichText, Sense, Stroke, Vec2};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("WinFileMgr — Dual Pane File Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "WinFileMgr",
        options,
        Box::new(|cc| {
            // Prefer a clean, modern look
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals.window_rounding = egui::Rounding::same(6.0);
            style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
            style.spacing.item_spacing = Vec2::new(6.0, 4.0);
            style.spacing.button_padding = Vec2::new(8.0, 4.0);
            cc.egui_ctx.set_style(style);
            Ok(Box::new(FileManagerApp::new()))
        }),
    )
}

#[derive(Clone, Debug)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
    modified: Option<SystemTime>,
    extension: String,
}

impl FileEntry {
    fn from_path(path: PathBuf) -> Option<Self> {
        let meta = fs::metadata(&path).ok()?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        let modified = meta.modified().ok();
        let extension = if is_dir {
            String::new()
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        Some(Self {
            name,
            path,
            is_dir,
            size,
            modified,
            extension,
        })
    }

    fn display_size(&self) -> String {
        if self.is_dir {
            "<DIR>".to_string()
        } else {
            format_size(self.size)
        }
    }

    fn display_modified(&self) -> String {
        self.modified
            .and_then(|t| {
                let dt: DateTime<Local> = t.into();
                Some(dt.format("%Y-%m-%d %H:%M").to_string())
            })
            .unwrap_or_else(|| "-".to_string())
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PaneId {
    Left,
    Right,
}

struct Pane {
    current_path: PathBuf,
    path_edit: String,
    entries: Vec<FileEntry>,
    selected: HashSet<usize>,
    focused_index: Option<usize>,
    error: Option<String>,
    sort_by_name: bool,
}

impl Pane {
    fn new(path: PathBuf) -> Self {
        let mut p = Self {
            path_edit: path.to_string_lossy().into_owned(),
            current_path: path,
            entries: Vec::new(),
            selected: HashSet::new(),
            focused_index: None,
            error: None,
            sort_by_name: true,
        };
        p.refresh();
        p
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.selected.clear();
        self.focused_index = None;
        self.error = None;

        let path = self.current_path.clone();
        if !path.exists() {
            self.error = Some(format!("Path does not exist: {}", path.display()));
            return;
        }
        if !path.is_dir() {
            self.error = Some(format!("Not a directory: {}", path.display()));
            return;
        }

        match fs::read_dir(&path) {
            Ok(rd) => {
                let mut entries: Vec<FileEntry> = rd
                    .filter_map(|e| e.ok())
                    .filter_map(|e| FileEntry::from_path(e.path()))
                    .collect();

                // Sort: directories first, then by name (case-insensitive)
                entries.sort_by(|a, b| {
                    match (a.is_dir, b.is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    }
                });

                self.entries = entries;
                self.path_edit = path.to_string_lossy().into_owned();
            }
            Err(e) => {
                self.error = Some(format!("Cannot read directory: {}", e));
            }
        }
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.current_path = parent.to_path_buf();
            self.refresh();
        }
    }

    fn enter_dir(&mut self, idx: usize) {
        if let Some(entry) = self.entries.get(idx) {
            if entry.is_dir {
                self.current_path = entry.path.clone();
                self.refresh();
            }
        }
    }

    fn navigate_to(&mut self, path: &str) {
        let p = PathBuf::from(path.trim());
        if p.exists() && p.is_dir() {
            self.current_path = p;
            self.refresh();
        } else {
            self.error = Some(format!("Invalid path: {}", path));
            self.path_edit = self.current_path.to_string_lossy().into_owned();
        }
    }

    fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected
            .iter()
            .filter_map(|&i| self.entries.get(i).map(|e| e.path.clone()))
            .collect()
    }
}

struct FileManagerApp {
    left: Pane,
    right: Pane,
    active_pane: PaneId,
    // Dialog state
    confirm_delete: Option<(PaneId, Vec<PathBuf>)>,
    new_folder_open: bool,
    new_folder_name: String,
    new_folder_pane: PaneId,
    status_message: String,
    status_timer: f32,
}

impl FileManagerApp {
    fn new() -> Self {
        let home = dirs_home().unwrap_or_else(|| PathBuf::from("."));
        let left = Pane::new(home.clone());
        let right = Pane::new(home);
        Self {
            left,
            right,
            active_pane: PaneId::Left,
            confirm_delete: None,
            new_folder_open: false,
            new_folder_name: String::new(),
            new_folder_pane: PaneId::Left,
            status_message: "Ready — Dual-pane file manager. Tab to switch panes. F5/F6 copy/move, Del delete, F7 new folder.".to_string(),
            status_timer: 5.0,
        }
    }

    fn active_pane_mut(&mut self) -> &mut Pane {
        match self.active_pane {
            PaneId::Left => &mut self.left,
            PaneId::Right => &mut self.right,
        }
    }

    fn active_pane(&self) -> &Pane {
        match self.active_pane {
            PaneId::Left => &self.left,
            PaneId::Right => &self.right,
        }
    }

    fn other_pane_mut(&mut self) -> &mut Pane {
        match self.active_pane {
            PaneId::Left => &mut self.right,
            PaneId::Right => &mut self.left,
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_timer = 4.0;
    }

    fn do_copy(&mut self) {
        let src_paths = self.active_pane().selected_paths();
        if src_paths.is_empty() {
            self.set_status("Nothing selected to copy.");
            return;
        }
        let dest_dir = self.other_pane().current_path.clone();
        let mut ok = 0;
        let mut err = 0;
        for src in &src_paths {
            let name = src.file_name().unwrap_or_default();
            let dest = dest_dir.join(name);
            if src.is_dir() {
                match copy_dir_recursive(src, &dest) {
                    Ok(_) => ok += 1,
                    Err(e) => {
                        err += 1;
                        eprintln!("Copy dir error: {}", e);
                    }
                }
            } else {
                match fs::copy(src, &dest) {
                    Ok(_) => ok += 1,
                    Err(e) => {
                        err += 1;
                        eprintln!("Copy error: {}", e);
                    }
                }
            }
        }
        self.other_pane_mut().refresh();
        self.set_status(format!("Copied {} item(s), {} error(s).", ok, err));
    }

    fn do_move(&mut self) {
        let src_paths = self.active_pane().selected_paths();
        if src_paths.is_empty() {
            self.set_status("Nothing selected to move.");
            return;
        }
        let dest_dir = self.other_pane().current_path.clone();
        let mut ok = 0;
        let mut err = 0;
        for src in &src_paths {
            let name = src.file_name().unwrap_or_default();
            let dest = dest_dir.join(name);
            match fs::rename(src, &dest) {
                Ok(_) => ok += 1,
                Err(e) => {
                    // Cross-device move fallback: copy then delete
                    if src.is_dir() {
                        if copy_dir_recursive(src, &dest).is_ok() {
                            let _ = fs::remove_dir_all(src);
                            ok += 1;
                        } else {
                            err += 1;
                            eprintln!("Move dir error: {}", e);
                        }
                    } else if fs::copy(src, &dest).is_ok() {
                        let _ = fs::remove_file(src);
                        ok += 1;
                    } else {
                        err += 1;
                        eprintln!("Move error: {}", e);
                    }
                }
            }
        }
        self.active_pane_mut().refresh();
        self.other_pane_mut().refresh();
        self.set_status(format!("Moved {} item(s), {} error(s).", ok, err));
    }

    fn request_delete(&mut self) {
        let paths = self.active_pane().selected_paths();
        if paths.is_empty() {
            self.set_status("Nothing selected to delete.");
            return;
        }
        self.confirm_delete = Some((self.active_pane, paths));
    }

    fn do_delete(&mut self) {
        if let Some((pane_id, paths)) = self.confirm_delete.take() {
            let mut ok = 0;
            let mut err = 0;
            for p in &paths {
                let res = if p.is_dir() {
                    fs::remove_dir_all(p)
                } else {
                    fs::remove_file(p)
                };
                match res {
                    Ok(_) => ok += 1,
                    Err(e) => {
                        err += 1;
                        eprintln!("Delete error: {}", e);
                    }
                }
            }
            match pane_id {
                PaneId::Left => self.left.refresh(),
                PaneId::Right => self.right.refresh(),
            }
            self.set_status(format!("Deleted {} item(s), {} error(s).", ok, err));
        }
    }

    fn create_folder(&mut self) {
        let name = self.new_folder_name.trim();
        if name.is_empty() {
            self.set_status("Folder name cannot be empty.");
            return;
        }
        let pane = match self.new_folder_pane {
            PaneId::Left => &mut self.left,
            PaneId::Right => &mut self.right,
        };
        let new_path = pane.current_path.join(name);
        match fs::create_dir(&new_path) {
            Ok(_) => {
                pane.refresh();
                self.set_status(format!("Created folder: {}", name));
                self.new_folder_open = false;
                self.new_folder_name.clear();
            }
            Err(e) => {
                self.set_status(format!("Failed to create folder: {}", e));
            }
        }
    }

    fn open_selected(&mut self) {
        let pane = self.active_pane();
        if let Some(idx) = pane.focused_index {
            if let Some(entry) = pane.entries.get(idx) {
                if entry.is_dir {
                    // will be handled by enter
                } else {
                    let _ = open::that(&entry.path);
                }
            }
        } else if pane.selected.len() == 1 {
            if let Some(&idx) = pane.selected.iter().next() {
                if let Some(entry) = pane.entries.get(idx) {
                    if !entry.is_dir {
                        let _ = open::that(&entry.path);
                    }
                }
            }
        }
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

impl eframe::App for FileManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Status timer
        if self.status_timer > 0.0 {
            self.status_timer -= ctx.input(|i| i.unstable_dt);
        }

        // Global keyboard shortcuts
        let mut copy_req = false;
        let mut move_req = false;
        let mut delete_req = false;
        let mut new_folder_req = false;
        let mut refresh_req = false;
        let mut switch_pane = false;
        let mut go_up = false;
        let mut enter_dir = false;

        ctx.input(|i| {
            if i.key_pressed(Key::Tab) {
                switch_pane = true;
            }
            if i.key_pressed(Key::F5) {
                copy_req = true;
            }
            if i.key_pressed(Key::F6) {
                move_req = true;
            }
            if i.key_pressed(Key::Delete) || i.key_pressed(Key::F8) {
                delete_req = true;
            }
            if i.key_pressed(Key::F7) {
                new_folder_req = true;
            }
            if i.modifiers.ctrl && i.key_pressed(Key::R) {
                refresh_req = true;
            }
            if i.key_pressed(Key::Backspace) || (i.modifiers.alt && i.key_pressed(Key::ArrowUp)) {
                go_up = true;
            }
            if i.key_pressed(Key::Enter) {
                enter_dir = true;
            }
        });

        if switch_pane {
            self.active_pane = match self.active_pane {
                PaneId::Left => PaneId::Right,
                PaneId::Right => PaneId::Left,
            };
        }
        if copy_req {
            self.do_copy();
        }
        if move_req {
            self.do_move();
        }
        if delete_req {
            self.request_delete();
        }
        if new_folder_req {
            self.new_folder_pane = self.active_pane;
            self.new_folder_open = true;
            self.new_folder_name.clear();
        }
        if refresh_req {
            self.active_pane_mut().refresh();
            self.set_status("Refreshed.");
        }
        if go_up {
            self.active_pane_mut().go_up();
        }
        if enter_dir {
            let pane = self.active_pane_mut();
            if let Some(idx) = pane.focused_index {
                if pane.entries.get(idx).map(|e| e.is_dir).unwrap_or(false) {
                    pane.enter_dir(idx);
                } else {
                    // open file
                    if let Some(entry) = pane.entries.get(idx) {
                        let _ = open::that(&entry.path);
                    }
                }
            }
        }

        // Top menu / toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("WinFileMgr").strong().size(16.0));
                ui.separator();

                if ui.button("⬆ Parent").clicked() {
                    self.active_pane_mut().go_up();
                }
                if ui.button("🔄 Refresh").clicked() {
                    self.active_pane_mut().refresh();
                    self.set_status("Refreshed.");
                }
                ui.separator();
                if ui.button("📄 Copy (F5)").clicked() {
                    self.do_copy();
                }
                if ui.button("📦 Move (F6)").clicked() {
                    self.do_move();
                }
                if ui.button("🗑 Delete (Del)").clicked() {
                    self.request_delete();
                }
                if ui.button("📁 New Folder (F7)").clicked() {
                    self.new_folder_pane = self.active_pane;
                    self.new_folder_open = true;
                    self.new_folder_name.clear();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Tab = switch pane  |  Double-click = open  |  Ctrl+R = refresh").weak().small());
                });
            });
        });

        // Status bar
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let active = match self.active_pane {
                    PaneId::Left => "LEFT",
                    PaneId::Right => "RIGHT",
                };
                ui.label(RichText::new(format!("Active: {}", active)).strong());
                ui.separator();
                if self.status_timer > 0.0 {
                    ui.label(&self.status_message);
                } else {
                    let left_sel = self.left.selected.len();
                    let right_sel = self.right.selected.len();
                    ui.label(format!(
                        "Left: {} items ({} selected)  |  Right: {} items ({} selected)",
                        self.left.entries.len(),
                        left_sel,
                        self.right.entries.len(),
                        right_sel
                    ));
                }
            });
        });

        // Main dual panes
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let pane_width = (available.x - 8.0) / 2.0;

            ui.horizontal(|ui| {
                // LEFT PANE
                ui.allocate_ui_with_layout(
                    Vec2::new(pane_width, available.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.draw_pane(ui, PaneId::Left, ctx);
                    },
                );

                ui.separator();

                // RIGHT PANE
                ui.allocate_ui_with_layout(
                    Vec2::new(pane_width, available.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.draw_pane(ui, PaneId::Right, ctx);
                    },
                );
            });
        });

        // Confirm delete modal
        if let Some((_, ref paths)) = self.confirm_delete {
            let count = paths.len();
            let names: Vec<String> = paths
                .iter()
                .take(5)
                .map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
                .collect();
            let more = if count > 5 { format!("\n... and {} more", count - 5) } else { String::new() };

            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Delete {} item(s)?", count));
                    ui.add_space(4.0);
                    for n in &names {
                        ui.label(RichText::new(n).monospace());
                    }
                    if !more.is_empty() {
                        ui.label(&more);
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Delete").color(Color32::RED)).clicked() {
                            self.do_delete();
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_delete = None;
                        }
                    });
                });
        }

        // New folder modal
        if self.new_folder_open {
            egui::Window::new("New Folder")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Folder name:");
                    let response = ui.text_edit_singleline(&mut self.new_folder_name);
                    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.create_folder();
                    }
                    response.request_focus();
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            self.create_folder();
                        }
                        if ui.button("Cancel").clicked() {
                            self.new_folder_open = false;
                            self.new_folder_name.clear();
                        }
                    });
                });
        }
    }
}

impl FileManagerApp {
    fn draw_pane(&mut self, ui: &mut egui::Ui, pane_id: PaneId, ctx: &egui::Context) {
        let is_active = self.active_pane == pane_id;

        // Header / path bar
        let (go_up_clicked, path_changed, refresh_clicked) = {
            let pane = match pane_id {
                PaneId::Left => &mut self.left,
                PaneId::Right => &mut self.right,
            };

            ui.horizontal(|ui| {
                let up = ui.button("⬆").on_hover_text("Parent directory (Backspace)");
                let refresh = ui.button("🔄").on_hover_text("Refresh (Ctrl+R)");

                let path_resp = ui.add(
                    egui::TextEdit::singleline(&mut pane.path_edit)
                        .desired_width(ui.available_width() - 10.0)
                        .font(egui::TextStyle::Monospace),
                );

                let path_changed = path_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));

                (up.clicked(), path_changed, refresh.clicked())
            })
            .inner
        };

        // Apply path change / buttons after borrow
        {
            let pane = match pane_id {
                PaneId::Left => &mut self.left,
                PaneId::Right => &mut self.right,
            };
            if go_up_clicked {
                pane.go_up();
            }
            if refresh_clicked {
                pane.refresh();
            }
            if path_changed {
                let edit = pane.path_edit.clone();
                pane.navigate_to(&edit);
            }
        }

        // Make pane clickable to activate
        let frame = if is_active {
            egui::Frame::group(ui.style())
                .stroke(Stroke::new(2.0, Color32::from_rgb(0, 120, 215))) // Win11 accent-ish
                .rounding(egui::Rounding::same(4.0))
                .inner_margin(4.0)
        } else {
            egui::Frame::group(ui.style())
                .stroke(Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
                .rounding(egui::Rounding::same(4.0))
                .inner_margin(4.0)
        };

        frame.show(ui, |ui| {
            // Column headers
            ui.horizontal(|ui| {
                ui.add_sized([ui.available_width() * 0.45, 18.0], egui::Label::new(RichText::new("Name").strong().small()));
                ui.add_sized([ui.available_width() * 0.15, 18.0], egui::Label::new(RichText::new("Ext").strong().small()));
                ui.add_sized([ui.available_width() * 0.18, 18.0], egui::Label::new(RichText::new("Size").strong().small()));
                ui.add_sized([ui.available_width() * 0.22, 18.0], egui::Label::new(RichText::new("Modified").strong().small()));
            });
            ui.separator();

            let mut double_clicked_idx: Option<usize> = None;
            let mut clicked_idx: Option<usize> = None;
            let mut ctrl_held = false;
            let mut shift_held = false;

            ctx.input(|i| {
                ctrl_held = i.modifiers.ctrl || i.modifiers.command;
                shift_held = i.modifiers.shift;
            });

            egui::ScrollArea::vertical()
                .id_source(format!("scroll_{:?}", pane_id))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let pane = match pane_id {
                        PaneId::Left => &self.left,
                        PaneId::Right => &self.right,
                    };

                    if let Some(err) = &pane.error {
                        ui.colored_label(Color32::RED, err);
                        return;
                    }

                    if pane.entries.is_empty() {
                        ui.label(RichText::new("(empty directory)").weak().italics());
                        return;
                    }

                    for (idx, entry) in pane.entries.iter().enumerate() {
                        let selected = pane.selected.contains(&idx);
                        let focused = pane.focused_index == Some(idx);

                        let bg = if selected {
                            Color32::from_rgb(0, 120, 215).linear_multiply(0.35)
                        } else if focused {
                            Color32::from_rgb(0, 120, 215).linear_multiply(0.15)
                        } else {
                            Color32::TRANSPARENT
                        };

                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 22.0),
                            Sense::click(),
                        );

                        // Background
                        ui.painter().rect_filled(rect, 2.0, bg);

                        // Content
                        let name_color = if entry.is_dir {
                            Color32::from_rgb(0, 100, 180)
                        } else {
                            ui.visuals().text_color()
                        };

                        let icon = if entry.is_dir { "📁 " } else { "📄 " };
                        let name_text = format!("{}{}", icon, entry.name);

                        // Draw columns manually for alignment
                        let mut x = rect.min.x + 4.0;
                        let y = rect.center().y;

                        // Name
                        let name_w = rect.width() * 0.45;
                        ui.painter().text(
                            egui::pos2(x, y),
                            egui::Align2::LEFT_CENTER,
                            &name_text,
                            egui::FontId::proportional(13.0),
                            name_color,
                        );
                        x += name_w;

                        // Ext
                        let ext_w = rect.width() * 0.15;
                        ui.painter().text(
                            egui::pos2(x, y),
                            egui::Align2::LEFT_CENTER,
                            &entry.extension,
                            egui::FontId::proportional(12.0),
                            ui.visuals().weak_text_color(),
                        );
                        x += ext_w;

                        // Size
                        let size_w = rect.width() * 0.18;
                        ui.painter().text(
                            egui::pos2(x, y),
                            egui::Align2::LEFT_CENTER,
                            entry.display_size(),
                            egui::FontId::proportional(12.0),
                            ui.visuals().text_color(),
                        );
                        x += size_w;

                        // Modified
                        ui.painter().text(
                            egui::pos2(x, y),
                            egui::Align2::LEFT_CENTER,
                            entry.display_modified(),
                            egui::FontId::proportional(12.0),
                            ui.visuals().weak_text_color(),
                        );

                        // Interaction
                        if response.clicked() {
                            clicked_idx = Some(idx);
                        }
                        if response.double_clicked() {
                            double_clicked_idx = Some(idx);
                        }

                        // Hover highlight
                        if response.hovered() && !selected {
                            ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, Color32::from_rgb(0, 120, 215)));
                        }
                    }
                });

            // Handle selection after the loop (to avoid borrow issues)
            if let Some(idx) = clicked_idx {
                // Activate this pane
                self.active_pane = pane_id;

                let pane = match pane_id {
                    PaneId::Left => &mut self.left,
                    PaneId::Right => &mut self.right,
                };

                if ctrl_held {
                    if pane.selected.contains(&idx) {
                        pane.selected.remove(&idx);
                    } else {
                        pane.selected.insert(idx);
                    }
                    pane.focused_index = Some(idx);
                } else if shift_held {
                    if let Some(prev) = pane.focused_index {
                        let (start, end) = if prev < idx { (prev, idx) } else { (idx, prev) };
                        for i in start..=end {
                            pane.selected.insert(i);
                        }
                    } else {
                        pane.selected.clear();
                        pane.selected.insert(idx);
                    }
                    pane.focused_index = Some(idx);
                } else {
                    pane.selected.clear();
                    pane.selected.insert(idx);
                    pane.focused_index = Some(idx);
                }
            }

            if let Some(idx) = double_clicked_idx {
                self.active_pane = pane_id;
                let pane = match pane_id {
                    PaneId::Left => &mut self.left,
                    PaneId::Right => &mut self.right,
                };
                if pane.entries.get(idx).map(|e| e.is_dir).unwrap_or(false) {
                    pane.enter_dir(idx);
                } else if let Some(entry) = pane.entries.get(idx) {
                    let _ = open::that(&entry.path);
                }
            }

            // Keyboard navigation when pane is active
            if is_active {
                let mut nav_up = false;
                let mut nav_down = false;
                ctx.input(|i| {
                    if i.key_pressed(Key::ArrowUp) {
                        nav_up = true;
                    }
                    if i.key_pressed(Key::ArrowDown) {
                        nav_down = true;
                    }
                });

                let pane = match pane_id {
                    PaneId::Left => &mut self.left,
                    PaneId::Right => &mut self.right,
                };

                if nav_up || nav_down {
                    let len = pane.entries.len();
                    if len == 0 {
                        return;
                    }
                    let current = pane.focused_index.unwrap_or(0);
                    let next = if nav_up {
                        if current == 0 { 0 } else { current - 1 }
                    } else {
                        if current + 1 >= len { len - 1 } else { current + 1 }
                    };
                    pane.focused_index = Some(next);

                    // Also select if no modifiers (simple navigation selects)
                    if !ctrl_held && !shift_held {
                        pane.selected.clear();
                        pane.selected.insert(next);
                    }
                }
            }
        });

        // Click on empty area of pane to activate
        if ui.interact(ui.max_rect(), ui.id().with("pane_bg"), Sense::click()).clicked() {
            self.active_pane = pane_id;
        }
    }
}
