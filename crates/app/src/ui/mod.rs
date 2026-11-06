//! Main window of the application.

mod duration;
mod main_screen;
mod status;

use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use eframe::egui;
use sqlrestore_core::domain::config::{AppConfig, SUPPORTED_LANGUAGES, Theme};
use sqlrestore_core::domain::restore::RestoreRequest;

use crate::commands::{
    AppCommands, RestoreEvent, RestoreOutcome, RestoreTask, SelectBackupCommandHandler,
};
use crate::i18n::I18n;

use status::Status;

/// eframe application.
pub struct SqlRestoreApp {
    commands: AppCommands,
    i18n: I18n,
    config: AppConfig,
    port_input: String,
    password: String,
    backup_path: Option<PathBuf>,
    status: Status,
    progress: f32,
    task: Option<RestoreTask>,
    pending_restore: Option<RestoreRequest>,
    select_backup: SelectBackupCommandHandler,
}

impl SqlRestoreApp {
    pub fn new(
        creation_context: &eframe::CreationContext<'_>,
        commands: AppCommands,
        i18n: I18n,
        config: AppConfig,
        password: String,
    ) -> Self {
        creation_context
            .egui_ctx
            .set_visuals(visuals_for(config.theme));
        let port_input = config.connection.port.to_string();
        let backup_path = config
            .last_backup_file
            .clone()
            .filter(|path| path.is_file());

        Self {
            commands,
            i18n,
            config,
            port_input,
            password,
            backup_path,
            status: Status::Idle,
            progress: 0.0,
            task: None,
            pending_restore: None,
            select_backup: SelectBackupCommandHandler,
        }
    }

    fn poll_restore_events(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;

        if let Some(task) = self.task.as_ref() {
            loop {
                match task.try_recv() {
                    Ok(Some(event)) => events.push(event),
                    Ok(None) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
        }

        let mut outcome = None;
        for event in events {
            match event {
                RestoreEvent::Progress(fraction) => {
                    self.progress = fraction.clamp(0.0, 1.0);
                }
                RestoreEvent::Finished(finished) => outcome = Some(finished),
            }
        }

        if let Some(outcome) = outcome {
            self.task = None;
            self.status = match outcome {
                RestoreOutcome::Success(report) => {
                    self.progress = 1.0;
                    Status::Success(report)
                }
                RestoreOutcome::Cancelled => Status::Cancelled,
                RestoreOutcome::Failed(message) => Status::Failed(message),
            };
        } else if disconnected && self.task.is_some() {
            self.task = None;
            self.status = Status::Failed(self.i18n.t("status.connection_lost"));
        }
    }

    fn draw_top_bar(&mut self, ui: &mut egui::Ui) {
        let mut selected_language = None;
        let mut toggle_theme = false;
        let context = ui.ctx().clone();

        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(self.i18n.t("app.title"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let theme_key = match self.config.theme {
                        Theme::Light => "theme.light",
                        Theme::Dark => "theme.dark",
                    };
                    let theme_text = format!(
                        "{}: {}",
                        self.i18n.t("topbar.theme"),
                        self.i18n.t(theme_key)
                    );
                    if ui.button(theme_text).clicked() {
                        toggle_theme = true;
                    }

                    let selected_text = self.i18n.t(&format!("language.{}", self.config.language));
                    egui::ComboBox::from_id_salt("language_selector")
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for language in SUPPORTED_LANGUAGES {
                                let label = self.i18n.t(&format!("language.{language}"));
                                if ui
                                    .selectable_label(*language == self.config.language, label)
                                    .clicked()
                                {
                                    selected_language = Some((*language).to_string());
                                }
                            }
                        });

                    ui.label(self.i18n.t("language.label"));
                });
            });
            ui.add_space(4.0);
        });

        if let Some(language) = selected_language {
            self.config.language = language.clone();
            self.i18n.set_language(language);
            self.persist_config();
        }

        if toggle_theme {
            self.config.theme = self.config.theme.toggled();
            context.set_visuals(visuals_for(self.config.theme));
            self.persist_config();
        }
    }

    fn persist_config(&self) {
        if let Err(error) = self.commands.save_config(self.config.clone()) {
            self.commands
                .log_warning(&format!("Configuration could not be saved: {error}"));
        }
    }
}

impl eframe::App for SqlRestoreApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_restore_events();

        if self.task.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        self.draw_top_bar(ui);

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.draw_main_screen(ui);
            });
        });

        let confirmation_context = ui.ctx().clone();
        self.draw_recreate_confirmation(&confirmation_context);
    }
}

fn visuals_for(theme: Theme) -> egui::Visuals {
    let (mut visuals, primary) = match theme {
        Theme::Light => (
            egui::Visuals::light(),
            egui::Color32::from_rgb(0x25, 0x63, 0xEB),
        ),
        Theme::Dark => (
            egui::Visuals::dark(),
            egui::Color32::from_rgb(0x3B, 0x82, 0xF6),
        ),
    };
    visuals.selection.bg_fill = primary;
    visuals.hyperlink_color = primary;
    visuals
}
