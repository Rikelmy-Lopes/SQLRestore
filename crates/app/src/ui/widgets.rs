//! Reusable styled widgets for the main screen.

use eframe::egui;

use super::style::Palette;

/// Height of the styled text inputs.
pub const FIELD_HEIGHT: f32 = 40.0;

/// Tone of a status message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageTone {
    Neutral,
    Info,
    Success,
    Error,
}

impl MessageTone {
    fn color(self, palette: &Palette) -> egui::Color32 {
        match self {
            Self::Neutral => palette.weak_text,
            Self::Info => palette.primary,
            Self::Success => palette.success,
            Self::Error => palette.error,
        }
    }
}

/// Draws a rounded panel with a title area.
pub fn section_card<R>(
    ui: &mut egui::Ui,
    palette: &Palette,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(palette.card)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        })
        .inner
}

/// Draws a section title.
pub fn section_title(ui: &mut egui::Ui, palette: &Palette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(18.0)
            .strong()
            .color(palette.text),
    );
    ui.add_space(6.0);
}

/// Draws a small weak-colored hint.
pub fn hint_text(ui: &mut egui::Ui, palette: &Palette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .color(palette.weak_text),
    );
}

/// Draws a small validation error.
pub fn error_text(ui: &mut egui::Ui, palette: &Palette, text: &str) {
    ui.label(egui::RichText::new(text).size(12.0).color(palette.error));
}

/// Builder for a labelled text input.
pub struct TextField<'a> {
    label: &'a str,
    hint: &'a str,
    value: &'a mut String,
    width: f32,
    enabled: bool,
    password: bool,
}

impl<'a> TextField<'a> {
    pub fn new(label: &'a str, hint: &'a str, value: &'a mut String, width: f32) -> Self {
        Self {
            label,
            hint,
            value,
            width,
            enabled: true,
            password: false,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Draws the label above a rounded input.
    pub fn show(self, ui: &mut egui::Ui, palette: &Palette) -> egui::Response {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.set_width(self.width);
            ui.label(
                egui::RichText::new(self.label)
                    .size(12.0)
                    .strong()
                    .color(palette.weak_text),
            );
            ui.add_space(2.0);

            let edit = egui::TextEdit::singleline(self.value)
                .hint_text(self.hint.to_string())
                .margin(egui::Margin::symmetric(12, 9))
                .desired_width(self.width)
                .min_size(egui::vec2(self.width, FIELD_HEIGHT))
                .background_color(palette.input)
                .text_color(palette.text)
                .password(self.password);

            ui.add_enabled(self.enabled, edit)
        })
        .inner
    }
}

/// Draws a read-only box that shows the selected backup path.
pub fn path_box(ui: &mut egui::Ui, palette: &Palette, text: &str, width: f32, has_file: bool) {
    egui::Frame::new()
        .fill(palette.input)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width((width - 24.0).max(0.0));
            ui.set_height(FIELD_HEIGHT - 20.0);
            let color = if has_file {
                palette.text
            } else {
                palette.weak_text
            };
            ui.add(egui::Label::new(egui::RichText::new(text).size(14.0).color(color)).truncate());
        });
}

/// Button variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
}

/// Draws a styled button.
pub fn button(
    ui: &mut egui::Ui,
    palette: &Palette,
    text: &str,
    kind: ButtonKind,
    enabled: bool,
    min_width: f32,
) -> egui::Response {
    let (fill, text_color, stroke) = if enabled {
        match kind {
            ButtonKind::Primary => (
                palette.primary,
                egui::Color32::WHITE,
                egui::Stroke::new(1.0, palette.primary),
            ),
            ButtonKind::Secondary => (
                palette.card,
                palette.text,
                egui::Stroke::new(1.0, palette.border),
            ),
        }
    } else {
        (
            palette.panel,
            palette.weak_text,
            egui::Stroke::new(1.0, palette.border),
        )
    };

    let button = egui::Button::new(
        egui::RichText::new(text)
            .size(15.0)
            .strong()
            .color(text_color),
    )
    .fill(fill)
    .stroke(stroke)
    .corner_radius(egui::CornerRadius::same(10))
    .min_size(egui::vec2(min_width, 42.0));

    ui.add_enabled(enabled, button)
}

/// Draws a tinted message banner.
pub fn message_banner(ui: &mut egui::Ui, palette: &Palette, text: &str, tone: MessageTone) {
    let color = tone.color(palette);
    let fill = match tone {
        MessageTone::Neutral => palette.panel,
        _ => Palette::tint(color, 24),
    };
    let stroke = match tone {
        MessageTone::Neutral => egui::Stroke::new(1.0, palette.border),
        _ => egui::Stroke::new(1.0, Palette::tint(color, 110)),
    };

    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(text).size(14.0).color(color));
        });
}
