//! Main screen widgets.

use eframe::egui;
use sqlrestore_core::domain::restore::RestoreRequest;

use super::SqlRestoreApp;
use super::duration::format_duration;
use super::status::{ConnectionStatus, Status};
use super::style::Palette;
use super::widgets::{self, ButtonKind, MessageTone, TextField};

const CONTENT_MAX_WIDTH: f32 = 620.0;
const COLUMN_GAP: f32 = 12.0;

impl SqlRestoreApp {
    pub(super) fn draw_main_screen(&mut self, ui: &mut egui::Ui) {
        let palette = Palette::for_theme(self.config.theme);
        let available = ui.available_width();
        let content_width = available.min(CONTENT_MAX_WIDTH);
        let side_margin = ((available - content_width) / 2.0).max(0.0);

        ui.horizontal(|ui| {
            ui.add_space(side_margin);
            ui.vertical(|ui| {
                ui.set_width(content_width);
                self.draw_content(ui, &palette);
            });
        });
    }

    fn draw_content(&mut self, ui: &mut egui::Ui, palette: &Palette) {
        let can_edit = self.task.is_none();
        let database_from_file =
            self.config.use_file_name_as_database && self.backup_path.is_some();
        let port = self
            .port_input
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0);
        let port_is_valid = !self.port_input.trim().is_empty() && port.is_some();

        let mut use_file_name_changed = false;
        let mut recreate_changed = false;
        let mut connection_changed = false;
        let mut select_clicked = false;
        let mut test_clicked = false;
        let mut restore_clicked = false;
        let mut cancel_clicked = false;

        let connection_title = self.i18n.t("connection.title");
        let host_label = self.i18n.t("connection.host");
        let host_hint = self.i18n.t("connection.host.hint");
        let port_label = self.i18n.t("connection.port");
        let port_hint = self.i18n.t("connection.port.hint");
        let user_label = self.i18n.t("connection.user");
        let user_hint = self.i18n.t("connection.user.hint");
        let password_label = self.i18n.t("connection.password");
        let password_hint = self.i18n.t("connection.password.hint");
        let database_label = self.i18n.t("connection.database");
        let database_hint = self.i18n.t("connection.database.hint");
        let invalid_port = self.i18n.t("status.invalid_port");
        let use_file_name_label = self.i18n.t("connection.use_file_name_as_database");
        let use_file_name_hint = self.i18n.t("connection.use_file_name_as_database.hint");
        let recreate_label = self.i18n.t("connection.recreate_database");
        let recreate_hint = self.i18n.t("connection.recreate_database.hint");
        let test_label = self.i18n.t("connection.test");

        ui.add_space(12.0);

        widgets::section_card(ui, palette, |ui| {
            widgets::section_title(ui, palette, &connection_title);

            let width = ui.available_width();
            let host_width = ((width - COLUMN_GAP) * 0.66).max(120.0);
            let port_width = (width - COLUMN_GAP - host_width).max(90.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = COLUMN_GAP;
                connection_changed |= TextField::new(
                    &host_label,
                    &host_hint,
                    &mut self.config.connection.host,
                    host_width,
                )
                .enabled(can_edit)
                .show(ui, palette)
                .changed();
                connection_changed |=
                    TextField::new(&port_label, &port_hint, &mut self.port_input, port_width)
                        .enabled(can_edit)
                        .show(ui, palette)
                        .changed();
            });
            ui.add_space(1.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = COLUMN_GAP;
                connection_changed |= TextField::new(
                    &user_label,
                    &user_hint,
                    &mut self.config.connection.user,
                    host_width,
                )
                .enabled(can_edit)
                .show(ui, palette)
                .changed();
                connection_changed |= TextField::new(
                    &password_label,
                    &password_hint,
                    &mut self.password,
                    port_width,
                )
                .enabled(can_edit)
                .password(true)
                .show(ui, palette)
                .changed();
            });
            ui.add_space(1.0);

            connection_changed |= TextField::new(
                &database_label,
                &database_hint,
                &mut self.config.connection.database,
                width,
            )
            .enabled(can_edit && !database_from_file)
            .show(ui, palette)
            .changed();

