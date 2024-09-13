use eframe::egui;
use chrono::NaiveDate;
use egui_extras::DatePickerButton;

struct MyApp {
    selected_date: NaiveDate,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            selected_date: chrono::Local::now().date_naive(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Datepicker Beispiel");
            
            ui.horizontal(|ui| {
                ui.label("Ausgewähltes Datum:");
                ui.add(DatePickerButton::new(&mut self.selected_date));
            });
            
            ui.label(format!("Sie haben {} ausgewählt.", self.selected_date.format("%d.%m.%Y")));
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Datepicker Beispiel",
        native_options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
}
