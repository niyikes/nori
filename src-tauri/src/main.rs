#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[tauri::command]
async fn open_note_window(app: tauri::AppHandle) {
    println!("OPEN NOTE ENTERED");

    let note_id = format!("note-{}", chrono::Utc::now().timestamp_millis());

    let result = WebviewWindowBuilder::new(
        &app,
        note_id,
        WebviewUrl::App("note.html".into()),

    )
    .title("nori note")
    .inner_size(250.0, 250.0)
    .decorations(true)
    .resizable(true)
    .always_on_top(true)
    .build();

    match result {
        Ok(window) => {
            println!("NOTE WINDOW CREATED: {}", window.label());
        }
        Err(e) => {
            eprintln!("NOTE WINDOW FAILED: {:?}", e);
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let app = app.clone();

                        tauri::async_runtime::spawn(async move {
                            open_note_window(app).await;
                        });
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