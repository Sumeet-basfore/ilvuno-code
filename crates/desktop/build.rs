fn main() {
    // tauri::generate_context! embeds ui/ at compile time. Without this line cargo does not watch the
    // folder, so an edit to a .css or .js file rebuilds nothing and the window shows the previous markup.
    println!("cargo:rerun-if-changed=ui");
    tauri_build::build()
}
