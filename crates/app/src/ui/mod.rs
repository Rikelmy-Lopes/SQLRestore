//! Main window of the application.

mod date;
mod duration;
mod history;
mod icon;
mod main_screen;
mod status;
mod style;
mod widgets;

pub(crate) use icon::window_icon;

use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use eframe::egui;
use sqlrestore_core::domain::config::{
    AppConfig, SUPPORTED_LANGUAGES, Theme, database_name_from_backup,
};
use sqlrestore_core::domain::history::RestoreHistoryEntry;
use sqlrestore_core::domain::restore::RestoreRequest;

use crate::commands::{
    AppCommands, ConnectionTestTask, RestoreEvent, RestoreOutcome, RestoreTask,
    SelectBackupCommandHandler,
};
use crate::i18n::I18n;

use status::{ConnectionStatus, Status};
use style::Palette;

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
    connection_test: Option<ConnectionTestTask>,
    connection_status: ConnectionStatus,
    pending_restore: Option<RestoreRequest>,
    select_backup: SelectBackupCommandHandler,
    history_open: bool,
    history_entries: Vec<RestoreHistoryEntry>,
    history_error: Option<String>,
    history_clear_confirmation: bool,
}

impl SqlRestoreApp {
    pub fn new(
        creation_context: &eframe::CreationContext<'_>,
        commands: AppCommands,
        i18n: I18n,
        config: AppConfig,
        password: String,
    ) -> Self {
        style::apply_style(&creation_context.egui_ctx, config.theme);
        let port_input = config.connection.port.to_string();
        let backup_path = config
            .last_backup_file
            .clone()
            .filter(|path| path.is_file());
        let mut config = config;
        if config.use_file_name_as_database
            && let Some(name) = backup_path.as_deref().and_then(database_name_from_backup)
        {
            config.connection.database = name;
        }

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
            connection_test: None,
            connection_status: ConnectionStatus::Idle,
            pending_restore: None,
            select_backup: SelectBackupCommandHandler,
            history_open: false,
            history_entries: Vec::new(),
            history_error: None,
            history_clear_confirmation: false,
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

    fn poll_connection_test(&mut self) {
        let Some(task) = self.connection_test.as_ref() else {
            return;
        };

        match task.try_recv() {
            Ok(Some(Ok(()))) => {
                self.connection_status = ConnectionStatus::Success;
                self.connection_test = None;
            }
            Ok(Some(Err(error))) => {
                self.connection_status = ConnectionStatus::Failed(error);
                self.connection_test = None;
            }
            Ok(None) => {}
            Err(TryRecvError::Disconnected) => {
                self.connection_status =
                    ConnectionStatus::Failed(self.i18n.t("status.connection_test_lost"));
                self.connection_test = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn draw_top_bar(&mut self, ui: &mut egui::Ui) {
        let mut selected_language = None;
        let mut toggle_theme = false;
        let mut open_history = false;
        let context = ui.ctx().clone();
        let palette = Palette::for_theme(self.config.theme);

        egui::Panel::top("top_bar")
            .frame(
                egui::Frame::new()
                    .fill(palette.panel)
                    .inner_margin(egui::Margin::symmetric(16, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(self.i18n.t("app.title"))
                            .size(18.0)
                            .strong()
                            .color(palette.primary),
                    );
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

                        if ui.button(self.i18n.t("history.open")).clicked() {
                            open_history = true;
                        }

                        let selected_text =
                            self.i18n.t(&format!("language.{}", self.config.language));
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
            });

        if let Some(language) = selected_language {
            self.config.language = language.clone();
            self.i18n.set_language(language);
            self.persist_config();
        }

        if toggle_theme {
            self.config.theme = self.config.theme.toggled();
            style::apply_style(&context, self.config.theme);
            self.persist_config();
        }

        if open_history {
            self.open_history();
        }
    }

    fn persist_config(&self) {
        if let Err(error) = self.commands.save_config(self.config.clone()) {
            self.commands
                .log_warning(&format!("Configuration could not be saved: {error}"));
        }
    }

    /// Copies the selected backup file name into the database field when the
    /// option is enabled.
    pub(super) fn sync_database_from_backup(&mut self) {
        if self.config.use_file_name_as_database
            && let Some(name) = self
                .backup_path
                .as_deref()
                .and_then(database_name_from_backup)
        {
            self.config.connection.database = name;
        }
    }
}

impl eframe::App for SqlRestoreApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_restore_events();
        self.poll_connection_test();

        if self.task.is_some() || self.connection_test.is_some() {
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
        let history_context = ui.ctx().clone();
        self.draw_history_dialog(&history_context);
    }
}
