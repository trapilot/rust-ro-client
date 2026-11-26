use std::path::{Path, PathBuf};

use eframe::egui;
use egui_ltreeview::{Action, TreeView, TreeViewState};
use ragnarok_audio::SoundManager;
use ragnarok_formats::grf::{GrfArchive, GrfFileInfo};

use crate::file_list;
use crate::gallery::Gallery;
use crate::preview::{self, BinaryPreview, BmpPreview, TextPreview};
use crate::sprite_preview::SpritePreview;
use crate::tree::{self, TreeNode};

struct LoadedGrf {
    archive: GrfArchive,
    name: String,
    file_list: Vec<GrfFileInfo>,
    tree: Vec<TreeNode>,
    tree_state: TreeViewState<u32>,
    visible_files: Vec<usize>,
    selected_dir: String,
    selected_file: Option<usize>,
    search_filter: String,
    dirty: bool,
    preview: BmpPreview,
    text_preview: TextPreview,
    binary_preview: BinaryPreview,
    sprite_preview: Option<SpritePreview>,
    gallery: Gallery,
}

impl LoadedGrf {
    fn new(archive: GrfArchive) -> Self {
        let name = archive
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());
        let file_list = archive.file_list();
        let indexed_names: Vec<(usize, &str)> = file_list
            .iter()
            .enumerate()
            .map(|(i, f)| (i, f.name.as_str()))
            .collect();
        let tree = tree::build_tree(&indexed_names);
        let visible_files = (0..file_list.len()).collect();

        Self {
            archive,
            name,
            file_list,
            tree,
            tree_state: TreeViewState::default(),
            visible_files,
            selected_dir: String::new(),
            selected_file: None,
            search_filter: String::new(),
            dirty: false,
            preview: BmpPreview::default(),
            text_preview: TextPreview::default(),
            binary_preview: BinaryPreview::default(),
            sprite_preview: None,
            gallery: Gallery::default(),
        }
    }

    fn refresh(&mut self) {
        self.file_list = self.archive.file_list();
        let indexed_names: Vec<(usize, &str)> = self
            .file_list
            .iter()
            .enumerate()
            .map(|(i, f)| (i, f.name.as_str()))
            .collect();
        self.tree = tree::build_tree(&indexed_names);
        self.selected_file = None;
        self.gallery.clear();
        self.update_visible_files();
    }

    fn update_visible_files(&mut self) {
        self.visible_files = file_list::compute_visible_files(
            &self.file_list,
            &self.selected_dir,
            &self.search_filter,
        );
    }
}

const DEFAULT_VOLUME: f32 = 0.8;
const MAX_REPORTED_FAILURES: usize = 5;

pub struct GrfEditorApp {
    archives: Vec<LoadedGrf>,
    active_tab: usize,
    error_msg: Option<String>,
    status_msg: Option<String>,
    confirm_delete: Option<usize>,
    sound: SoundManager,
    volume: f32,
    autoplayed: Option<String>,
}

impl Default for GrfEditorApp {
    fn default() -> Self {
        Self {
            archives: Vec::new(),
            active_tab: 0,
            error_msg: None,
            status_msg: None,
            confirm_delete: None,
            sound: SoundManager::new(0.0, DEFAULT_VOLUME),
            volume: DEFAULT_VOLUME,
            autoplayed: None,
        }
    }
}

#[derive(Debug, Default)]
struct ExtractReport {
    ok: usize,
    skipped_unsafe: usize,
    failed: Vec<(String, String)>,
}

/// Extracts `names` from `archive` under `dir`, keeping the GRF directory layout.
/// Unsafe entry names are skipped and a failure on one file does not stop the rest.
fn extract_files(archive: &GrfArchive, names: &[&str], dir: &Path) -> ExtractReport {
    let mut report = ExtractReport::default();
    for &name in names {
        let Some(dest) = file_list::safe_extract_path(dir, name) else {
            report.skipped_unsafe += 1;
            continue;
        };
        let result = (|| -> Result<(), String> {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("create directories: {e}"))?;
            }
            let data = archive
                .read_file(name)
                .map_err(|e| format!("read from GRF: {e}"))?;
            std::fs::write(&dest, data).map_err(|e| format!("write file: {e}"))
        })();
        match result {
            Ok(()) => report.ok += 1,
            Err(e) => report.failed.push((name.to_string(), e)),
        }
    }
    report
}

