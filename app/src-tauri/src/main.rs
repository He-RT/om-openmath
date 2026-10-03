//! OpenMath desktop application entry point.
#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod menu;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            use tauri::Manager;
            let store = if let Some(path) = std::env::var_os("OPENMATH_CONFIG_PATH") {
                om_kernel::native::ConfigStore::new(
                    path,
                    std::sync::Arc::new(om_kernel::native::NativeCredentials),
                    om_kernel::native::KeyStorage::Vault,
                )
            } else {
                om_kernel::native::ConfigStore::system_default()?
            };
            let zh = store.load()?.general.language == om_kernel::config::Language::ZhCn;
            let host = om_desktop::KernelHost::new(store).map_err(std::io::Error::other)?;
            app.manage(host);
            app.set_menu(menu::build(app.handle(), zh)?)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            use tauri::Emitter;
            if event.id().as_ref().starts_with("om-") {
                let _ = app.emit("openmath:menu", event.id().as_ref());
            }
        })
        .invoke_handler(tauri::generate_handler![
            kernel_request,
            kernel_subscribe,
            kernel_interrupt,
            secret_set,
            secret_delete
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenMath desktop application");
}

#[tauri::command]
async fn kernel_request(
    host: tauri::State<'_, om_desktop::KernelHost>,
    envelope: String,
) -> Result<String, String> {
    host.request(envelope).await
}
#[tauri::command]
fn kernel_subscribe(
    host: tauri::State<'_, om_desktop::KernelHost>,
    channel: tauri::ipc::Channel<String>,
) -> Result<(), String> {
    host.subscribe(std::sync::Arc::new(move |event| {
        let _ = channel.send(event);
    }))
}
#[tauri::command]
fn kernel_interrupt(host: tauri::State<'_, om_desktop::KernelHost>) {
    host.interrupt();
}
#[tauri::command]
async fn secret_set(
    host: tauri::State<'_, om_desktop::KernelHost>,
    profile: String,
    key: String,
) -> Result<(), String> {
    host.secret(profile, Some(key)).await
}
#[tauri::command]
async fn secret_delete(
    host: tauri::State<'_, om_desktop::KernelHost>,
    profile: String,
) -> Result<(), String> {
    host.secret(profile, None).await
}
