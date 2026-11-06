//! Main screen widgets.

use eframe::egui;
use sqlrestore_core::domain::config::Theme;
use sqlrestore_core::domain::restore::RestoreRequest;

use super::SqlRestoreApp;
use super::duration::format_duration;
use super::status::Status;

impl SqlRestoreApp {
    pub(super) fn draw_main_screen(&mut self, ui: &mut egui::Ui) {
        let can_edit = self.task.is_none();
        let database_from_file =
            self.config.use_file_name_as_database && self.backup_path.is_some();
        ui.add_space(8.0);
        ui.heading(self.i18n.t("connection.title"));

        egui::Grid::new("connection_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(self.i18n.t("connection.host"));
                ui.add_enabled(
                    can_edit,
                    egui::TextEdit::singleline(&mut self.config.connection.host),
                );
                ui.end_row();

                ui.label(self.i18n.t("connection.port"));
                ui.add_enabled(can_edit, egui::TextEdit::singleline(&mut self.port_input));
                ui.end_row();

                ui.label(self.i18n.t("connection.user"));
                ui.add_enabled(
                    can_edit,
                    egui::TextEdit::singleline(&mut self.config.connection.user),
                );
                ui.end_row();

                ui.label(self.i18n.t("connection.password"));
                ui.add_enabled(
                    can_edit,
                    egui::TextEdit::singleline(&mut self.password).password(true),
                );
                ui.end_row();

                ui.label(self.i18n.t("connection.database"));
                ui.add_enabled(
                    can_edit && !database_from_file,
                    egui::TextEdit::singleline(&mut self.config.connection.database),
                );
                ui.end_row();
            });

        let use_file_name_label = self.i18n.t("connection.use_file_name_as_database");
        let use_file_name_changed = ui
            .add_enabled(
                can_edit,
                egui::Checkbox::new(
                    &mut self.config.use_file_name_as_database,
                    use_file_name_label,
                ),
            )
            .changed();
        ui.label(
            egui::RichText::new(self.i18n.t("connection.use_file_name_as_database.hint")).weak(),
        );
        if use_file_name_changed {
            self.sync_database_from_backup();
            self.persist_config();
        }

