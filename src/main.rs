use eframe::egui;
use chrono::{NaiveDate, NaiveTime, Local, Duration};
use egui_extras::DatePickerButton;
use rusqlite::{Connection, Result};
use std::path::Path;
use notify_rust::Notification;
use tokio::time::interval;
use std::cell::RefCell;
use std::process::Command;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Termin {
    id: Option<i64>,
    datum: NaiveDate,
    uhrzeit: NaiveTime,
    beschreibung: String,
    ort: String,
}

struct MyApp {
    termine: Arc<Mutex<Vec<Termin>>>,
    neuer_termin: Termin,
    db_conn: RefCell<Connection>,
    uhrzeit_input: String,
}

impl MyApp {
    fn new() -> Result<Self> {
        Command::new("notify-send")
            .arg("Terminplaner")
            .arg("Ich bin jetzt gestartet")
            .spawn()
            .expect("Fehler beim Senden der Startbenachrichtigung");

        let db_path = "termine.db";
        let db_exists = Path::new(db_path).exists();
        let db_conn = Connection::open(db_path)?;

        if !db_exists {
            db_conn.execute(
                "CREATE TABLE termine (
                    id INTEGER PRIMARY KEY,
                    datum TEXT NOT NULL,
                    uhrzeit TEXT NOT NULL,
                    beschreibung TEXT NOT NULL,
                    ort TEXT NOT NULL
                )",
                [],
            )?;
        }

        let mut termine = Vec::new();
        {
            let mut stmt = db_conn.prepare("SELECT id, datum, uhrzeit, beschreibung, ort FROM termine")?;
            let termin_iter = stmt.query_map([], |row| {
                Ok(Termin {
                    id: Some(row.get(0)?),
                    datum: NaiveDate::parse_from_str(&row.get::<_, String>(1)?, "%Y-%m-%d").unwrap(),
                    uhrzeit: NaiveTime::parse_from_str(&row.get::<_, String>(2)?, "%H:%M").unwrap(),
                    beschreibung: row.get(3)?,
                    ort: row.get(4)?,
                })
            })?;

            for termin in termin_iter {
                termine.push(termin?);
            }
        }

        Ok(Self {
            termine: Arc::new(Mutex::new(termine)),
            neuer_termin: Termin {
                id: None,
                datum: Local::now().date_naive(),
                uhrzeit: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                beschreibung: String::new(),
                ort: String::new(),
            },
            db_conn: RefCell::new(db_conn),
            uhrzeit_input: "09:00".to_string(),
        })
    }

    fn speichere_termin(&self, termin: &Termin) -> Result<i64> {
        let mut conn = self.db_conn.borrow_mut();
        conn.execute(
            "INSERT INTO termine (datum, uhrzeit, beschreibung, ort) VALUES (?1, ?2, ?3, ?4)",
            (
                &termin.datum.format("%Y-%m-%d").to_string(),
                &termin.uhrzeit.format("%H:%M").to_string(),
                &termin.beschreibung,
                &termin.ort,
            ),
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn loesche_termin(&self, id: i64) -> Result<()> {
        self.db_conn.borrow_mut().execute("DELETE FROM termine WHERE id = ?1", [id])?;
        Ok(())
    }

    fn ueberwache_termine(termine: Arc<Mutex<Vec<Termin>>>) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::seconds(10).to_std().unwrap());
            loop {
                interval.tick().await;
                let jetzt = Local::now();
                let termine = termine.lock().unwrap();
                for termin in termine.iter() {
                    let termin_datetime = termin.datum.and_time(termin.uhrzeit);
                    if termin_datetime.and_local_timezone(jetzt.timezone()).unwrap() <= jetzt {
                        println!("Überfälliger Termin: {} um {} in {}", termin.beschreibung, termin.uhrzeit.format("%H:%M"), termin.ort);
                        Notification::new()
                            .summary("Termin-Erinnerung")
                            .body(&format!("{}: {} in {}", termin.beschreibung, termin.uhrzeit.format("%H:%M"), termin.ort))
                            .show()
                            .unwrap();
                    }
                }
            }
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Terminplaner");
            
            ui.horizontal(|ui| {
                ui.label("Datum:");
                ui.add(DatePickerButton::new(&mut self.neuer_termin.datum));
            });
            
            ui.horizontal(|ui| {
                ui.label("Uhrzeit:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.uhrzeit_input)
                    .show_ui(ui, |ui| {
                        for hour in 0..24 {
                            for minute in [0, 15, 30, 45] {
                                let time = format!("{:02}:{:02}", hour, minute);
                                ui.selectable_value(&mut self.uhrzeit_input, time.clone(), time);
                            }
                        }
                    });
                if ui.text_edit_singleline(&mut self.uhrzeit_input).changed() {
                    if let Ok(time) = NaiveTime::parse_from_str(&self.uhrzeit_input, "%H:%M") {
                        self.neuer_termin.uhrzeit = time;
                    }
                }
            });
            
            ui.horizontal(|ui| {
                ui.label("Beschreibung:");
                ui.text_edit_singleline(&mut self.neuer_termin.beschreibung);
            });
            
            ui.horizontal(|ui| {
                ui.label("Ort:");
                ui.text_edit_singleline(&mut self.neuer_termin.ort);
            });
            
            if ui.button("Termin hinzufügen").clicked() {
                let mut neuer_termin = self.neuer_termin.clone();
                if let Ok(id) = self.speichere_termin(&neuer_termin) {
                    neuer_termin.id = Some(id);
                    self.termine.lock().unwrap().push(neuer_termin);
                    self.neuer_termin.beschreibung.clear();
                    self.neuer_termin.ort.clear();
                    self.uhrzeit_input = "09:00".to_string();
                }
            }
            
            ui.separator();
            
            ui.heading("Termine:");
            let mut zu_loeschen = None;
            for (index, termin) in self.termine.lock().unwrap().iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{} {}: {} in {}",
                        termin.datum.format("%d.%m.%Y"),
                        termin.uhrzeit.format("%H:%M"),
                        termin.beschreibung,
                        termin.ort
                    ));
                    if ui.button("Löschen").clicked() {
                        if let Some(id) = termin.id {
                            if self.loesche_termin(id).is_ok() {
                                zu_loeschen = Some(index);
                            }
                        }
                    }
                });
            }
            if let Some(index) = zu_loeschen {
                self.termine.lock().unwrap().remove(index);
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = MyApp::new()?;
    let termine = app.termine.clone();
    MyApp::ueberwache_termine(termine);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Terminplaner",
        native_options,
        Box::new(|_cc| Box::new(app)),
    ).unwrap();
    Ok(())
}
