//! Visual theme: colors, spacing and widget style.

use eframe::egui;
use sqlrestore_core::domain::config::Theme;

/// Colors used by the interface for one theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub background: egui::Color32,
    pub panel: egui::Color32,
    pub card: egui::Color32,
    pub input: egui::Color32,
    pub border: egui::Color32,
    pub text: egui::Color32,
    pub weak_text: egui::Color32,
    pub primary: egui::Color32,
    pub success: egui::Color32,
    pub error: egui::Color32,
}

impl Palette {
    /// Returns the palette for the given theme.
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self {
                background: egui::Color32::from_rgb(0xF4, 0xF6, 0xFA),
                panel: egui::Color32::from_rgb(0xFF, 0xFF, 0xFF),
                card: egui::Color32::from_rgb(0xFF, 0xFF, 0xFF),
                input: egui::Color32::from_rgb(0xFA, 0xFB, 0xFD),
                border: egui::Color32::from_rgb(0xD6, 0xDC, 0xE7),
                text: egui::Color32::from_rgb(0x1A, 0x1A, 0x1A),
                weak_text: egui::Color32::from_rgb(0x6B, 0x72, 0x80),
                primary: egui::Color32::from_rgb(0x25, 0x63, 0xEB),
                success: egui::Color32::from_rgb(0x16, 0xA3, 0x4A),
                error: egui::Color32::from_rgb(0xDC, 0x26, 0x26),
            },
            Theme::Dark => Self {
                background: egui::Color32::from_rgb(0x12, 0x12, 0x12),
                panel: egui::Color32::from_rgb(0x1E, 0x1E, 0x1E),
                card: egui::Color32::from_rgb(0x1E, 0x1E, 0x1E),
                input: egui::Color32::from_rgb(0x26, 0x26, 0x26),
                border: egui::Color32::from_rgb(0x33, 0x3A, 0x45),
                text: egui::Color32::from_rgb(0xED, 0xED, 0xED),
                weak_text: egui::Color32::from_rgb(0x9C, 0xA3, 0xAF),
                primary: egui::Color32::from_rgb(0x3B, 0x82, 0xF6),
                success: egui::Color32::from_rgb(0x22, 0xC5, 0x5E),
                error: egui::Color32::from_rgb(0xEF, 0x44, 0x44),
            },
        }
    }

    /// A translucent version of a color, used for message backgrounds.
    pub fn tint(color: egui::Color32, alpha: u8) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
    }
}

/// Applies the application style to the given context.
pub fn apply_style(context: &egui::Context, theme: Theme) {
    let palette = Palette::for_theme(theme);
    let egui_theme = match theme {
        Theme::Light => egui::Theme::Light,
        Theme::Dark => egui::Theme::Dark,
    };
    context.set_theme(egui_theme);

    let mut style = (*context.style_of(egui_theme)).clone();

    style.visuals = visuals_for(theme, &palette);
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(18.0, 9.0);
    style.spacing.interact_size.y = 38.0;
    style.spacing.text_edit_width = 260.0;
    style.spacing.icon_width = 18.0;
    style.spacing.icon_width_inner = 10.0;
    style.spacing.icon_spacing = 8.0;
    style.spacing.menu_margin = egui::Margin::same(8);

    style.text_styles = [
        (egui::TextStyle::Heading, egui::FontId::proportional(22.0)),
        (egui::TextStyle::Body, egui::FontId::proportional(15.0)),
        (egui::TextStyle::Button, egui::FontId::proportional(15.0)),
        (egui::TextStyle::Small, egui::FontId::proportional(12.0)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(13.0)),
    ]
    .into();

    context.set_style_of(egui_theme, style);
}

fn visuals_for(theme: Theme, palette: &Palette) -> egui::Visuals {
    let mut visuals = match theme {
        Theme::Light => egui::Visuals::light(),
        Theme::Dark => egui::Visuals::dark(),
    };

    visuals.panel_fill = palette.background;
    visuals.window_fill = palette.card;
    visuals.extreme_bg_color = palette.input;
    visuals.faint_bg_color = palette.panel;
    visuals.window_stroke = egui::Stroke::new(1.0, palette.border);
    visuals.selection.bg_fill = palette.primary;
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.hyperlink_color = palette.primary;
    visuals.window_corner_radius = egui::CornerRadius::same(14);
    visuals.menu_corner_radius = egui::CornerRadius::same(10);
    visuals.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: egui::Color32::from_black_alpha(40),
    };
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: egui::Color32::from_black_alpha(32),
    };

    visuals.widgets.noninteractive.bg_fill = palette.card;
    visuals.widgets.noninteractive.weak_bg_fill = palette.card;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.text);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(10);

    visuals.widgets.inactive.bg_fill = palette.input;
    visuals.widgets.inactive.weak_bg_fill = palette.input;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.text);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(10);
    visuals.widgets.inactive.expansion = 0.0;

    visuals.widgets.hovered.bg_fill = palette.input;
    visuals.widgets.hovered.weak_bg_fill = palette.input;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.primary);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, palette.text);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(10);
    visuals.widgets.hovered.expansion = 0.0;

    visuals.widgets.active.bg_fill = palette.input;
    visuals.widgets.active.weak_bg_fill = palette.input;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, palette.primary);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, palette.text);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(10);
    visuals.widgets.active.expansion = 0.0;

    visuals.widgets.open.bg_fill = palette.input;
    visuals.widgets.open.weak_bg_fill = palette.input;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, palette.primary);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, palette.text);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(10);

    visuals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_have_distinct_palettes() {
        let light = Palette::for_theme(Theme::Light);
        let dark = Palette::for_theme(Theme::Dark);
        assert_ne!(light.background, dark.background);
        assert_ne!(light.text, dark.text);
        assert_eq!(light.primary, egui::Color32::from_rgb(0x25, 0x63, 0xEB));
        assert_eq!(dark.primary, egui::Color32::from_rgb(0x3B, 0x82, 0xF6));
    }

    #[test]
    fn tint_sets_alpha_and_keeps_color_family() {
        let color = egui::Color32::from_rgb(0x25, 0x63, 0xEB);
        let tinted = Palette::tint(color, 24);
        assert_eq!(tinted.a(), 24);
        assert_ne!(tinted, color);
    }
}
