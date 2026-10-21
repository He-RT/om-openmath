//! Tauri application entry and shared-kernel commands.
#![forbid(unsafe_code)]
#[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
mod host;
#[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
mod menu;
#[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
pub use host::KernelHost;
/// Launch the shared Web workbench in the platform's Tauri window.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());
    #[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
    let builder = builder.setup(|app| {
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
        let host = KernelHost::new(store).map_err(std::io::Error::other)?;
        app.manage(host);
        app.set_menu(menu::build(app.handle(), zh)?)?;
        Ok(())
    });
    #[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
    let builder = builder.on_menu_event(|app, event| {
        use tauri::Emitter;
        if event.id().as_ref().starts_with("om-") {
            let _ = app.emit("openmath:menu", event.id().as_ref());
        }
    });
    #[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        kernel_request,
        kernel_subscribe,
        kernel_interrupt,
        secret_set,
        secret_delete
    ]);
    builder
        .run(tauri::generate_context!())
        .expect("failed to run OpenMath desktop application");
}

#[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
#[tauri::command]
async fn kernel_request(
    host: tauri::State<'_, KernelHost>,
    envelope: String,
) -> Result<String, String> {
    host.request(envelope).await
}
#[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
#[tauri::command]
fn kernel_subscribe(
    host: tauri::State<'_, KernelHost>,
    channel: tauri::ipc::Channel<String>,
) -> Result<(), String> {
    host.subscribe(std::sync::Arc::new(move |event| {
        let _ = channel.send(event);
    }))
}
#[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
#[tauri::command]
fn kernel_interrupt(host: tauri::State<'_, KernelHost>) {
    host.interrupt();
}
#[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
#[tauri::command]
async fn secret_set(
    host: tauri::State<'_, KernelHost>,
    profile: String,
    key: String,
) -> Result<(), String> {
    host.secret(profile, Some(key)).await
}
#[cfg(all(desktop, feature = "native-kernel", not(feature = "web-shell")))]
#[tauri::command]
async fn secret_delete(host: tauri::State<'_, KernelHost>, profile: String) -> Result<(), String> {
    host.secret(profile, None).await
}
