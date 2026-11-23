//! The Ilvuno window. A plain page in `ui/` that reads the local API on 127.0.0.1:4737 (docs/api.md).
//! It holds no engine code: start the runner first, or use tools/window.sh, which does it for you.

/// Read the goal file the person picked. The picker only returns a path, and the window has no file
/// access of its own. The size cap keeps one huge file from freezing the window.
#[tauri::command]
fn read_goal(path: String) -> Result<String, String> {
    const MAX: u64 = 1024 * 1024;
    let meta = std::fs::metadata(&path).map_err(|e| format!("{path}: {e}"))?;
    if meta.len() > MAX {
        return Err(format!("{path} is {} bytes; a goal file should be under 1 MiB", meta.len()));
    }
    std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![read_goal])
        .setup(|app| {
            // ILVUNO_WINDOW_SIZE=1280x900 opens the window at another size. tools/shot.sh uses it to
            // check the layout at a width other than the 1440x900 in tauri.conf.json: without a window
            // manager, resizing the X window does not resize the page inside it.
            if let Some((w, h)) = size_from_env() {
                use tauri::Manager;
                if let Some(win) = app.get_webview_window("main") {
                    win.set_size(tauri::PhysicalSize::new(w, h))?;
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("the Ilvuno window failed to start");
}

fn size_from_env() -> Option<(u32, u32)> {
    let raw = std::env::var("ILVUNO_WINDOW_SIZE").ok()?;
    let (w, h) = raw.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}
