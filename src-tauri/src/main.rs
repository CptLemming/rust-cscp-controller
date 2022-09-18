#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use common::Fader;
use cscp::commands::AsyncProcInputTx;
use tauri::{async_runtime::Mutex, Manager, Window};
use tokio::sync::mpsc;

use crate::cscp::{commands::{setFaderLevel, setFaderCut, setFaderPfl, getDatabase}, client::CSCPClient};

mod cscp;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    let (async_proc_input_tx, async_proc_input_rx) = mpsc::channel(1);
    let (fader_event_tx, mut fader_event_rx) = mpsc::channel(1);

    tauri::Builder::default()
        .manage(AsyncProcInputTx {
            inner: Mutex::new(async_proc_input_tx),
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            setFaderLevel,
            setFaderCut,
            setFaderPfl,
            getDatabase,
        ])
        .setup(|app| {
            tauri::async_runtime::spawn(async move {
                let _client = CSCPClient::connect(
                    "172.16.255.5:49556",
                    async_proc_input_rx,
                    fader_event_tx,
                ).await.unwrap();
                println!("Client disconnected");
            });

            // let app_handle = app.handle();
            let main_window = app.get_window("main").unwrap();
            tauri::async_runtime::spawn(async move {
                loop {
                    if let Some(fader) = fader_event_rx.recv().await {
                        publish_fader(fader, &main_window);
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn publish_fader(fader: Fader, manager: &Window) {
    println!("fader::changed {:?}", fader);
    manager
        .emit("fader::changed", fader)
        .unwrap();
}
