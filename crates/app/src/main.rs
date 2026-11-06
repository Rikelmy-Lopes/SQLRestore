//! Desktop entry point for SQLRestore.

mod commands;
mod i18n;
mod startup;
mod ui;

fn main() -> eframe::Result<()> {
    startup::run()
}
