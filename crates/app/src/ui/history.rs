//! Restore history dialog.

use std::time::Duration;

use eframe::egui;

use super::SqlRestoreApp;
use super::date::{format_date, format_time};
use super::duration::format_duration;
use super::style::Palette;
use super::widgets::{self, ButtonKind, MessageTone};

impl SqlRestoreApp {
    /// Loads the history and opens the dialog.
    pub(super) fn open_history(&mut self) {
        match self.commands.load_history() {
            Ok(entries) => {
                self.history_entries = entries;
                self.history_error = None;
            }
            Err(error) => {
                self.commands
                    .log_warning(&format!("Restore history could not be loaded: {error}"));
                self.history_entries.clear();
                self.history_error = Some(error.to_string());
            }
        }
        self.history_clear_confirmation = false;
        self.history_open = true;
    }

    pub(super) fn draw_history_dialog(&mut self, context: &egui::Context) {
        if !self.history_open {
            return;
        }

        self.draw_history_entries(context);

        if self.history_clear_confirmation {
            self.draw_history_clear_confirmation(context);
        }
    }

    fn draw_history_entries(&mut self, context: &egui::Context) {
        let palette = Palette::for_theme(self.config.theme);
        let width = (context.content_rect().width() - 120.0).clamp(360.0, 760.0);
        let title = self.i18n.t("history.title");
        let empty = self.i18n.t("history.empty");
        let close_label = self.i18n.t("history.close");
        let clear_label = self.i18n.t("history.clear");
        let i18n = &self.i18n;
        let entries = &self.history_entries;
        let error = self.history_error.as_deref();

        let mut close = false;
        let mut request_clear = false;
        let response =
            egui::Modal::new(egui::Id::unique("restore_history_dialog")).show(context, |ui| {
                ui.set_width(width);
                ui.label(
                    egui::RichText::new(&title)
                        .size(17.0)
                        .strong()
                        .color(palette.text),
                );
                ui.add_space(8.0);

                if let Some(error) = error {
                    widgets::message_banner(ui, &palette, error, MessageTone::Error);
                    ui.add_space(8.0);
                }

                if entries.is_empty() {
                    ui.label(
                        egui::RichText::new(&empty)
                            .size(13.5)
                            .color(palette.weak_text),
                    );
                } else {
                    egui::ScrollArea::both().max_height(420.0).show(ui, |ui| {
                        egui::Grid::new("restore_history_grid")
                            .num_columns(6)
                            .striped(true)
                            .spacing([16.0, 8.0])
                            .show(ui, |ui| {
                                for key in [
                                    "history.date",
                                    "history.time",
                                    "history.file",
                                    "history.database",
                                    "history.connection",
                                    "history.duration",
                                ] {
                                    ui.label(
                                        egui::RichText::new(i18n.t(key))
                                            .size(11.5)
                                            .strong()
                                            .color(palette.weak_text),
                                    );
                                }
                                ui.end_row();

                                for entry in entries.iter().rev() {
                                    cell(ui, &palette, &format_date(i18n, &entry.date));
                                    cell(ui, &palette, &format_time(i18n, &entry.time));
                                    cell(ui, &palette, &entry.file_name);
                                    cell(ui, &palette, &entry.database);
                                    cell(ui, &palette, &entry.connection);
                                    cell(
                                        ui,
                                        &palette,
                                        &format_duration(
                                            i18n,
                                            Duration::from_millis(entry.duration_ms),
                                        ),
                                    );
                                    ui.end_row();
                                }
                            });
                    });
                }

                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        let clear_width = 130.0;
                        let close_width = 118.0;
                        let actions_width = clear_width + 12.0 + close_width;
                        ui.spacing_mut().item_spacing.x = 12.0;
                        let side = ((ui.available_width() - actions_width) / 2.0
                            - ui.spacing().item_spacing.x)
                            .max(0.0);
                        ui.add_space(side);
                        if widgets::button(
                            ui,
                            &palette,
                            &clear_label,
                            ButtonKind::Danger,
                            !entries.is_empty(),
                            clear_width,
                        )
                        .clicked()
                        {
                            request_clear = true;
                        }
                        if widgets::button(
                            ui,
                            &palette,
                            &close_label,
                            ButtonKind::Secondary,
                            true,
                            close_width,
                        )
                        .clicked()
                        {
                            close = true;
                        }
                    });
                });
            });

        if request_clear {
            self.history_clear_confirmation = true;
        }
        if close || response.should_close() {
            self.history_open = false;
            self.history_clear_confirmation = false;
        }
    }

    fn draw_history_clear_confirmation(&mut self, context: &egui::Context) {
        let palette = Palette::for_theme(self.config.theme);
        let title = self.i18n.t("history.clear.title");
        let warning = self.i18n.t("history.clear.warning");
        let cancel_label = self.i18n.t("history.clear.cancel");
        let confirm_label = self.i18n.t("history.clear.confirm");

        let mut confirm = false;
        let mut cancel = false;
        let response =
            egui::Modal::new(egui::Id::unique("clear_history_confirmation")).show(context, |ui| {
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
            match self.commands.clear_history() {
                Ok(()) => {
                    self.history_entries.clear();
                    self.history_error = None;
                }
                Err(error) => {
                    self.commands
                        .log_warning(&format!("Restore history could not be cleared: {error}"));
                    self.history_error = Some(error.to_string());
                }
            }
            self.history_clear_confirmation = false;
        } else if cancel || response.should_close() {
            self.history_clear_confirmation = false;
        }
    }
}

fn cell(ui: &mut egui::Ui, palette: &Palette, text: &str) {
    ui.label(egui::RichText::new(text).size(12.5).color(palette.text));
}
