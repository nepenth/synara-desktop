#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = app_lib::run_cli(&args) {
        std::process::exit(code);
    }
    app_lib::run();
}
