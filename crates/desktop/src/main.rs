//! The Ilvuno window. A plain page in `ui/` that reads the local API on 127.0.0.1:4737 (docs/api.md).
//! It holds no engine code: start the runner first, or use tools/window.sh, which does it for you.

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("the Ilvuno window failed to start");
}