            if !port_is_valid {
                ui.add_space(4.0);
                widgets::error_text(ui, palette, &invalid_port);
            }

            ui.add_space(8.0);
            use_file_name_changed = ui
                .add_enabled(
                    can_edit,
                    egui::Checkbox::new(
                        &mut self.config.use_file_name_as_database,
                        use_file_name_label.as_str(),
                    ),
                )
                .changed();
            widgets::hint_text(ui, palette, &use_file_name_hint);

            ui.add_space(6.0);
            recreate_changed = ui
                .add_enabled(
                    can_edit,
                    egui::Checkbox::new(
                        &mut self.config.recreate_database,
                        recreate_label.as_str(),
                    ),
                )
                .changed();
            widgets::hint_text(ui, palette, &recreate_hint);

            ui.add_space(10.0);
            let can_test = can_edit
                && self.connection_test.is_none()
                && self.config.connection.is_complete()
                && port_is_valid;
            if widgets::button(
                ui,
                palette,
                &test_label,
                ButtonKind::Secondary,
                can_test,
                150.0,
            )
            .clicked()
            {
                test_clicked = true;
            }
            self.draw_connection_status(ui, palette);
        });

        ui.add_space(12.0);

        let backup_title = self.i18n.t("backup.title");
        let select_label = self.i18n.t("backup.select");
        let path_text = self.backup_path.as_ref().map_or_else(
            || self.i18n.t("backup.no_file"),
            |path| path.display().to_string(),
        );

        widgets::section_card(ui, palette, |ui| {
            widgets::section_title(ui, palette, &backup_title);
            let width = ui.available_width();
            let select_width = 112.0;
            let path_width = (width - select_width - 12.0).max(120.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                widgets::path_box(
                    ui,
                    palette,
                    &path_text,
                    path_width,
                    self.backup_path.is_some(),
                );
                if widgets::button(
                    ui,
                    palette,
                    &select_label,
                    ButtonKind::Secondary,
                    can_edit,
                    select_width,
                )
                .clicked()
                {
                    select_clicked = true;
                }
            });
        });

        ui.add_space(12.0);

        let can_restore = self.task.is_none()
            && self.config.connection.is_complete()
            && port.is_some()
            && self.backup_path.is_some();
        let can_cancel = self.task.is_some();
        let restore_label = self.i18n.t("action.restore");
        let cancel_label = self.i18n.t("action.cancel");

        let restore_width = 150.0;
        let cancel_width = 118.0;
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                let actions_width = restore_width + 12.0 + cancel_width;
                ui.spacing_mut().item_spacing.x = 12.0;
                let side = ((ui.available_width() - actions_width) / 2.0
                    - ui.spacing().item_spacing.x)
                    .max(0.0);
                ui.add_space(side);
                if widgets::button(
                    ui,
                    palette,
                    &restore_label,
                    ButtonKind::Primary,
                    can_restore,
                    restore_width,
                )
                .clicked()
                {
                    restore_clicked = true;
                }
                if widgets::button(
                    ui,
                    palette,
                    &cancel_label,
                    ButtonKind::Secondary,
                    can_cancel,
                    cancel_width,
                )
                .clicked()
                {
                    cancel_clicked = true;
                }
            });
        });

        ui.add_space(12.0);

        let result_title = self.i18n.t("result.title");
        widgets::section_card(ui, palette, |ui| {
            widgets::section_title(ui, palette, &result_title);

            if self.task.is_some() {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(16.0).color(palette.primary));
                    ui.add(
                        egui::ProgressBar::new(self.progress)
                            .desired_width(ui.available_width())
                            .desired_height(16.0)
                            .corner_radius(egui::CornerRadius::same(8))
                            .fill(palette.primary)
                            .show_percentage(),
                    );
                });
                ui.add_space(12.0);
            }

            let (message, tone) = self.status_message();
            widgets::message_banner(ui, palette, &message, tone);
        });

        ui.add_space(16.0);

        if use_file_name_changed {
            self.sync_database_from_backup();
            self.persist_config();
        }
        if recreate_changed {
            self.persist_config();
        }
        if connection_changed && self.connection_test.is_none() {
            self.connection_status = ConnectionStatus::Idle;
        }
        if test_clicked && let Some(port) = port {
            self.start_connection_test(port);
        }
        if select_clicked && let Some(path) = self.select_backup.execute() {
            self.backup_path = Some(path.clone());
            self.config.last_backup_file = Some(path);
            self.sync_database_from_backup();
            self.persist_config();
            if matches!(self.status, Status::Failed(_) | Status::Cancelled) {
                self.status = Status::Idle;
            }
        }
        if restore_clicked && let Some(port) = port {
            self.start_restore(port);
        }
        if cancel_clicked {
            self.cancel_restore();
        }
    }

    fn draw_connection_status(&self, ui: &mut egui::Ui, palette: &Palette) {
        let (message, tone) = match &self.connection_status {
            ConnectionStatus::Idle => return,
            ConnectionStatus::Testing => {
                (self.i18n.t("connection.test.testing"), MessageTone::Info)
            }
            ConnectionStatus::Success => {
                (self.i18n.t("connection.test.success"), MessageTone::Success)
            }
            ConnectionStatus::Failed(error) => (
                self.i18n
                    .t_args("connection.test.failed", &[("error", error.clone())]),
                MessageTone::Error,
            ),
        };

        ui.add_space(8.0);
        widgets::message_banner(ui, palette, &message, tone);
    }

    fn start_connection_test(&mut self, port: u16) {
        let mut connection = self.config.connection.clone();
        connection.port = port;
        self.connection_status = ConnectionStatus::Testing;
        self.connection_test = Some(self.commands.test_connection(
            connection,
            self.password.clone(),
            self.config.recreate_database,
        ));
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
        let palette = Palette::for_theme(self.config.theme);

        let title = self.i18n.t("recreate.title");
        let warning = self
            .i18n
            .t_args("recreate.warning", &[("database", database)]);
        let cancel_label = self.i18n.t("recreate.cancel");
        let confirm_label = self.i18n.t("recreate.confirm");

        let mut confirm = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::unique("recreate_database_confirmation")).show(context, |ui| {
            ui.set_max_width(400.0);
            ui.label(
                egui::RichText::new(title)
                    .size(17.0)
                    .strong()
                    .color(palette.error),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(warning)
                    .size(13.5)
                    .color(palette.weak_text),
            );
            ui.add_space(12.0);
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    let cancel_width = 118.0;
                    let confirm_width = 150.0;
                    let actions_width = cancel_width + 12.0 + confirm_width;
                    ui.spacing_mut().item_spacing.x = 12.0;
                    let side = ((ui.available_width() - actions_width) / 2.0
                        - ui.spacing().item_spacing.x)
                        .max(0.0);
                    ui.add_space(side);
                    if widgets::button(
                        ui,
                        &palette,
                        &cancel_label,
                        ButtonKind::Secondary,
                        true,
                        cancel_width,
                    )
                    .clicked()
                    {
                        cancel = true;
                    }
                    if widgets::button(
                        ui,
                        &palette,
                        &confirm_label,
                        ButtonKind::Danger,
                        true,
                        confirm_width,
                    )
                    .clicked()
                    {
                        confirm = true;
                    }
                });
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

    fn status_message(&self) -> (String, MessageTone) {
        match &self.status {
            Status::Idle => (self.i18n.t("status.idle"), MessageTone::Neutral),
            Status::Running => (self.i18n.t("status.running"), MessageTone::Info),
            Status::Success(report) => (
                self.i18n.t_args(
                    "status.success",
                    &[
                        ("statements", report.statements.to_string()),
                        ("bytes", report.bytes.to_string()),
                        ("duration", format_duration(&self.i18n, report.elapsed)),
                    ],
                ),
                MessageTone::Success,
            ),
            Status::Cancelled => (self.i18n.t("status.cancelled"), MessageTone::Info),
            Status::Failed(error) => (
                self.i18n
                    .t_args("status.failed", &[("error", error.clone())]),
                MessageTone::Error,
            ),
        }
    }
}
