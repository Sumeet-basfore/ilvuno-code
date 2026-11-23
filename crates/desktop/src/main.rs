//! The Ilvuno window. A plain page in `ui/` that reads the local API on 127.0.0.1:4737 (docs/api.md).
//! It holds no engine code: start the runner first, or use tools/window.sh, which does it for you.

fn main() {
    tauri::Builder::default()
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
