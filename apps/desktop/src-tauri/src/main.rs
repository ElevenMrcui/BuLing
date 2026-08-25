// Prevent Windows console window from popping up in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    opc_desktop_lib::run();
}
