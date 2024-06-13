use std::sync::{Arc, RwLock};
use three_d::egui::*;

use crate::{load_models, Resources};

pub struct Config {
    pub asset_menu: bool,
    pub showing_assets: bool,
    pub selected_asset: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            asset_menu: true,
            showing_assets: false,
            selected_asset: None,
        }
    }
}

pub trait IGui {
    fn name(&self) -> &'static str {
        "Window"
    }

    fn show(&mut self, config: &mut Config, ctx: &three_d::egui::Context);
}

pub struct Gui {
    resources: Arc<RwLock<Resources>>,
    children: Vec<Box<dyn IGui>>,
}

impl Gui {
    pub fn new(resources: Arc<RwLock<Resources>>) -> Self {
        let mut gui = Self {
            resources: Arc::clone(&resources),
            children: Vec::new(),
        };

        let mm = MainMenu {};
        let asset_menu = AssetMenu {
            resources: Arc::clone(&resources),
        };
        let asset_viewer = AssetViewer {
            resources: Arc::clone(&resources)
        };
        gui.add_ui_element(mm);
        gui.add_ui_element(asset_menu);
        gui.add_ui_element(asset_viewer);
        gui
    }

    pub fn add_ui_element(&mut self, elem: impl IGui + 'static) {
        self.children.push(Box::new(elem))
    }
}

impl IGui for Gui {
    fn name(&self) -> &'static str {
        "Void"
    }

    fn show(&mut self, config: &mut Config, ctx: &three_d::egui::Context) {
        for child in &mut self.children {
            child.show(config, ctx);
        }
    }
}

pub struct MainMenu {}

impl IGui for MainMenu {
    fn show(&mut self, config: &mut Config, ctx: &three_d::egui::Context) {
        TopBottomPanel::top("TOp Menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Assets").clicked() {
                    config.asset_menu = !config.asset_menu;
                }
            });
        });
    }
}

pub struct AssetMenu {
    resources: Arc<RwLock<Resources>>,
}

impl IGui for AssetMenu {
    fn show(&mut self, config: &mut Config, ctx: &three_d::egui::Context) {
        use three_d::egui::*;
        if config.asset_menu {
            SidePanel::right("Asset Panel").show(ctx, |ui| {
                ui.vertical(|ui| {
                    if ui.button("Asset Folder").clicked() {
                        if let  Some(path) = rfd::FileDialog::new().pick_folder() {
                            let mut res = self.resources.write().unwrap();
                            if let Ok(models) = load_models(&res.ctx, path) {
                                res.models.extend(models);
                            }
                        }
                    }
                    ui.separator();
                    if ui.button("Import Asset").clicked() {
                        if let  Some(path) = rfd::FileDialog::new().pick_file() {
                            let mut res = self.resources.write().unwrap();
                            if let Ok(models) = load_models(&res.ctx, path) {
                                res.models.extend(models);
                            }
                        }
                    }
                    ui.separator();
                    ScrollArea::vertical().show(ui, |ui| {
                        let res = self.resources.read().unwrap();
                        for (idx, _model) in res.models.iter().enumerate() {
                            let name = format!("Segment {idx}");
                            if ui.button(name).clicked() {
                                println!("Selecting asset  {idx}");
                                config.selected_asset = Some(idx);
                            }
                        }
                    });
                });
            });
        }
    }
}

pub struct AssetViewer {
    resources: Arc<RwLock<Resources>>
}

#[rustfmt::skip]
impl IGui for AssetViewer {
    fn show(&mut self, config: &mut Config, ctx: &three_d::egui::Context) {
        if let Some(idx) = config.selected_asset {
            let mut res = self.resources.write().unwrap();
            let models = &mut res.models;
            let selected_model = &mut models[idx];
            SidePanel::left("Asset Menu").show(ctx, |ui| {
                if ui.button("Close").clicked() {
                    config.selected_asset = None;
                }
                ui.label("Surface Properties");
                ui.add(Slider::new::<f32>(&mut selected_model.normal_mesh.material.metallic, 0.0..=1.0).text("Metallic"));
                ui.add(Slider::new::<f32>(&mut selected_model.normal_mesh.material.roughness, 0.0..=1.0).text("Roughness"));


                ui.label("RGBA");
                ui.add(Slider::new::<u8>(&mut selected_model.normal_mesh.material.albedo.r, 0..=255).text("R"));
                ui.add(Slider::new::<u8>(&mut selected_model.normal_mesh.material.albedo.g, 0..=255).text("G"));
                ui.add(Slider::new::<u8>(&mut selected_model.normal_mesh.material.albedo.b, 0..=255).text("B"));
                ui.add(Slider::new::<u8>(&mut selected_model.normal_mesh.material.albedo.a, 0..=255).text("A"));
            });
        }
    }
}
