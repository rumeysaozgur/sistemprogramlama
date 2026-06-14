#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aitoolgrep::replace::{self, ReplaceOptions, ReplaceReport};
use aitoolgrep::search::{self, SearchOptions, SearchReport};
use aitoolgrep::stats::{self, StatsOptions, StatsReport};
use eframe::egui;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1050.0, 760.0])
            .with_min_inner_size([760.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "aitoolgrep - Kod Arama ve Degistirme",
        options,
        Box::new(|creation_context| {
            configure_style(&creation_context.egui_ctx);
            Ok(Box::new(AitoolgrepApp::default()))
        }),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveTab {
    Search,
    Replace,
    Stats,
}

struct TaskResult {
    title: String,
    output: String,
    failed: bool,
}

struct AitoolgrepApp {
    active_tab: ActiveTab,
    path: String,
    search_pattern: String,
    ignore_case: bool,
    regex: bool,
    old_text: String,
    new_text: String,
    dry_run: bool,
    backup: bool,
    json_output: bool,
    busy: bool,
    confirm_replace: bool,
    status: String,
    output: String,
    receiver: Option<Receiver<TaskResult>>,
}

impl Default for AitoolgrepApp {
    fn default() -> Self {
        let path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .display()
            .to_string();

        Self {
            active_tab: ActiveTab::Search,
            path,
            search_pattern: String::new(),
            ignore_case: false,
            regex: false,
            old_text: String::new(),
            new_text: String::new(),
            dry_run: true,
            backup: false,
            json_output: false,
            busy: false,
            confirm_replace: false,
            status: "Hazir".to_owned(),
            output: "Bir komut calistirdiginizda sonuclar burada gorunecek.".to_owned(),
            receiver: None,
        }
    }
}

impl eframe::App for AitoolgrepApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_task();
        if self.busy {
            context.request_repaint_after(Duration::from_millis(100));
        }

        egui::TopBottomPanel::top("header").show(context, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("aitoolgrep");
                ui.label("Kod arama, guvenli degistirme ve proje istatistikleri");
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("status").show(context, |ui| {
            ui.horizontal(|ui| {
                if self.busy {
                    ui.spinner();
                }
                ui.label(&self.status);
            });
        });

        egui::CentralPanel::default().show(context, |ui| {
            self.show_path_picker(ui);
            ui.add_space(8.0);
            self.show_tabs(ui);
            ui.separator();

            match self.active_tab {
                ActiveTab::Search => self.show_search(ui),
                ActiveTab::Replace => self.show_replace(ui),
                ActiveTab::Stats => self.show_stats(ui),
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.heading("Sonuc");
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Temizle"))
                    .clicked()
                {
                    self.output.clear();
                    self.status = "Sonuc alani temizlendi".to_owned();
                }
            });

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.output)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(22)
                            .interactive(false),
                    );
                });
        });

        self.show_replace_confirmation(context);
    }
}

impl AitoolgrepApp {
    fn show_path_picker(&mut self, ui: &mut egui::Ui) {
        ui.label("Dosya veya klasor yolu");
        ui.horizontal(|ui| {
            ui.add_enabled(
                !self.busy,
                egui::TextEdit::singleline(&mut self.path).desired_width(f32::INFINITY),
            );
            if ui
                .add_enabled(!self.busy, egui::Button::new("Klasor Sec"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.path = path.display().to_string();
                }
            }
            if ui
                .add_enabled(!self.busy, egui::Button::new("Dosya Sec"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.path = path.display().to_string();
                }
            }
        });
    }

