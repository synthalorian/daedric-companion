// Prevents an extra console window on Windows in release builds.
// A console-subsystem exe dies when that window is closed.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    daedric_companion_lib::run();
}
