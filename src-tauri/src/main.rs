#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::menu::{Menu, MenuItem};
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

    let width = 250.0;
    let height = 250.0;
    let mut pos_x = 100.0;
    let mut pos_y = 100.0;

    if let Ok(cursor) = app.cursor_position() {
        pos_x = cursor.x - width / 2.0;
        pos_y = cursor.y - height / 2.0;
    }

    let builder = WebviewWindowBuilder::new(&app, id, WebviewUrl::App("note.html".into()))
        .title("nori note")
        .inner_size(width, height)
        .decorations(false)
        .transparent(true)
        .resizable(true)
        .always_on_top(true)
        .position(pos_x, pos_y);

    let result = builder.build();

    match result {
        Ok(window) => {
            if let Ok(Some(monitor)) = window.current_monitor() {
                let monitor_pos = monitor.position();
                let monitor_size = monitor.size();
                let scale = monitor.scale_factor();

                let min_x = monitor_pos.x as f64;
                let min_y = monitor_pos.y as f64;
                let max_x = min_x + (monitor_size.width as f64 / scale) - width;
                let max_y = min_y + (monitor_size.height as f64 / scale) - height;

                let clamped_x = pos_x.max(min_x).min(max_x.max(min_x));
                let clamped_y = pos_y.max(min_y).min(max_y.max(min_y));

                if clamped_x != pos_x || clamped_y != pos_y {
                    let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                        x: clamped_x,
                        y: clamped_y,
                    }));
                }
            }
        }
        Err(e) => eprintln!("note window FAILED: {:?}", e),
    }
}

#[tauri::command]
async fn show_main_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
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

            let show_item = MenuItem::with_id(app, "show", "Show nori", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            open_note_window,
            get_notes,
            save_note,
            delete_note,
            reorder_notes,
            show_main_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}