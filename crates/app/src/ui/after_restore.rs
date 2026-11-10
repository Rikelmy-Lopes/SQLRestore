//! After-restore script dialog.

use eframe::egui;

use super::SqlRestoreApp;
use super::style::Palette;
use super::widgets::{self, ButtonKind};

impl SqlRestoreApp {
    /// Opens the dialog with the currently saved script.
    pub(super) fn open_after_restore_dialog(&mut self) {
        self.after_restore_draft = self.config.after_restore_script.clone();
        self.after_restore_dialog_open = true;
    }

    pub(super) fn draw_after_restore_dialog(&mut self, context: &egui::Context) {
        if !self.after_restore_dialog_open {
            return;
        }

        let palette = Palette::for_theme(self.config.theme);
        let title = self.i18n.t("after_restore.dialog.title");
        let hint = self.i18n.t("after_restore.hint");
        let placeholder = self.i18n.t("after_restore.placeholder");
        let save_label = self.i18n.t("after_restore.save");
        let cancel_label = self.i18n.t("after_restore.cancel");
        let clear_label = self.i18n.t("after_restore.clear");

        let mut draft = std::mem::take(&mut self.after_restore_draft);
        let mut save = false;
        let mut clear = false;
        let mut cancel = false;

        let response =
            egui::Modal::new(egui::Id::unique("after_restore_script_dialog")).show(context, |ui| {
                ui.set_width(560.0);
                ui.label(
                    egui::RichText::new(&title)
                        .size(17.0)
                        .strong()
                        .color(palette.text),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(&hint)
                        .size(13.0)
                        .color(palette.weak_text),
                );
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut draft)
                                .hint_text(placeholder)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(12)
                                .background_color(palette.input)
                                .text_color(palette.text),
                        );
                    });

                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        let clear_width = 140.0;
                        let cancel_width = 118.0;
                        let save_width = 150.0;
                        let actions_width = clear_width + 12.0 + cancel_width + 12.0 + save_width;
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
                            !draft.trim().is_empty(),
                            clear_width,
                        )
                        .clicked()
                        {
                            clear = true;
                        }
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
                            &save_label,
                            ButtonKind::Primary,
                            true,
                            save_width,
                        )
                        .clicked()
                        {
                            save = true;
                        }
                    });
                });
            });

        if save {
            self.config.after_restore_script = draft.trim().to_string();
            self.after_restore_dialog_open = false;
            self.persist_config();
        } else if clear {
            self.config.after_restore_script.clear();
            draft.clear();
            self.after_restore_dialog_open = false;
            self.persist_config();
        } else if cancel || response.should_close() {
            self.after_restore_dialog_open = false;
        }

        self.after_restore_draft = draft;
    }
}