    fn show_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, ActiveTab::Search, "Arama");
            ui.selectable_value(&mut self.active_tab, ActiveTab::Replace, "Degistirme");
            ui.selectable_value(&mut self.active_tab, ActiveTab::Stats, "Istatistik");
        });
    }

    fn show_search(&mut self, ui: &mut egui::Ui) {
        ui.heading("Recursive Arama");
        ui.label("Aranacak literal metni veya regex desenini girin.");
        ui.add_enabled(
            !self.busy,
            egui::TextEdit::singleline(&mut self.search_pattern)
                .hint_text("Ornek: LoginController")
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            ui.add_enabled(
                !self.busy,
                egui::Checkbox::new(&mut self.ignore_case, "Buyuk/kucuk harfi yoksay"),
            );
            ui.add_enabled(
                !self.busy,
                egui::Checkbox::new(&mut self.regex, "Regex kullan"),
            );
            ui.add_enabled(
                !self.busy,
                egui::Checkbox::new(&mut self.json_output, "JSON goster"),
            );
        });

        if ui
            .add_enabled(
                !self.busy && !self.search_pattern.is_empty() && !self.path.is_empty(),
                egui::Button::new("Aramayi Baslat"),
            )
            .clicked()
        {
            self.start_search();
        }
    }

    fn show_replace(&mut self, ui: &mut egui::Ui) {
        ui.heading("Guvenli Metin Degistirme");
        ui.columns(2, |columns| {
            columns[0].label("Eski metin");
            columns[0].add_enabled(
                !self.busy,
                egui::TextEdit::singleline(&mut self.old_text)
                    .hint_text("Ornek: oldName")
                    .desired_width(f32::INFINITY),
            );
            columns[1].label("Yeni metin");
            columns[1].add_enabled(
                !self.busy,
                egui::TextEdit::singleline(&mut self.new_text)
                    .hint_text("Ornek: newName")
                    .desired_width(f32::INFINITY),
            );
        });
        ui.horizontal(|ui| {
            ui.add_enabled(
                !self.busy,
                egui::Checkbox::new(&mut self.dry_run, "Dry-run (dosyalara yazma)"),
            );
            ui.add_enabled(
                !self.busy,
                egui::Checkbox::new(&mut self.backup, ".bak yedegi olustur"),
            );
            ui.add_enabled(
                !self.busy,
                egui::Checkbox::new(&mut self.json_output, "JSON goster"),
            );
        });

        if !self.dry_run {
            ui.colored_label(
                egui::Color32::from_rgb(190, 70, 55),
                "Dikkat: Bu mod dosyalari gercekten degistirir.",
            );
        }

        let button_text = if self.dry_run {
            "Degisiklikleri Onizle"
        } else {
            "Degisiklikleri Uygula"
        };
        if ui
            .add_enabled(
                !self.busy && !self.old_text.is_empty() && !self.path.is_empty(),
                egui::Button::new(button_text),
            )
            .clicked()
        {
            if self.dry_run {
                self.start_replace();
            } else {
                self.confirm_replace = true;
            }
        }
    }

    fn show_stats(&mut self, ui: &mut egui::Ui) {
        ui.heading("Proje Istatistikleri");
        ui.label(
            "UTF-8 metin dosyalarini tarar; binary, okunamayan ve atlanan dosyalari raporlar.",
        );
        ui.checkbox(&mut self.json_output, "JSON goster");

        if ui
            .add_enabled(
                !self.busy && !self.path.is_empty(),
                egui::Button::new("Istatistikleri Hesapla"),
            )
            .clicked()
        {
            self.start_stats();
        }
    }

    fn show_replace_confirmation(&mut self, context: &egui::Context) {
        if !self.confirm_replace {
            return;
        }

        egui::Window::new("Degisikligi Onayla")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(context, |ui| {
                ui.label("Secilen dosyalarda gercek degisiklik yapilacak.");
                ui.label("Once dry-run ile kontrol ettiginizden emin olun.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Iptal").clicked() {
                        self.confirm_replace = false;
                    }
                    if ui.button("Onayla ve Uygula").clicked() {
                        self.confirm_replace = false;
                        self.start_replace();
                    }
                });
            });
    }

    fn start_search(&mut self) {
        let options = SearchOptions {
            pattern: self.search_pattern.clone(),
            path: PathBuf::from(self.path.trim()),
            ignore_case: self.ignore_case,
            regex: self.regex,
        };
        let json_output = self.json_output;

        self.start_task("Arama yapiliyor...", move || {
            search::run(&options)
                .map(|report| TaskResult {
                    title: "Arama tamamlandi".to_owned(),
                    output: format_search_report(&report, json_output),
                    failed: false,
                })
                .unwrap_or_else(error_result)
        });
    }

    fn start_replace(&mut self) {
        let options = ReplaceOptions {
            old_text: self.old_text.clone(),
            new_text: self.new_text.clone(),
            path: PathBuf::from(self.path.trim()),
            dry_run: self.dry_run,
            backup: self.backup,
        };
        let json_output = self.json_output;

        self.start_task("Degistirme islemi calisiyor...", move || {
            replace::run(&options)
                .map(|report| TaskResult {
                    title: if report.dry_run {
                        "Dry-run tamamlandi".to_owned()
                    } else {
                        "Degistirme tamamlandi".to_owned()
                    },
                    output: format_replace_report(&report, json_output),
                    failed: report.summary.failed_files > 0,
                })
                .unwrap_or_else(error_result)
        });
    }

    fn start_stats(&mut self) {
        let options = StatsOptions {
            path: PathBuf::from(self.path.trim()),
        };
        let json_output = self.json_output;

        self.start_task("Istatistikler hesaplaniyor...", move || {
            stats::run(&options)
                .map(|report| TaskResult {
                    title: "Istatistikler hazir".to_owned(),
                    output: format_stats_report(&report, json_output),
                    failed: false,
                })
                .unwrap_or_else(error_result)
        });
    }

    fn start_task<F>(&mut self, status: &str, operation: F)
    where
        F: FnOnce() -> TaskResult + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.busy = true;
        self.status = status.to_owned();
        self.output = format!("{status}\n");

        thread::spawn(move || {
            let _ = sender.send(operation());
        });
    }

    fn poll_task(&mut self) {
        let result = self
            .receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());

        if let Some(result) = result {
            self.busy = false;
            self.receiver = None;
            self.status = if result.failed {
                format!("{} (uyarilar var)", result.title)
            } else {
                result.title
            };
            self.output = result.output;
        }
    }
}

