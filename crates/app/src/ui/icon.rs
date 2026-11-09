//! Application icon embedded in the binary.

use eframe::egui;

const ICON_PNG: &[u8] = include_bytes!("../../../../assets/icon.png");

/// Loads the window icon. Returns `None` when the embedded image is invalid.
pub fn window_icon() -> Option<egui::IconData> {
    eframe::icon_data::from_png_bytes(ICON_PNG).ok()
}
