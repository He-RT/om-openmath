//! Native menus delegate notebook operations to the existing frontend controller.
use tauri::{
    AppHandle,
    menu::{AboutMetadata, Menu, MenuBuilder, SubmenuBuilder},
};
pub fn build(app: &AppHandle, zh: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let text = |en, chinese| if zh { chinese } else { en };
    let application = SubmenuBuilder::new(app, "OpenMath")
        .about(Some(AboutMetadata {
            name: Some("OpenMath".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            comments: Some("Pre-alpha · MIT OR Apache-2.0".into()),
            ..Default::default()
        }))
        .separator()
        .text("om-settings", text("Settings…", "设置…"))
        .separator();
    // These application actions are macOS-specific; Show All fails on Windows.
    #[cfg(target_os = "macos")]
    let application = application.hide().hide_others().show_all().separator();
    let application = application.quit().build()?;
    let file = SubmenuBuilder::new(app, text("File", "文件"))
        .text("om-new", text("New Notebook", "新建笔记本"))
        .text("om-open", text("Open…", "打开…"))
        .text("om-save", text("Save…", "保存…"))
        .text("om-save-as", text("Save As…", "另存为…"))
        .separator()
        .text("om-export-md", text("Export Markdown…", "导出 Markdown…"))
        .text("om-export-tex", text("Export LaTeX…", "导出 LaTeX…"))
        .separator()
        .close_window()
        .build()?;
    let edit = SubmenuBuilder::new(app, text("Edit", "编辑"))
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let view = SubmenuBuilder::new(app, text("View", "视图"))
        .text("om-theme-light", text("Light Theme", "浅色主题"))
        .text("om-theme-dark", text("Dark Theme", "深色主题"))
        .text("om-theme-system", text("System Theme", "跟随系统主题"))
        .separator()
        .text("om-panel", text("Toggle Inspector", "切换上下文面板"))
        .text("om-assistant", text("AI Assistant", "AI 助手"))
        .text("om-commands", text("Commands…", "命令…"))
        .build()?;
    let help = SubmenuBuilder::new(app, text("Help", "帮助"))
        .text("om-docs", text("Documentation", "文档"))
        .about(None)
        .build()?;
    MenuBuilder::new(app)
        .items(&[&application, &file, &edit, &view, &help])
        .build()
}
