//! Reusable styled widgets for the main screen.

use eframe::egui;

use super::style::Palette;

/// Height of the styled text inputs.
pub const FIELD_HEIGHT: f32 = 36.0;

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
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
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
            .size(16.5)
            .strong()
            .color(palette.text),
    );
    ui.add_space(4.0);
}

/// Draws a small weak-colored hint.
pub fn hint_text(ui: &mut egui::Ui, palette: &Palette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(11.5)
            .color(palette.weak_text),
    );
}

/// Draws a small validation error.
pub fn error_text(ui: &mut egui::Ui, palette: &Palette, text: &str) {
    ui.label(egui::RichText::new(text).size(11.5).color(palette.error));
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
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.set_width(self.width);
            ui.label(
                egui::RichText::new(self.label)
                    .size(11.5)
                    .strong()
                    .color(palette.weak_text),
            );
            ui.add_space(1.0);

            let edit = egui::TextEdit::singleline(self.value)
                .hint_text(self.hint.to_string())
                .margin(egui::Margin::symmetric(11, 7))
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
        .corner_radius(egui::CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(11, 8))
        .show(ui, |ui| {
            ui.set_width((width - 22.0).max(0.0));
            ui.set_height(FIELD_HEIGHT - 16.0);
            let color = if has_file {
                palette.text
            } else {
                palette.weak_text
            };
            ui.add(egui::Label::new(egui::RichText::new(text).size(13.5).color(color)).truncate());
        });
}

/// Button variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
}

/// Draws a styled button with hover and pressed feedback.
pub fn button(
    ui: &mut egui::Ui,
    palette: &Palette,
    text: &str,
    kind: ButtonKind,
    enabled: bool,
    min_width: f32,
) -> egui::Response {
    let primary_hover = palette.primary.lerp_to_gamma(egui::Color32::WHITE, 0.14);
    let primary_pressed = palette.primary.lerp_to_gamma(egui::Color32::BLACK, 0.12);
    let error_hover = palette.error.lerp_to_gamma(egui::Color32::WHITE, 0.14);
    let error_pressed = palette.error.lerp_to_gamma(egui::Color32::BLACK, 0.12);
    let secondary_hover = palette.primary.lerp_to_gamma(palette.card, 0.88);
    let secondary_pressed = palette.primary.lerp_to_gamma(palette.card, 0.80);

    let (fill, hover_fill, pressed_fill, border, border_hover, text_color) = if !enabled {
        (
            palette.panel,
            palette.panel,
            palette.panel,
            palette.border,
            palette.border,
            palette.weak_text,
        )
    } else {
        match kind {
            ButtonKind::Primary => (
                palette.primary,
                primary_hover,
                primary_pressed,
                palette.primary,
                primary_hover,
                egui::Color32::WHITE,
            ),
            ButtonKind::Secondary => (
                palette.card,
                secondary_hover,
                secondary_pressed,
                palette.border,
                palette.primary,
                palette.text,
            ),
            ButtonKind::Danger => (
                palette.error,
                error_hover,
                error_pressed,
                palette.error,
                error_hover,
                egui::Color32::WHITE,
            ),
        }
    };

    ui.scope(|ui| {
        let widgets = &mut ui.style_mut().visuals.widgets;

        widgets.inactive.weak_bg_fill = fill;
        widgets.inactive.bg_fill = fill;
        widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border);
        widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_color);
        widgets.inactive.expansion = 0.0;

        widgets.hovered.weak_bg_fill = hover_fill;
        widgets.hovered.bg_fill = hover_fill;
        widgets.hovered.bg_stroke = egui::Stroke::new(1.0, border_hover);
        widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text_color);
        widgets.hovered.expansion = 0.0;

        widgets.active.weak_bg_fill = pressed_fill;
        widgets.active.bg_fill = pressed_fill;
        widgets.active.bg_stroke = egui::Stroke::new(1.0, border_hover);
        widgets.active.fg_stroke = egui::Stroke::new(1.0, text_color);
        widgets.active.expansion = 0.0;

        widgets.noninteractive.weak_bg_fill = palette.panel;
        widgets.noninteractive.bg_fill = palette.panel;
        widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.border);
        widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.weak_text);

        let button = egui::Button::new(
            egui::RichText::new(text)
                .size(14.5)
                .strong()
                .color(text_color),
        )
        .corner_radius(egui::CornerRadius::same(9))
        .min_size(egui::vec2(min_width, 38.0));

        ui.add_enabled(enabled, button)
    })
    .inner
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
        .corner_radius(egui::CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(text).size(13.5).color(color));
        });
}
