//! The Ilvuno window. A plain page in `ui/` that reads the local API on 127.0.0.1:4737 (docs/api.md).
//! It holds no engine code. A packaged window starts its bundled runner when the local API is not up.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use tauri::Manager;

const RUNNER_PORT: u16 = 4737;

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

#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![read_goal, app_version])
        .setup(|app| {
            start_bundled_runner_if_needed(app.handle());

            // ILVUNO_WINDOW_SIZE=1280x900 opens the window at another size. tools/shot.sh uses it to
            // check the layout at a width other than the 1440x900 in tauri.conf.json: without a window
            // manager, resizing the X window does not resize the page inside it.
            if let Some((w, h)) = size_from_env() {
                if let Some(win) = app.get_webview_window("main") {
                    win.set_size(tauri::PhysicalSize::new(w, h))?;
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("the Ilvuno window failed to start");
}

fn start_bundled_runner_if_needed(app: &tauri::AppHandle) {
    if api_is_healthy(RUNNER_PORT) {
        return;
    }

    let Ok(resources) = app.path().resource_dir() else {
        return;
    };
    let runner = resources.join("ilvunod");
    if !runner.is_file() {
        return;
    }

    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };

    let mut command = Command::new(runner);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env_clear()
        .env("HOME", &home)
        .env("PATH", runner_path(&home, std::env::var_os("PATH").as_deref()));

    for key in ["USER", "LANG", "LC_ALL", "TERM", "TMPDIR", "ILVUNO_HOME"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }

    // The API outlives the window so closing the window cannot kill a run that is in progress.
    let _ = command.spawn();
}

fn runner_path(home: &Path, inherited: Option<&std::ffi::OsStr>) -> std::ffi::OsString {
    let mut paths = Vec::new();
    let mut candidates = inherited
        .into_iter()
        .flat_map(std::env::split_paths)
        .chain([
            home.join(".local/bin"),
            home.join(".cargo/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ]);

    for path in candidates.by_ref() {
        if path.is_absolute() && !paths.contains(&path) {
            paths.push(path);
        }
    }

    std::env::join_paths(paths).unwrap_or_default()
}

fn api_is_healthy(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(250)) else {
        return false;
    };
    let timeout = Some(Duration::from_millis(500));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);
    if stream
        .write_all(b"GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }

    let mut response = Vec::new();
    if stream.take(4096).read_to_end(&mut response).is_err() {
        return false;
    }
    let Ok(response) = std::str::from_utf8(&response) else {
        return false;
    };
    let Some((headers, body)) = response.split_once("\r\n\r\n") else {
        return false;
    };

    (headers.starts_with("HTTP/1.0 200 ") || headers.starts_with("HTTP/1.1 200 "))
        && body.contains("\"ok\":true")
}

fn size_from_env() -> Option<(u32, u32)> {
    let raw = std::env::var("ILVUNO_WINDOW_SIZE").ok()?;
    let (w, h) = raw.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn app_version_comes_from_the_desktop_cargo_package() {
        assert_eq!(app_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn runner_path_keeps_absolute_entries_and_adds_common_cli_locations() {
        let home = Path::new("/home/example");
        let inherited = std::ffi::OsStr::new("/custom/bin:relative:/usr/bin");
        let paths = std::env::split_paths(&runner_path(home, Some(inherited))).collect::<Vec<_>>();

        assert_eq!(
            paths,
            vec![
                PathBuf::from("/custom/bin"),
                PathBuf::from("/usr/bin"),
                PathBuf::from("/home/example/.local/bin"),
                PathBuf::from("/home/example/.cargo/bin"),
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/bin"),
            ]
        );
    }

    #[test]
    fn health_check_only_accepts_the_runner_health_response() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind local test listener");
        let port = listener.local_addr().expect("listener address").port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept health request");
            let mut request = [0; 256];
            let _ = stream.read(&mut request);
            stream
                .write_all(
                    b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\n\r\n{\"ok\":true}",
                )
                .expect("write health response");
        });

        assert!(api_is_healthy(port));
        server.join().expect("health server thread");
    }
}
