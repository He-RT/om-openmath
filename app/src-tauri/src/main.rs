//! OpenMath desktop application entry point.
#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    om_desktop::run();
}
