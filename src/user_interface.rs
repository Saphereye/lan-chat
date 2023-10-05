#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::FontFamily::Proportional;
use egui::FontId;
use egui::TextStyle::*;

pub fn create_gui() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(400.0, 300.0)),
        ..Default::default()
    };

    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::<ChatApp>::default()),
    )
}

struct ChatApp {
    chat: Vec<String>,
    input: String,
}

impl Default for ChatApp {
    fn default() -> Self {
        Self {
            chat: vec!["Hi!".to_string(), "Hello!".to_string(), "What a beautiful day!".to_string()],
            input: String::new(),
        }
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Get current context style
        let mut style = (*ctx.style()).clone();

        // Redefine text_styles
        style.text_styles = [
            (Heading, FontId::new(30.0, Proportional)),
            (Name("Heading2".into()), FontId::new(25.0, Proportional)),
            (Name("Context".into()), FontId::new(23.0, Proportional)),
            (Body, FontId::new(18.0, Proportional)),
            (Monospace, FontId::new(14.0, Proportional)),
            (Button, FontId::new(14.0, Proportional)),
            (Small, FontId::new(10.0, Proportional)),
        ]
        .into();

        // Mutate global style with above changes
        ctx.set_style(style);

        egui::TopBottomPanel::bottom("Input").show(&ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Input Box: ");
                let response = ui.text_edit_singleline(&mut self.input);

                if response.lost_focus() {
                    if self.input != String::new() {
                        self.chat.push(std::mem::take(&mut self.input));
                        self.input = String::new();
                    }
                    response.request_focus();
                }
            });

        });

        // Create a top panel with text.
        egui::CentralPanel::default().show(&ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(
                    self.chat.join("\n")
                );
            });
        });
    }
}
