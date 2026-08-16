#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[tauri::command]
fn open_note_window(app: tauri::AppHandle) {
    let note_id = format!("note-{}", chrono::Utc::now().timestamp_millis());

    let result = WebviewWindowBuilder::new(&app, note_id, WebviewUrl::App("index.html".into()))
        .title("nori note")
        .inner_size(250.0, 250.0)
        .decorations(true)
        .resizable(true)
        .always_on_top(true)
        .build();

    match result {
        Ok(_) => println!("note window created OK"),
        Err(e) => eprintln!("note window FAILED: {:?}", e),
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        open_note_window(app.clone());
                    }
                })
                .build(),
        )
        .setup(|app| {
            app.global_shortcut().register("Alt+N")?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![open_note_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}