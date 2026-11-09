//! Desktop entry point for SQLRestore.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod i18n;
mod startup;
mod ui;

fn main() -> eframe::Result<()> {
    startup::run()
}