/// Asks for a destination folder and extracts `names` into it.
/// Returns `None` when the dialog is cancelled.
fn extract_with_dialog(archive: &GrfArchive, names: &[&str]) -> Option<(PathBuf, ExtractReport)> {
    let dir = rfd::FileDialog::new().pick_folder()?;
    let report = extract_files(archive, names, &dir);
    Some((dir, report))
}

impl GrfEditorApp {
    pub fn open_grf(&mut self, path: &Path) {
        match GrfArchive::open_rw(path) {
            Ok(archive) => {
                self.archives.push(LoadedGrf::new(archive));
                self.active_tab = self.archives.len() - 1;
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Failed to open {}: {e}", path.display()));
            }
        }
    }

    fn action_new(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GRF files", &["grf"])
            .save_file()
        {
            match GrfArchive::create(&path) {
                Ok(mut archive) => {
                    if let Err(e) = archive.save() {
                        self.error_msg = Some(format!("Failed to save new GRF: {e}"));
                        return;
                    }
                    self.archives.push(LoadedGrf::new(archive));
                    self.active_tab = self.archives.len() - 1;
                    self.error_msg = None;
                }
                Err(e) => {
                    self.error_msg = Some(format!("Failed to create GRF: {e}"));
                }
            }
        }
    }

    fn action_open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GRF files", &["grf"])
            .pick_file()
        {
            self.open_grf(&path);
        }
    }

    fn action_add_files(&mut self) {
        if let Some(paths) = rfd::FileDialog::new().pick_files() {
            let grf = match self.archives.get_mut(self.active_tab) {
                Some(g) => g,
                None => return,
            };
            let dir_prefix = if grf.selected_dir.is_empty() {
                String::new()
            } else {
                grf.selected_dir.clone()
            };
            for path in paths {
                let basename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let grf_path = format!("{dir_prefix}{basename}");
                match std::fs::read(&path) {
                    Ok(data) => {
                        if let Err(e) = grf.archive.add_file(&grf_path, &data) {
                            self.error_msg = Some(format!("Failed to add {basename}: {e}"));
                            return;
                        }
                    }
                    Err(e) => {
                        self.error_msg = Some(format!("Failed to read {basename}: {e}"));
                        return;
                    }
                }
            }
            grf.dirty = true;
            grf.refresh();
            self.error_msg = None;
        }
    }

    fn action_save(&mut self) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        match grf.archive.save() {
            Ok(()) => {
                grf.dirty = false;
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Save failed: {e}"));
            }
        }
    }

    fn action_extract(&mut self) {
        let Some(grf) = self.archives.get(self.active_tab) else {
            return;
        };
        let Some(file) = grf.selected_file.and_then(|i| grf.file_list.get(i)) else {
            return;
        };
        let outcome = extract_with_dialog(&grf.archive, &[file.name.as_str()]);
        self.show_extract_outcome(outcome);
    }

    fn action_extract_visible(&mut self) {
        let Some(grf) = self.archives.get(self.active_tab) else {
            return;
        };
        let names: Vec<&str> = grf
            .visible_files
            .iter()
            .filter_map(|&i| grf.file_list.get(i))
            .map(|f| f.name.as_str())
            .collect();
        let outcome = extract_with_dialog(&grf.archive, &names);
        self.show_extract_outcome(outcome);
    }

    fn action_extract_all(&mut self) {
        let Some(grf) = self.archives.get(self.active_tab) else {
            return;
        };
        let names: Vec<&str> = grf.file_list.iter().map(|f| f.name.as_str()).collect();
        let outcome = extract_with_dialog(&grf.archive, &names);
        self.show_extract_outcome(outcome);
    }

    fn show_extract_outcome(&mut self, outcome: Option<(PathBuf, ExtractReport)>) {
        let Some((dir, report)) = outcome else {
            return;
        };
        let mut status = format!("Extracted {} files to {}", report.ok, dir.display());
        if report.skipped_unsafe > 0 {
            status.push_str(&format!(
                ", {} skipped (unsafe path)",
                report.skipped_unsafe
            ));
        }
        if !report.failed.is_empty() {
            status.push_str(&format!(", {} failed", report.failed.len()));
        }
        self.status_msg = Some(status);
        self.error_msg = if report.failed.is_empty() {
            None
        } else {
            let mut msg = report
                .failed
                .iter()
                .take(MAX_REPORTED_FAILURES)
                .map(|(name, e)| format!("{name}: {e}"))
                .collect::<Vec<_>>()
                .join("; ");
            if report.failed.len() > MAX_REPORTED_FAILURES {
                msg.push_str(&format!(
                    "; ... and {} more",
                    report.failed.len() - MAX_REPORTED_FAILURES
                ));
            }
            Some(msg)
        };
    }

    fn action_remove(&mut self) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        let file_idx = match grf.selected_file {
            Some(i) => i,
            None => return,
        };
        let filename = grf.file_list[file_idx].name.clone();
        match grf.archive.remove_file(&filename) {
            Ok(_) => {
                if let Err(e) = grf.archive.save() {
                    self.error_msg = Some(format!("Failed to save after remove: {e}"));
                    return;
                }
                grf.dirty = false;
                grf.refresh();
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Failed to remove: {e}"));
            }
        }
        self.confirm_delete = None;
    }

    fn action_repack(&mut self) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        match grf.archive.repack() {
            Ok(()) => {
                grf.dirty = false;
                grf.refresh();
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Repack failed: {e}"));
            }
        }
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                self.action_new();
            }
            if ui.button("Open").clicked() {
                self.action_open();
            }

            let has_grf = self.archives.get(self.active_tab).is_some();
            let is_writable = self
                .archives
                .get(self.active_tab)
                .is_some_and(|g| g.archive.is_writable());
            let is_dirty = self.archives.get(self.active_tab).is_some_and(|g| g.dirty);
            let has_selection = self
                .archives
                .get(self.active_tab)
                .is_some_and(|g| g.selected_file.is_some());

            let has_visible = self
                .archives
                .get(self.active_tab)
                .is_some_and(|g| !g.visible_files.is_empty());

            ui.separator();

            if ui
                .add_enabled(is_writable, egui::Button::new("Add Files"))
                .clicked()
            {
                self.action_add_files();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Extract"))
                .clicked()
            {
                self.action_extract();
            }
            if ui
                .add_enabled(has_visible, egui::Button::new("Extract Visible"))
                .on_hover_text("Extract every file in the selected folder / current filter")
                .clicked()
            {
                self.action_extract_visible();
            }
            if ui
                .add_enabled(has_grf, egui::Button::new("Extract All"))
                .on_hover_text("Extract every file in this GRF")
                .clicked()
            {
                self.action_extract_all();
            }
            if ui
                .add_enabled(has_selection && is_writable, egui::Button::new("Remove"))
                .clicked()
            {
                if let Some(grf) = self.archives.get(self.active_tab) {
                    if grf.selected_file.is_some() {
                        self.confirm_delete = grf.selected_file;
                    }
                }
            }

            ui.separator();

            if ui
                .add_enabled(is_dirty && is_writable, egui::Button::new("Save"))
                .clicked()
            {
                self.action_save();
            }
            if ui
                .add_enabled(has_grf && is_writable, egui::Button::new("Repack"))
                .clicked()
            {
                self.action_repack();
            }
        });
    }

    fn show_delete_confirmation(&mut self, ctx: &egui::Context) {
        if let Some(idx) = self.confirm_delete {
            let filename = self
                .archives
                .get(self.active_tab)
                .and_then(|g| g.file_list.get(idx))
                .map(|f| f.name.clone())
                .unwrap_or_default();

            let mut open = true;
            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(format!("Delete \"{filename}\"?"));
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            self.action_remove();
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_delete = None;
                        }
                    });
                });
            if !open {
                self.confirm_delete = None;
            }
        }
    }

    fn show_tabs(&mut self, ui: &mut egui::Ui) {
        let mut close_tab = None;

        ui.horizontal(|ui| {
            for (i, grf) in self.archives.iter().enumerate() {
                let selected = i == self.active_tab;
                let label = if grf.dirty {
                    format!("{}*", grf.name)
                } else {
                    grf.name.clone()
                };
                let response = ui.selectable_label(selected, &label);
                if response.clicked() {
                    self.active_tab = i;
                }
                if ui.small_button("x").clicked() {
                    close_tab = Some(i);
                }
                ui.separator();
            }

            if ui.button("+").clicked() {
                self.action_open();
            }
        });

        if let Some(idx) = close_tab {
            self.archives.remove(idx);
            if self.active_tab >= self.archives.len() && !self.archives.is_empty() {
                self.active_tab = self.archives.len() - 1;
            }
        }
    }

    fn show_tree_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tree");
        ui.separator();

        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };

        let tree_nodes = grf.tree.clone();
        let (_response, actions) = TreeView::new(ui.make_persistent_id("grf_tree")).show_state(
            ui,
            &mut grf.tree_state,
            |builder| {
                for child in &tree_nodes {
                    tree::add_tree_node(child, builder);
                }
            },
        );

        for action in actions {
            if let Action::SetSelected(selected) = action {
                if let Some(&selected_id) = selected.last() {
                    if let Some(node) = tree::find_node_by_id(&tree_nodes, selected_id) {
                        if node.is_dir {
                            grf.selected_dir = node.full_path.clone();
                        } else {
                            if let Some(pos) = node.full_path.rfind('/') {
                                grf.selected_dir = format!("{}/", &node.full_path[..pos]);
                            } else {
                                grf.selected_dir.clear();
                            }
                            if let Some(file_idx) = node.file_index {
                                grf.selected_file = Some(file_idx);
                            }
                        }
                    } else {
                        grf.selected_dir.clear();
                    }
                    grf.update_visible_files();
                }
            }
        }
    }

    fn show_file_panel(&mut self, ui: &mut egui::Ui) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("Open a GRF file to get started");
                });
                return;
            }
        };

        ui.horizontal(|ui| {
            ui.label("Filter:");
            let changed = ui
                .add(egui::TextEdit::singleline(&mut grf.search_filter).desired_width(200.0))
                .changed();
            if changed {
                grf.update_visible_files();
            }
            ui.separator();
            ui.selectable_value(&mut grf.gallery.enabled, false, "List");
            ui.selectable_value(&mut grf.gallery.enabled, true, "Grid");
            if !grf.selected_dir.is_empty() {
                ui.separator();
                ui.label(format!("Dir: {}", grf.selected_dir));
                if ui.small_button("x").clicked() {
                    grf.selected_dir.clear();
                    grf.update_visible_files();
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{} files shown", grf.visible_files.len()));
            });
        });
        ui.separator();

        if grf.gallery.enabled {
            if grf.sprite_preview.is_none() {
                grf.sprite_preview = SpritePreview::new();
            }
            grf.gallery.show(
                ui,
                &grf.archive,
                &grf.file_list,
                &grf.visible_files,
                &mut grf.selected_file,
                &mut grf.sprite_preview,
            );
        } else {
            file_list::show_file_list(
                ui,
                &grf.file_list,
                &grf.visible_files,
                &mut grf.selected_file,
            );
        }
    }

    fn show_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(grf) = self.archives.get(self.active_tab) {
                ui.label(format!("{} files", grf.file_list.len()));
                ui.separator();
                ui.label(grf.archive.path().display().to_string());
            } else {
                ui.label("No file open");
            }
            if let Some(msg) = &self.status_msg {
                ui.separator();
                ui.label(msg);
            }
            if let Some(err) = &self.error_msg {
                ui.separator();
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    }

    fn show_file_info_panel(&mut self, ctx: &egui::Context) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        let file_idx = match grf.selected_file {
            Some(i) => i,
            None => return,
        };
        if grf.file_list.get(file_idx).is_none() {
            return;
        }

        let name = grf.file_list[file_idx].name.clone();

        if self.autoplayed.as_deref() != Some(name.as_str()) {
            self.autoplayed = Some(name.clone());
            if preview::is_audio_previewable(&name) {
                self.sound.stop_all_sfx();
                let archive = &grf.archive;
                self.sound
                    .play_sfx(&name, 1.0, 0.0, || archive.read_file(&name).ok());
            }
        }

        if preview::is_audio_previewable(&name) {
            let file = &grf.file_list[file_idx];
            let mut play = false;
            let mut stop = false;
            let mut volume_changed = false;
            egui::TopBottomPanel::bottom("file_info")
                .resizable(true)
                .default_height(120.0)
                .show(ctx, |ui| {
                    ui.heading("File Info");
                    ui.separator();
                    file_list::show_file_info(ui, file);
                    ui.separator();
                    ui.horizontal(|ui| {
                        play = ui.button("▶ Play").clicked();
                        stop = ui.button("⏹ Stop").clicked();
                        volume_changed = ui
                            .add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Volume"))
                            .changed();
                    });
                });
            if volume_changed {
                self.sound.set_volumes(0.0, self.volume);
            }
            if play {
                let archive = &grf.archive;
                self.sound
                    .play_sfx(&name, 1.0, 0.0, || archive.read_file(&name).ok());
            }
            if stop {
                self.sound.stop_all_sfx();
            }
            return;
        }

        if preview::is_text_previewable(&name) {
            preview::update_text_preview(
                &mut grf.text_preview,
                &mut grf.binary_preview,
                file_idx,
                &grf.file_list,
                &grf.archive,
            );
            let text_preview = &grf.text_preview;
            let binary_preview = &grf.binary_preview;
            let file = &grf.file_list[file_idx];
            egui::TopBottomPanel::bottom("file_info")
                .resizable(true)
                .default_height(400.0)
                .show(ctx, |ui| {
                    ui.heading("File Info");
                    ui.separator();
                    file_list::show_file_info(ui, file);
                    ui.separator();
                    if binary_preview.has_preview() {
                        binary_preview.show(ui);
                    } else {
                        text_preview.show(ui);
                    }
                });
            return;
        }

        let is_animated = preview::is_animated_previewable(&name, &grf.archive);

        if is_animated {
            if grf.sprite_preview.is_none() {
                grf.sprite_preview = SpritePreview::new();
            }
            let spr_path = grf.file_list[file_idx].name.clone();
            let file = &grf.file_list[file_idx];
            egui::TopBottomPanel::bottom("file_info")
                .resizable(true)
                .default_height(460.0)
                .show(ctx, |ui| {
                    ui.heading("File Info");
                    ui.separator();
                    file_list::show_file_info(ui, file);
                    ui.separator();
                    match &mut grf.sprite_preview {
                        Some(sp) => sp.show(ui, &grf.archive, &spr_path, file_idx),
                        None => {
                            ui.colored_label(
                                egui::Color32::RED,
                                "Sprite preview unavailable (no GPU adapter)",
                            );
                        }
                    }
                });
            return;
        }

        grf.preview
            .update(ctx, grf.selected_file, &grf.file_list, &grf.archive);
        let has_preview = grf.preview.has_preview();
        let default_height = if has_preview { 300.0 } else { 120.0 };
        let preview = &grf.preview;
        let file = &grf.file_list[file_idx];

        egui::TopBottomPanel::bottom("file_info")
            .resizable(true)
            .default_height(default_height)
            .show(ctx, |ui| {
                ui.heading("File Info");
                ui.separator();
                file_list::show_file_info(ui, file);
                if has_preview {
                    ui.separator();
                    preview.show(ui);
                }
            });
    }
}