fn format_search_report(report: &SearchReport, json_output: bool) -> String {
    if json_output {
        return pretty_json(report);
    }

    let mut output = String::new();
    for item in &report.matches {
        let _ = writeln!(output, "{}:{}: {}", item.path, item.line_number, item.line);
    }
    let _ = writeln!(
        output,
        "\nEslesme: {} | Taranan dosya: {} | Atlanan dosya: {} | Atlanan klasor: {}",
        report.summary.matches,
        report.summary.scanned_files,
        report.summary.skipped_files,
        report.summary.skipped_directories
    );
    append_errors(&mut output, &report.errors);
    output
}

fn format_replace_report(report: &ReplaceReport, json_output: bool) -> String {
    if json_output {
        return pretty_json(report);
    }

    let mut output = String::new();
    let mode = if report.dry_run { "DRY-RUN" } else { "DEGISTI" };
    for change in &report.changes {
        let _ = writeln!(
            output,
            "[{mode}] {}:{} ({} degisim)",
            change.path, change.line_number, change.replacements
        );
        let _ = writeln!(output, "- {}", change.old_line);
        let _ = writeln!(output, "+ {}\n", change.new_line);
    }
    let _ = writeln!(
        output,
        "Degisen dosya: {} | Degisen satir: {} | Degisim: {} | Yedek: {} | Basarisiz: {}",
        report.summary.changed_files,
        report.summary.changed_lines,
        report.summary.replacements,
        report.summary.backups_created,
        report.summary.failed_files
    );
    append_errors(&mut output, &report.errors);
    output
}

fn format_stats_report(report: &StatsReport, json_output: bool) -> String {
    if json_output {
        return pretty_json(report);
    }

    let stats = &report.stats;
    let mut output = format!(
        "Kok: {}\n\
         Toplam dosya: {}\n\
         Taranan dosya: {}\n\
         Atlanan dosya: {}\n\
         Binary dosya: {}\n\
         UTF-8 olmayan dosya: {}\n\
         Okunamayan dosya: {}\n\
         Atlanan klasor: {}\n\
         Toplam satir: {}\n\
         Toplam byte: {}\n",
        report.root,
        stats.total_files,
        stats.scanned_files,
        stats.skipped_files,
        stats.binary_files,
        stats.non_utf8_files,
        stats.unreadable_files,
        stats.skipped_directories,
        stats.total_lines,
        stats.total_bytes
    );
    append_errors(&mut output, &report.errors);
    output
}

fn pretty_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|error| format!("JSON olusturulamadi: {error}"))
}

fn append_errors(output: &mut String, errors: &[String]) {
    for error in errors {
        let _ = writeln!(output, "Uyari: {error}");
    }
}

fn error_result(error: anyhow::Error) -> TaskResult {
    TaskResult {
        title: "Islem basarisiz".to_owned(),
        output: format!("Hata: {error:#}"),
        failed: true,
    }
}

fn configure_style(context: &egui::Context) {
    let mut style = (*context.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    context.set_style(style);
}