        let port = self
            .port_input
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0);
        if self.port_input.trim().is_empty() || port.is_none() {
            ui.colored_label(self.error_color(), self.i18n.t("status.invalid_port"));
        }

        let recreate_label = self.i18n.t("connection.recreate_database");
        let recreate_changed = ui
            .add_enabled(
                can_edit,
                egui::Checkbox::new(&mut self.config.recreate_database, recreate_label),
            )
            .changed();
        ui.label(egui::RichText::new(self.i18n.t("connection.recreate_database.hint")).weak());
        if recreate_changed {
            self.persist_config();
        }

        ui.add_space(12.0);
        ui.heading(self.i18n.t("backup.title"));

        let path_text = self.backup_path.as_ref().map_or_else(
            || self.i18n.t("backup.no_file"),
            |path| path.display().to_string(),
        );
        let can_select = can_edit;
        let mut select_clicked = false;
        ui.horizontal(|ui| {
            ui.label(path_text);
            if ui
                .add_enabled(can_select, egui::Button::new(self.i18n.t("backup.select")))
                .clicked()
            {
                select_clicked = true;
            }
        });
        if select_clicked && let Some(path) = self.select_backup.execute() {
            self.backup_path = Some(path.clone());
            self.config.last_backup_file = Some(path);
            self.sync_database_from_backup();
            self.persist_config();
            if matches!(self.status, Status::Failed(_) | Status::Cancelled) {
                self.status = Status::Idle;
            }
        }

        ui.add_space(12.0);

        let can_restore = self.task.is_none()
            && self.config.connection.is_complete()
            && port.is_some()
            && self.backup_path.is_some();
        let can_cancel = self.task.is_some();

        let mut restore_clicked = false;
        let mut cancel_clicked = false;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    can_restore,
                    egui::Button::new(self.i18n.t("action.restore")),
                )
                .clicked()
            {
                restore_clicked = true;
            }
            if ui
                .add_enabled(can_cancel, egui::Button::new(self.i18n.t("action.cancel")))
                .clicked()
            {
                cancel_clicked = true;
            }
        });

        if restore_clicked && let Some(port) = port {
            self.start_restore(port);
        }
        if cancel_clicked {
            self.cancel_restore();
        }

        ui.add_space(12.0);
        ui.heading(self.i18n.t("result.title"));

        if self.task.is_some() {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new());
                ui.add(egui::ProgressBar::new(self.progress).show_percentage());
            });
        }

        let (message, color) = self.status_message(ui);
        ui.colored_label(color, message);
    }

    fn start_restore(&mut self, port: u16) {
        let Some(file_path) = self.backup_path.clone() else {
            return;
        };

        let mut connection = self.config.connection.clone();
        connection.port = port;
        let request = RestoreRequest {
            connection: connection.clone(),
            password: self.password.clone(),
            file_path,
            recreate_database: self.config.recreate_database,
        };

        if let Err(error) = request.validate() {
            self.status = Status::Failed(error.to_string());
            return;
        }

        if request.recreate_database {
            self.pending_restore = Some(request);
        } else {
            self.begin_restore(request);
        }
    }

    fn begin_restore(&mut self, request: RestoreRequest) {
        if let Err(error) = self.commands.save_config(self.config.clone()) {
            self.commands
                .log_warning(&format!("Configuration could not be saved: {error}"));
        }

        if !self.password.is_empty()
            && let Err(error) = self
                .commands
                .save_password(&request.connection, &self.password)
        {
            self.commands
                .log_warning(&format!("Password could not be saved: {error}"));
        }

        self.progress = 0.0;
        self.status = Status::Running;
        self.task = Some(self.commands.start_restore(request));
    }

    pub(super) fn draw_recreate_confirmation(&mut self, context: &egui::Context) {
        let Some(request) = self.pending_restore.as_ref() else {
            return;
        };
        let database = request.connection.database.clone();

        let title = self.i18n.t("recreate.title");
        let warning = self
            .i18n
            .t_args("recreate.warning", &[("database", database)]);
        let cancel_label = self.i18n.t("recreate.cancel");
        let confirm_label = self.i18n.t("recreate.confirm");

        let mut confirm = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::unique("recreate_database_confirmation")).show(context, |ui| {
            ui.set_max_width(420.0);
            ui.heading(title);
            ui.add_space(8.0);
            ui.label(warning);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(cancel_label).clicked() {
                    cancel = true;
                }
                if ui.button(confirm_label).clicked() {
                    confirm = true;
                }
            });
        });

        if confirm {
            if let Some(request) = self.pending_restore.take() {
                self.begin_restore(request);
            }
        } else if cancel {
            self.pending_restore = None;
        }
    }

    fn cancel_restore(&mut self) {
        let Some(task) = self.task.as_ref() else {
            return;
        };
        if let Err(error) = task.cancel() {
            self.commands
                .log_warning(&format!("Restore could not be cancelled: {error}"));
        }
    }

    fn status_message(&self, ui: &egui::Ui) -> (String, egui::Color32) {
        match &self.status {
            Status::Idle => (self.i18n.t("status.idle"), ui.visuals().weak_text_color()),
            Status::Running => (self.i18n.t("status.running"), self.primary_color()),
            Status::Success(report) => (
                self.i18n.t_args(
                    "status.success",
                    &[
                        ("statements", report.statements.to_string()),
                        ("bytes", report.bytes.to_string()),
                        ("duration", format_duration(&self.i18n, report.elapsed)),
                    ],
                ),
                self.success_color(),
            ),
            Status::Cancelled => (self.i18n.t("status.cancelled"), self.primary_color()),
            Status::Failed(error) => (
                self.i18n
                    .t_args("status.failed", &[("error", error.clone())]),
                self.error_color(),
            ),
        }
    }

    fn primary_color(&self) -> egui::Color32 {
        match self.config.theme {
            Theme::Light => egui::Color32::from_rgb(0x25, 0x63, 0xEB),
            Theme::Dark => egui::Color32::from_rgb(0x3B, 0x82, 0xF6),
        }
    }

    fn success_color(&self) -> egui::Color32 {
        match self.config.theme {
            Theme::Light => egui::Color32::from_rgb(0x16, 0xA3, 0x4A),
            Theme::Dark => egui::Color32::from_rgb(0x22, 0xC5, 0x5E),
        }
    }

    fn error_color(&self) -> egui::Color32 {
        match self.config.theme {
            Theme::Light => egui::Color32::from_rgb(0xDC, 0x26, 0x26),
            Theme::Dark => egui::Color32::from_rgb(0xEF, 0x44, 0x44),
        }
    }
}