impl eframe::App for GrfEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sound.tick();
        self.show_delete_confirmation(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.show_toolbar(ui);
        });

        if !self.archives.is_empty() {
            egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
                self.show_tabs(ui);
            });
        }

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.show_status_bar(ui);
        });

        self.show_file_info_panel(ctx);

        egui::SidePanel::left("tree_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    self.show_tree_panel(ui);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_file_panel(ui);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("grf_editor_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn extract_files_keeps_layout_skips_unsafe_and_continues_on_failure() {
        let work = scratch_dir("extract");
        let grf_path = work.join("test.grf");
        let mut archive = GrfArchive::create(&grf_path).unwrap();
        archive.add_file("data/sprite/a.spr", b"AAA").unwrap();
        archive.add_file("data/b.txt", b"BB").unwrap();
        archive.save().unwrap();

        let out = work.join("out");
        let names = [
            "data/sprite/a.spr",
            "../escape.txt",
            "/etc/escape.txt",
            "data/missing.txt",
            "data/b.txt",
        ];
        let report = extract_files(&archive, &names, &out);

        assert_eq!(report.ok, 2);
        assert_eq!(report.skipped_unsafe, 2);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, "data/missing.txt");
        assert_eq!(
            std::fs::read(out.join("data/sprite/a.spr")).unwrap(),
            b"AAA"
        );
        assert_eq!(std::fs::read(out.join("data/b.txt")).unwrap(), b"BB");
        assert!(!work.join("escape.txt").exists());

        // Existing files are overwritten.
        std::fs::write(out.join("data/b.txt"), b"old").unwrap();
        let report = extract_files(&archive, &["data/b.txt"], &out);
        assert_eq!(report.ok, 1);
        assert_eq!(std::fs::read(out.join("data/b.txt")).unwrap(), b"BB");

        let _ = std::fs::remove_dir_all(&work);
    }
}
