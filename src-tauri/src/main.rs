#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
struct Note {
    id: String,
    text: String,
    color: String,
    rotation: f64,
}

fn notes_file_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    let dir = app.path().app_data_dir().expect("no app data dir");
    fs::create_dir_all(&dir).ok();
    dir.join("notes.json")
}

fn read_notes(app: &tauri::AppHandle) -> Vec<Note> {
    let path = notes_file_path(app);
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn write_notes(app: &tauri::AppHandle, notes: &Vec<Note>) {
    let path = notes_file_path(app);
    let json = serde_json::to_string_pretty(notes).unwrap();
    fs::write(path, json).ok();
}

#[tauri::command]
async fn get_notes(app: tauri::AppHandle) -> Vec<Note> {
    read_notes(&app)
}

#[tauri::command]
async fn save_note(app: tauri::AppHandle, id: String, text: String, color: String, rotation: f64) {
    let mut notes = read_notes(&app);
    if let Some(existing) = notes.iter_mut().find(|n| n.id == id) {
        existing.text = text;
    } else {
        notes.push(Note { id, text, color, rotation });
    }
    write_notes(&app, &notes);
}

#[tauri::command]
async fn reorder_notes(app: tauri::AppHandle, ordered_ids: Vec<String>) {
    let notes = read_notes(&app);
    let mut reordered: Vec<Note> = Vec::new();
    for id in &ordered_ids {
        if let Some(n) = notes.iter().find(|n| &n.id == id) {
            reordered.push(n.clone());
        }
    }
    write_notes(&app, &reordered);
}

#[tauri::command]
async fn delete_note(app: tauri::AppHandle, id: String) {
    let mut notes = read_notes(&app);
    notes.retain(|n| n.id != id);
    write_notes(&app, &notes);
}

#[tauri::command]
async fn open_note_window(app: tauri::AppHandle, note_id: Option<String>) {
    let id = note_id.unwrap_or_else(|| format!("note-{}", chrono::Utc::now().timestamp_millis()));

    if let Some(existing) = app.get_webview_window(&id) {
        let _ = existing.set_focus();
        return;
    }

    let result = WebviewWindowBuilder::new(&app, id, WebviewUrl::App("note.html".into()))
        .title("nori note")
        .inner_size(250.0, 250.0)
        .decorations(false)
        .transparent(true)
        .resizable(true)
        .always_on_top(true)
        .build();

    if let Err(e) = result {
        eprintln!("note window FAILED: {:?}", e);
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
                            open_note_window(app, None).await;
                        });
                    }
                })
                .build(),
        )
        .setup(|app| {
            app.global_shortcut().register("Alt+N")?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_note_window,
            get_notes,
            save_note,
            delete_note,
            reorder_notes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}