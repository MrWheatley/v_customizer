#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{CtxRef, Ui};
use eframe::epi::{Frame, Storage};
use eframe::{egui, epi};
use std::path::{Path, PathBuf};
use v_customizer::sca;
use v_customizer::sca::Origin;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct App {
    origin: sca::Origin,
    apply_to_all: bool,
    current_class: sca::Class,
    selected_class: sca::Class,
    selected_animation: String,
    status: String,
    sca: sca::Sca,
    // progress bar
    compiling: bool,
    progress_bar_progress: f32,
    total_items: usize,
    items_completed: usize,
    items: Vec<PathBuf>,
}

impl Default for App {
    fn default() -> Self {
        App {
            origin: sca::Origin {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                z_rot: 0.0,
            },
            apply_to_all: false,
            current_class: sca::Class::default(),
            selected_class: sca::Class::default(),
            selected_animation: "".to_owned(),
            status: "".to_owned(),
            compiling: false,
            progress_bar_progress: 0.0,
            total_items: 0,
            items_completed: 0,
            items: Vec::new(),
            sca: sca::Sca::default(),
        }
    }
}

impl epi::App for App {
    fn update(&mut self, ctx: &CtxRef, _frame: &mut Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                ui.set_enabled(!self.compiling);
                ui.group(|ui| {
                    ui.set_max_width(260f32);
                    ui.set_max_height(ui.available_height() - 25.0);
                    ui.set_min_height(ui.available_height());
                    ui.set_enabled(!self.apply_to_all);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.columns(2, |colum| {
                            for folder in &self.sca.folders {
                                if colum[0]
                                    .selectable_value(
                                        &mut self.current_class,
                                        folder.class,
                                        folder.class.to_string(),
                                    )
                                    .clicked()
                                {
                                    self.current_class = folder.class;
                                }
                            }
                            for folder in &self.sca.folders {
                                if folder.class == self.current_class {
                                    for animation in &folder.animations {
                                        if colum[1]
                                            .selectable_label(
                                                self.selected_class == folder.class
                                                    && self.selected_animation == animation.name,
                                                &animation.name,
                                            )
                                            .clicked()
                                        {
                                            self.selected_animation = animation.name.clone();
                                            self.selected_class = folder.class;
                                        }
                                    }
                                    break;
                                }
                            }
                        });
                    });
                });
                ui.group(|ui| {
                    ui.set_max_height(ui.available_height() - 25.0);
                    ui.set_min_height(ui.available_height());
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.add(egui::Checkbox::new(&mut self.apply_to_all, "Apply to all"))
                        });
                        if self.apply_to_all {
                            Self::add_rangers(ui, &mut self.origin);
                        } else {
                            for folder in self.sca.folders.iter_mut() {
                                if folder.class == self.selected_class {
                                    for animation in folder.animations.iter_mut() {
                                        if animation.name == self.selected_animation {
                                            Self::add_rangers(ui, &mut animation.origin);
                                            break;
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                        ui.separator();
                        ui.vertical_centered_justified(|ui| {
                            if ui.button("Reset").clicked() {
                                if self.apply_to_all {
                                    self.origin.reset();
                                } else {
                                    self.sca.reset_origin(
                                        &self.selected_class,
                                        &self.selected_animation,
                                    );
                                }
                            }
                            if ui.button("Reset all").clicked() {
                                self.sca.reset_all_origin();
                            }

                            if ui.button("Generate").clicked() {
                                if self.apply_to_all {
                                    self.sca.apply_to_all_origin(&self.origin);
                                }
                                if self.sca.get_selected_classes().is_empty() {
                                    self.status = "No origin is modified".to_owned();
                                    return;
                                } else {
                                    if let Err(e) = self.sca.copy_sca() {
                                        self.status = e.to_string();
                                        return;
                                    }
                                    if let Err(e) = self.sca.append_origins() {
                                        self.status = e.to_string();
                                        return;
                                    }
                                    if !self.compiling {
                                        self.compiling = true;
                                        self.progress_bar_progress = 0.0;
                                        self.items_completed = 0;
                                        let anim_qcs = match self.sca.get_temp_folder_qcs(false) {
                                            Ok(ok) => ok,
                                            Err(error) => {
                                                self.status = error.to_string();
                                                self.compiling = false;
                                                return;
                                            }
                                        };
                                        let class_qcs = match self.sca.get_selected_class_qcs() {
                                            Ok(ok) => ok,
                                            Err(error) => {
                                                self.status = error.to_string();
                                                self.compiling = false;
                                                return;
                                            }
                                        };
                                        self.items = vec![anim_qcs, class_qcs]
                                            .into_iter()
                                            .flatten()
                                            .collect::<Vec<PathBuf>>();
                                        self.total_items = self.items.len();
                                        match sca::Sca::create_temp_models_folder() {
                                            Ok(_) => {}
                                            Err(error) => self.status = error.to_string(),
                                        }
                                    }
                                }
                            }
                        });
                    });
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if self.compiling {
                    let item = match self.items.pop() {
                        None => {
                            self.status = "Compiling done!".to_owned();
                            self.compiling = false;
                            self.progress_bar_progress = 0.0;
                            self.items_completed = 0;
                            ctx.request_repaint();
                            match App::delete_temp_folder() {
                                Ok(_) => {}
                                Err(error) => {
                                    self.status = format!(
                                        "Compiling done! But failed to delete temp folder: {}",
                                        error.to_string()
                                    )
                                }
                            };
                            match sca::Sca::convert_to_vpk() {
                                Ok(_) => {}
                                Err(error) => self.status = error.to_string(),
                            }
                            match sca::Sca::delete_temp_models_folder() {
                                Ok(_) => {}
                                Err(error) => self.status = error.to_string(),
                            }
                            return;
                        }
                        Some(i) => i,
                    };
                    match sca::Sca::compile(&item) {
                        Ok(_) => {
                            self.items_completed += 1;
                            self.progress_bar_progress =
                                self.items_completed as f32 / self.total_items as f32;
                        }
                        Err(error) => {
                            self.status = error.to_string();
                            return;
                        }
                    }
                    ui.add(
                        egui::ProgressBar::new(self.progress_bar_progress)
                            .animate(true)
                            .desired_width(100.0),
                    );
                    self.status = format!(
                        "Compiling {}...",
                        item.file_name().unwrap().to_str().unwrap()
                    );
                }
                ui.label(&self.status);
            });
        });
    }
    // validates the SCA folder, sets selected_class, selected_animation, and sca
    fn setup(&mut self, _ctx: &CtxRef, _frame: &mut Frame<'_>, _storage: Option<&dyn Storage>) {
        match sca::Sca::check_folders() {
            Ok(()) => {
                // assumes these things exists
                self.selected_class = sca::Class::Scout;
                self.selected_animation = "Bat".to_owned();
            }
            Err(e) => self.status = e.to_string(),
        }
        self.sca = match sca::Sca::new() {
            Ok(ok) => {
                self.status = "Status: OK".to_owned();
                ok
            }
            Err(e) => {
                self.status = e.to_string();
                sca::Sca::default()
            }
        };
        if let Err(e) = sca::Sca::studiomdl_exe() {
            self.status = e.to_string();
        }
        if let Err(e) = sca::Sca::tf_folder() {
            self.status = e.to_string();
        }
        if let Some(storage) = _storage {
            match epi::get_value(storage, epi::APP_KEY) {
                None => {}
                Some(data) => *self = data,
            }
        }
    }

    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    // removes temp folder
    #[allow(unused_must_use)]
    fn on_exit(&mut self) {
        App::delete_temp_folder();
        match sca::Sca::delete_temp_models_folder() {
            Ok(_) => {}
            Err(error) => self.status = error.to_string(),
        }
    }

    fn name(&self) -> &str {
        "v_customizer"
    }
}

impl App {
    fn delete_temp_folder() -> anyhow::Result<()> {
        let temp_folder = sca::Sca::exe_folder().unwrap().join(sca::TEMP_FOLDER_NAME);
        if Path::new(&temp_folder).is_dir() {
            std::fs::remove_dir_all(temp_folder)?;
        }
        Ok(())
    }
    fn add_rangers(ui: &mut Ui, origin: &mut Origin) {
        ui.add(
            egui::Slider::new(&mut origin.x, -20f32..=20f32)
                .clamp_to_range(false)
                .text("x"),
        );
        ui.add(
            egui::Slider::new(&mut origin.y, -20f32..=20f32)
                .clamp_to_range(false)
                .text("y"),
        );
        ui.add(
            egui::Slider::new(&mut origin.z, -20f32..=20f32)
                .clamp_to_range(false)
                .text("z"),
        );
        ui.add(
            egui::Slider::new(&mut origin.z_rot, -20f32..=20f32)
                .clamp_to_range(false)
                .text("z rot"),
        );
    }
}

fn main() {
    let app = App::default();
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(eframe::egui::Vec2::new(560f32, 550f32));
    eframe::run_native(Box::new(app), native_options);
}
