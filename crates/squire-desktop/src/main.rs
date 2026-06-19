//! Squire native desktop app (SQUIRE-T-0092 / ADR SQUIRE-A-0012): instead of opening the Keep in a
//! browser, this hosts it in a native OS window. The home server ([`squire_home::run_home_server`]) is
//! embedded and runs on a background tokio runtime; the Tauri/webview event loop owns the main thread
//! and shows a window pointing at the loopback Keep.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::TcpStream;
use std::time::{Duration, Instant};

fn main() {
    let keep_port: u16 = std::env::var("KEEP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4920);

    // Run the embedded home server on its own tokio runtime in a background thread; its serve loop
    // blocks that thread for the life of the app.
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build tokio runtime");
        if let Err(e) = rt.block_on(squire_home::run_home_server()) {
            eprintln!("squire-desktop: home server exited: {e}");
        }
    });

    tauri::Builder::default()
        .setup(move |app| {
            // Wait for the embedded Keep to be listening, then open the window pointing at it.
            wait_for_port(keep_port, Duration::from_secs(20));
            let url = format!("http://127.0.0.1:{keep_port}");
            tauri::WebviewWindowBuilder::new(
                app,
                "keep",
                tauri::WebviewUrl::External(url.parse().expect("valid loopback url")),
            )
            .title("Squire")
            .inner_size(1100.0, 1500.0)
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the Squire desktop app");
}

/// Block until `127.0.0.1:port` accepts a connection (the server is up) or `timeout` elapses.
fn wait_for_port(port: u16, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
