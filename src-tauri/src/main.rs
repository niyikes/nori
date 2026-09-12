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
        if let Some(monitor) = app.primary_monitor().ok().flatten() {
            let scale = monitor.scale_factor();
            let logical_x = cursor.x / scale;
            let logical_y = cursor.y / scale;
            pos_x = logical_x - width / 2.0;
            pos_y = logical_y - height / 2.0;
        } else {
            pos_x = cursor.x - width / 2.0;
            pos_y = cursor.y - height / 2.0;
        }
    }

    let builder = WebviewWindowBuilder::new(&app, id, WebviewUrl::App("note.html".into()))
        .title("nori note")
        .inner_size(width, height)
        .decorations(false)
        .transparent(true)
        .resizable(true)
        .always_on_top(true)
        .visible(false)
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

            let focus_window = window.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let _ = focus_window.set_focus();
            });

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

#[tauri::command]
async fn show_note_window(app: tauri::AppHandle, id: String) {
    if let Some(window) = app.get_webview_window(&id) {
        let _ = window.show();
    }
}

//pin and unpin da windows
/*
#[tauri::command]
async fn toggle_pin(app: tauri::AppHandle, id: String) -> bool {
    if let Some(window) = app.get_webview_window(&id) {
        let is_pinned = window.is_always_on_top().unwrap_or(false);
        let _ = window.set_always_on_top(!is_pinned);
        return !is_pinned;
    }
    false
}
*/

fn seed_intro_note_if_needed(app: &tauri::AppHandle) {
    let notes = read_notes(app);
    if notes.is_empty() {
        let intro = Note {
            id: format!("note-{}", chrono::Utc::now().timestamp_millis()),
            text: r#"<div>welcome to nori!! here's how a few things work:</div>
<div><br></div>
<div class="bullet-item"><span class="bullet-dot"></span><span class="todo-text">type <b>--</b> at the start of a line to make a to-do checklist</span></div>
<div class="bullet-item"><span class="bullet-dot"></span><span class="todo-text">type <b>-</b> at the start of a line to make a bullet point</span></div>
<div class="bullet-item"><span class="bullet-dot"></span><span class="todo-text"><b>ctrl+b</b> for bold, <u>ctrl+u</u> to underline, italic with ctrl+i</span></div>
<div class="bullet-item"><span class="bullet-dot"></span><span class="todo-text"><b>alt+n</b> anywhere makes a new note</span></div>
<div class="bullet-item"><span class="bullet-dot"></span><span class="todo-text">paste a picture in and drag the little dot in the corner to resize it</span></div>
<div class="img-wrap" style="width: 180px;"><img class="pasted-img" src="data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjkxIiBoZWlnaHQ9IjI3OCIgdmlld0JveD0iMCAwIDI5MSAyNzgiIGZpbGw9Im5vbmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+CjxwYXRoIGQ9Ik05My41NzY5IDQ1LjgzMTFDNjcuNTgzNSA2My45ODkxIDYzLjc2NCA5MC44MjM2IDU1LjM2MTUgMTI3LjY0N0MyNS40NjEyIDE2MS44NzggMTcuODc0NyAxODAuNzM4IDM1LjMzOTYgMjEzLjIzN0M4MC4xMTggMjc3LjA0MSAyMzkuMTUyIDI1Ni44NTUgMjY2LjU5MiAxOTkuNTFDMjczLjMyIDE3NC42MTkgMjY4LjgzMSAxNTkuNjk3IDI1Mi4xNDQgMTMyLjAzOUMyMzAuMDEzIDU4LjM0NzMgMjE5LjkyMiAzNy4xMzA1IDIwNy41ODMgNDUuODMxMUMxOTcuNTgzIDUyLjg4MjcgMTg1LjAzIDY1LjQxMzkgMTY4Ljc1NiA5My42NDlMMTQxLjUxNyA5NS4zOTEzQzEyMS45MDQgNjMuMzM3OCAxMDIuMDgzIDM5Ljg4ODcgOTMuNTc2OSA0NS44MzExWiIgZmlsbD0iIzE0MTQxNCIvPgo8cGF0aCBkPSJNNzggMTQ2LjAwMUM5MS42NCAxMzIuNTQzIDExNy42OCAxMjIuNzU3IDE0MCAxNDYuMDAxIiBzdHJva2U9IndoaXRlIiBzdHJva2Utd2lkdGg9IjciIHN0cm9rZS1saW5lY2FwPSJyb3VuZCIvPgo8cGF0aCBkPSJNMTY3IDE0Ni4wMDFDMTgwLjY0IDEzMi41NDMgMjA2LjY4IDEyMi43NTcgMjI5IDE0Ni4wMDEiIHN0cm9rZT0id2hpdGUiIHN0cm9rZS13aWR0aD0iNyIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIi8+CjxtYXNrIGlkPSJwYXRoLTQtb3V0c2lkZS0xXzQ0XzE0NCIgbWFza1VuaXRzPSJ1c2VyU3BhY2VPblVzZSIgeD0iMjEyLjMxMSIgeT0iNDAuOTk1MSIgd2lkdGg9IjQ5LjM2NjciIGhlaWdodD0iNzkuMjIzNyIgZmlsbD0iYmxhY2siPgo8cmVjdCBmaWxsPSJ3aGl0ZSIgeD0iMjEyLjMxMSIgeT0iNDAuOTk1MSIgd2lkdGg9IjQ5LjM2NjciIGhlaWdodD0iNzkuMjIzNyIvPgo8cGF0aCBkPSJNMjI2LjY0NSA5Ni40MDQzQzIyNS4wNTYgOTYuNTg4NiAyMjMuNjM3IDk2LjIxNjMgMjIyLjM4OCA5NS4yODczQzIyMS4xMzIgOTQuMjkyIDIyMC4zNzggOTMuMDAzNiAyMjAuMTI4IDkxLjQyMTlMMjE0LjA3NiA1Mi4yNTgzQzIxMy44MzMgNTAuNzQyOSAyMTQuMTc2IDQ5LjM2MDggMjE1LjEwNSA0OC4xMTIxQzIxNi4wMzQgNDYuODYzNSAyMTcuMjYgNDYuMTUwOCAyMTguNzgzIDQ1Ljk3NDJMMjI0LjE0OCA0NS4zNTIxQzIyNS42NzEgNDUuMTc1NSAyMjcuMDI3IDQ1LjU4ODcgMjI4LjIxNyA0Ni41OTE2QzIyOS40MDcgNDcuNTk0NSAyMzAuMDU4IDQ4Ljg2MTMgMjMwLjE2OCA1MC4zOTIxTDIzMy4yNCA4OS45MDEzQzIzMy4zNTggOTEuNDk4MyAyMzIuOTE5IDkyLjkyNSAyMzEuOTI0IDk0LjE4MTRDMjMwLjkyMSA5NS4zNzE1IDIyOS42MjUgOTYuMDU4NyAyMjguMDM2IDk2LjI0M0wyMjYuNjQ1IDk2LjQwNDNaTTIyOS45NTEgMTA0LjA3NUMyMzEuNDc0IDEwMy44OTggMjMyLjgzIDEwNC4zMTEgMjM0LjAyMSAxMDUuMzE0QzIzNS4yNzcgMTA2LjMwOSAyMzUuOTkzIDEwNy41NjggMjM2LjE3IDEwOS4wOTJMMjM2LjQ3IDExMS42NzRDMjM2LjY0NiAxMTMuMTk3IDIzNi4yMzcgMTE0LjU4NyAyMzUuMjQyIDExNS44NDNDMjM0LjMxMyAxMTcuMDkyIDIzMy4wODcgMTE3LjgwNSAyMzEuNTYzIDExNy45ODFMMjI4LjE4NiAxMTguMzczQzIyNi42NjMgMTE4LjU1IDIyNS4yNzMgMTE4LjE0IDIyNC4wMTcgMTE3LjE0NUMyMjIuODI3IDExNi4xNDIgMjIyLjE0MyAxMTQuODc5IDIyMS45NjcgMTEzLjM1NkwyMjEuNjY3IDExMC43NzNDMjIxLjQ5MSAxMDkuMjUgMjIxLjg2NyAxMDcuODY0IDIyMi43OTYgMTA2LjYxNkMyMjMuNzkxIDEwNS4zNTkgMjI1LjA1IDEwNC42NDMgMjI2LjU3MyAxMDQuNDY2TDIyOS45NTEgMTA0LjA3NVpNMjQ5LjUxOSA5My43NTE2QzI0Ny45MyA5My45MzYgMjQ2LjUxMSA5My41NjM2IDI0NS4yNjIgOTIuNjM0NkMyNDQuMDA2IDkxLjYzOTQgMjQzLjI1MiA5MC4zNTA5IDI0My4wMDIgODguNzY5MkwyMzYuOTUgNDkuNjA1N0MyMzYuNzA3IDQ4LjA5MDIgMjM3LjA1IDQ2LjcwODIgMjM3Ljk3OSA0NS40NTk1QzIzOC45MDggNDQuMjEwOCAyNDAuMTM0IDQzLjQ5ODIgMjQxLjY1OCA0My4zMjE2TDI0Ny4wMjIgNDIuNjk5NUMyNDguNTQ1IDQyLjUyMjkgMjQ5LjkwMSA0Mi45MzYgMjUxLjA5MSA0My45Mzg5QzI1Mi4yODIgNDQuOTQxOCAyNTIuOTMyIDQ2LjIwODcgMjUzLjA0MiA0Ny43Mzk1TDI1Ni4xMTQgODcuMjQ4N0MyNTYuMjMyIDg4Ljg0NTcgMjU1Ljc5MyA5MC4yNzI0IDI1NC43OTggOTEuNTI4N0MyNTMuNzk1IDkyLjcxODggMjUyLjQ5OSA5My40MDYxIDI1MC45MSA5My41OTA0TDI0OS41MTkgOTMuNzUxNlpNMjUyLjgyNSAxMDEuNDIyQzI1NC4zNDggMTAxLjI0NSAyNTUuNzA1IDEwMS42NTggMjU2Ljg5NSAxMDIuNjYxQzI1OC4xNTEgMTAzLjY1NyAyNTguODY3IDEwNC45MTYgMjU5LjA0NCAxMDYuNDM5TDI1OS4zNDQgMTA5LjAyMkMyNTkuNTIgMTEwLjU0NSAyNTkuMTExIDExMS45MzQgMjU4LjExNiAxMTMuMTkxQzI1Ny4xODcgMTE0LjQzOSAyNTUuOTYxIDExNS4xNTIgMjU0LjQzNyAxMTUuMzI5TDI1MS4wNiAxMTUuNzJDMjQ5LjUzNyAxMTUuODk3IDI0OC4xNDcgMTE1LjQ4OCAyNDYuODkxIDExNC40OTJDMjQ1LjcwMSAxMTMuNDkgMjQ1LjAxNyAxMTIuMjI3IDI0NC44NDEgMTEwLjcwM0wyNDQuNTQxIDEwOC4xMjFDMjQ0LjM2NSAxMDYuNTk4IDI0NC43NDEgMTA1LjIxMiAyNDUuNjcgMTAzLjk2M0MyNDYuNjY1IDEwMi43MDcgMjQ3LjkyNCAxMDEuOTkgMjQ5LjQ0NyAxMDEuODE0TDI1Mi44MjUgMTAxLjQyMloiLz4KPC9tYXNrPgo8cGF0aCBkPSJNMjI2LjY0NSA5Ni40MDQzQzIyNS4wNTYgOTYuNTg4NiAyMjMuNjM3IDk2LjIxNjMgMjIyLjM4OCA5NS4yODczQzIyMS4xMzIgOTQuMjkyIDIyMC4zNzggOTMuMDAzNiAyMjAuMTI4IDkxLjQyMTlMMjE0LjA3NiA1Mi4yNTgzQzIxMy44MzMgNTAuNzQyOSAyMTQuMTc2IDQ5LjM2MDggMjE1LjEwNSA0OC4xMTIxQzIxNi4wMzQgNDYuODYzNSAyMTcuMjYgNDYuMTUwOCAyMTguNzgzIDQ1Ljk3NDJMMjI0LjE0OCA0NS4zNTIxQzIyNS42NzEgNDUuMTc1NSAyMjcuMDI3IDQ1LjU4ODcgMjI4LjIxNyA0Ni41OTE2QzIyOS40MDcgNDcuNTk0NSAyMzAuMDU4IDQ4Ljg2MTMgMjMwLjE2OCA1MC4zOTIxTDIzMy4yNCA4OS45MDEzQzIzMy4zNTggOTEuNDk4MyAyMzIuOTE5IDkyLjkyNSAyMzEuOTI0IDk0LjE4MTRDMjMwLjkyMSA5NS4zNzE1IDIyOS42MjUgOTYuMDU4NyAyMjguMDM2IDk2LjI0M0wyMjYuNjQ1IDk2LjQwNDNaTTIyOS45NTEgMTA0LjA3NUMyMzEuNDc0IDEwMy44OTggMjMyLjgzIDEwNC4zMTEgMjM0LjAyMSAxMDUuMzE0QzIzNS4yNzcgMTA2LjMwOSAyMzUuOTkzIDEwNy41NjggMjM2LjE3IDEwOS4wOTJMMjM2LjQ3IDExMS42NzRDMjM2LjY0NiAxMTMuMTk3IDIzNi4yMzcgMTE0LjU4NyAyMzUuMjQyIDExNS44NDNDMjM0LjMxMyAxMTcuMDkyIDIzMy4wODcgMTE3LjgwNSAyMzEuNTYzIDExNy45ODFMMjI4LjE4NiAxMTguMzczQzIyNi42NjMgMTE4LjU1IDIyNS4yNzMgMTE4LjE0IDIyNC4wMTcgMTE3LjE0NUMyMjIuODI3IDExNi4xNDIgMjIyLjE0MyAxMTQuODc5IDIyMS45NjcgMTEzLjM1NkwyMjEuNjY3IDExMC43NzNDMjIxLjQ5MSAxMDkuMjUgMjIxLjg2NyAxMDcuODY0IDIyMi43OTYgMTA2LjYxNkMyMjMuNzkxIDEwNS4zNTkgMjI1LjA1IDEwNC42NDMgMjI2LjU3MyAxMDQuNDY2TDIyOS45NTEgMTA0LjA3NVpNMjQ5LjUxOSA5My43NTE2QzI0Ny45MyA5My45MzYgMjQ2LjUxMSA5My41NjM2IDI0NS4yNjIgOTIuNjM0NkMyNDQuMDA2IDkxLjYzOTQgMjQzLjI1MiA5MC4zNTA5IDI0My4wMDIgODguNzY5MkwyMzYuOTUgNDkuNjA1N0MyMzYuNzA3IDQ4LjA5MDIgMjM3LjA1IDQ2LjcwODIgMjM3Ljk3OSA0NS40NTk1QzIzOC45MDggNDQuMjEwOCAyNDAuMTM0IDQzLjQ5ODIgMjQxLjY1OCA0My4zMjE2TDI0Ny4wMjIgNDIuNjk5NUMyNDguNTQ1IDQyLjUyMjkgMjQ5LjkwMSA0Mi45MzYgMjUxLjA5MSA0My45Mzg5QzI1Mi4yODIgNDQuOTQxOCAyNTIuOTMyIDQ2LjIwODcgMjUzLjA0MiA0Ny43Mzk1TDI1Ni4xMTQgODcuMjQ4N0MyNTYuMjMyIDg4Ljg0NTcgMjU1Ljc5MyA5MC4yNzI0IDI1NC43OTggOTEuNTI4N0MyNTMuNzk1IDkyLjcxODggMjUyLjQ5OSA5My40MDYxIDI1MC45MSA5My41OTA0TDI0OS41MTkgOTMuNzUxNlpNMjUyLjgyNSAxMDEuNDIyQzI1NC4zNDggMTAxLjI0NSAyNTUuNzA1IDEwMS42NTggMjU2Ljg5NSAxMDIuNjYxQzI1OC4xNTEgMTAzLjY1NyAyNTguODY3IDEwNC45MTYgMjU5LjA0NCAxMDYuNDM5TDI1OS4zNDQgMTA5LjAyMkMyNTkuNTIgMTEwLjU0NSAyNTkuMTExIDExMS45MzQgMjU4LjExNiAxMTMuMTkxQzI1Ny4xODcgMTE0LjQzOSAyNTUuOTYxIDExNS4xNTIgMjU0LjQzNyAxMTUuMzI5TDI1MS4wNiAxMTUuNzJDMjQ5LjUzNyAxMTUuODk3IDI0OC4xNDcgMTE1LjQ4OCAyNDYuODkxIDExNC40OTJDMjQ1LjcwMSAxMTMuNDkgMjQ1LjAxNyAxMTIuMjI3IDI0NC44NDEgMTEwLjcwM0wyNDQuNTQxIDEwOC4xMjFDMjQ0LjM2NSAxMDYuNTk4IDI0NC43NDEgMTA1LjIxMiAyNDUuNjcgMTAzLjk2M0MyNDYuNjY1IDEwMi43MDcgMjQ3LjkyNCAxMDEuOTkgMjQ5LjQ0NyAxMDEuODE0TDI1Mi44MjUgMTAxLjQyMloiIGZpbGw9IndoaXRlIi8+CjxwYXRoIGQ9Ik0yMjIuMzg4IDk1LjI4NzJMMjIyLjA3OCA5NS42NzkyTDIyMi4wODQgOTUuNjgzOUwyMjIuMDkgOTUuNjg4NEwyMjIuMzg4IDk1LjI4NzJaTTIyMC4xMjggOTEuNDIxOUwyMTkuNjM0IDkxLjQ5ODNMMjE5LjYzNCA5MS41MDAxTDIyMC4xMjggOTEuNDIxOVpNMjE0LjA3NiA1Mi4yNTgzTDIxNC41NyA1Mi4xODJMMjE0LjU3IDUyLjE3OTJMMjE0LjA3NiA1Mi4yNTgzWk0yMTUuMTA1IDQ4LjExMjJMMjE0LjcwNCA0Ny44MTM3TDIxNC43MDQgNDcuODEzN0wyMTUuMTA1IDQ4LjExMjJaTTIyOC4yMTcgNDYuNTkxNkwyMjguNTQgNDYuMjA5MlY0Ni4yMDkyTDIyOC4yMTcgNDYuNTkxNlpNMjMwLjE2OCA1MC4zOTIxTDIyOS42NjkgNTAuNDI4MUwyMjkuNjcgNTAuNDMwOUwyMzAuMTY4IDUwLjM5MjFaTTIzMy4yNCA4OS45MDEzTDIzMy43MzkgODkuODY0NEwyMzMuNzM4IDg5Ljg2MjZMMjMzLjI0IDg5LjkwMTNaTTIzMS45MjQgOTQuMTgxNEwyMzIuMzA3IDk0LjUwMzZMMjMyLjMxMSA5NC40OTc4TDIzMi4zMTYgOTQuNDkxOEwyMzEuOTI0IDk0LjE4MTRaTTIzNC4wMjEgMTA1LjMxNEwyMzMuNjk4IDEwNS42OTZMMjMzLjcwNCAxMDUuNzAxTDIzMy43MSAxMDUuNzA2TDIzNC4wMjEgMTA1LjMxNFpNMjM1LjI0MiAxMTUuODQzTDIzNC44NSAxMTUuNTMzTDIzNC44NDUgMTE1LjUzOUwyMzQuODQgMTE1LjU0NUwyMzUuMjQyIDExNS44NDNaTTIyNC4wMTcgMTE3LjE0NUwyMjMuNjk1IDExNy41MjdMMjIzLjcgMTE3LjUzMkwyMjMuNzA2IDExNy41MzdMMjI0LjAxNyAxMTcuMTQ1Wk0yMjIuNzk2IDEwNi42MTZMMjIyLjQwNCAxMDYuMzA1TDIyMi4zOTkgMTA2LjMxMUwyMjIuMzk1IDEwNi4zMTdMMjIyLjc5NiAxMDYuNjE2Wk0yMjYuNjQ1IDk2LjQwNDNMMjI2LjU4OCA5NS45MDc2QzIyNS4xMjEgOTYuMDc3NyAyMjMuODMgOTUuNzM2OCAyMjIuNjg3IDk0Ljg4NjFMMjIyLjM4OCA5NS4yODcyTDIyMi4wOSA5NS42ODg0QzIyMy40NDQgOTYuNjk1OCAyMjQuOTkxIDk3LjA5OTUgMjI2LjcwMyA5Ni45MDFMMjI2LjY0NSA5Ni40MDQzWk0yMjIuMzg4IDk1LjI4NzJMMjIyLjY5OSA5NC44OTUzQzIyMS41NCA5My45Nzc5IDIyMC44NTMgOTIuODAxNCAyMjAuNjIyIDkxLjM0MzdMMjIwLjEyOCA5MS40MjE5TDIxOS42MzQgOTEuNTAwMUMyMTkuOTA0IDkzLjIwNTggMjIwLjcyMyA5NC42MDYyIDIyMi4wNzggOTUuNjc5MkwyMjIuMzg4IDk1LjI4NzJaTTIyMC4xMjggOTEuNDIxOUwyMjAuNjIyIDkxLjM0NTVMMjE0LjU3IDUyLjE4MkwyMTQuMDc2IDUyLjI1ODNMMjEzLjU4MiA1Mi4zMzQ3TDIxOS42MzQgOTEuNDk4M0wyMjAuMTI4IDkxLjQyMTlaTTIxNC4wNzYgNTIuMjU4M0wyMTQuNTcgNTIuMTc5MkMyMTQuMzQ5IDUwLjc5OSAyMTQuNjU3IDQ5LjU1MjggMjE1LjUwNiA0OC40MTA2TDIxNS4xMDUgNDguMTEyMkwyMTQuNzA0IDQ3LjgxMzdDMjEzLjY5NiA0OS4xNjg5IDIxMy4zMTggNTAuNjg2NyAyMTMuNTgyIDUyLjMzNzRMMjE0LjA3NiA1Mi4yNTgzWk0yMTUuMTA1IDQ4LjExMjJMMjE1LjUwNiA0OC40MTA2QzIxNi4zNTUgNDcuMjcwNyAyMTcuNDU3IDQ2LjYzMTQgMjE4Ljg0MSA0Ni40NzA5TDIxOC43ODMgNDUuOTc0MkwyMTguNzI2IDQ1LjQ3NzVDMjE3LjA2MyA0NS42NzAzIDIxNS43MTQgNDYuNDU2MyAyMTQuNzA0IDQ3LjgxMzdMMjE1LjEwNSA0OC4xMTIyWk0yMTguNzgzIDQ1Ljk3NDJMMjE4Ljg0MSA0Ni40NzA5TDIyNC4yMDUgNDUuODQ4OEwyMjQuMTQ4IDQ1LjM1MjJMMjI0LjA5IDQ0Ljg1NTVMMjE4LjcyNiA0NS40Nzc1TDIxOC43ODMgNDUuOTc0MlpNMjI0LjE0OCA0NS4zNTIyTDIyNC4yMDUgNDUuODQ4OEMyMjUuNTg5IDQ1LjY4ODMgMjI2LjgwOSA0Ni4wNTgzIDIyNy44OTUgNDYuOTczOUwyMjguMjE3IDQ2LjU5MTZMMjI4LjU0IDQ2LjIwOTJDMjI3LjI0NiA0NS4xMTkgMjI1Ljc1MiA0NC42NjI3IDIyNC4wOSA0NC44NTU1TDIyNC4xNDggNDUuMzUyMlpNMjI4LjIxNyA0Ni41OTE2TDIyNy44OTUgNDYuOTczOUMyMjguOTg0IDQ3Ljg5MTMgMjI5LjU2OSA0OS4wMzQgMjI5LjY2OSA1MC40MjgxTDIzMC4xNjggNTAuMzkyMUwyMzAuNjY3IDUwLjM1NjJDMjMwLjU0NyA0OC42ODg3IDIyOS44MzEgNDcuMjk3NyAyMjguNTQgNDYuMjA5MkwyMjguMjE3IDQ2LjU5MTZaTTIzMC4xNjggNTAuMzkyMUwyMjkuNjcgNTAuNDMwOUwyMzIuNzQxIDg5Ljk0MDFMMjMzLjI0IDg5LjkwMTNMMjMzLjczOCA4OS44NjI2TDIzMC42NjcgNTAuMzUzNEwyMzAuMTY4IDUwLjM5MjFaTTIzMy4yNCA4OS45MDEzTDIzMi43NDEgODkuOTM4MkMyMzIuODUgOTEuNDEgMjMyLjQ1IDkyLjcxMjggMjMxLjUzMiA5My44NzA5TDIzMS45MjQgOTQuMTgxNEwyMzIuMzE2IDk0LjQ5MThDMjMzLjM4OSA5My4xMzczIDIzMy44NjYgOTEuNTg2NyAyMzMuNzM5IDg5Ljg2NDRMMjMzLjI0IDg5LjkwMTNaTTIzMS45MjQgOTQuMTgxNEwyMzEuNTQyIDkzLjg1OTJDMjMwLjYyNCA5NC45NDg5IDIyOS40NDUgOTUuNTc2MyAyMjcuOTc4IDk1Ljc0NjRMMjI4LjAzNiA5Ni4yNDNMMjI4LjA5MyA5Ni43Mzk3QzIyOS44MDUgOTYuNTQxMiAyMzEuMjE5IDk1Ljc5NDEgMjMyLjMwNyA5NC41MDM2TDIzMS45MjQgOTQuMTgxNFpNMjI4LjAzNiA5Ni4yNDNMMjI3Ljk3OCA5NS43NDY0TDIyNi41ODggOTUuOTA3NkwyMjYuNjQ1IDk2LjQwNDNMMjI2LjcwMyA5Ni45MDFMMjI4LjA5MyA5Ni43Mzk3TDIyOC4wMzYgOTYuMjQzWk0yMjkuOTUxIDEwNC4wNzVMMjMwLjAwOCAxMDQuNTcxQzIzMS4zOTIgMTA0LjQxMSAyMzIuNjEyIDEwNC43ODEgMjMzLjY5OCAxMDUuNjk2TDIzNC4wMjEgMTA1LjMxNEwyMzQuMzQzIDEwNC45MzJDMjMzLjA0OSAxMDMuODQxIDIzMS41NTUgMTAzLjM4NSAyMjkuODkzIDEwMy41NzhMMjI5Ljk1MSAxMDQuMDc1Wk0yMzQuMDIxIDEwNS4zMTRMMjMzLjcxIDEwNS43MDZDMjM0Ljg2OCAxMDYuNjIzIDIzNS41MTMgMTA3Ljc2NCAyMzUuNjczIDEwOS4xNDlMMjM2LjE3IDEwOS4wOTJMMjM2LjY2NyAxMDkuMDM0QzIzNi40NzQgMTA3LjM3MyAyMzUuNjg2IDEwNS45OTUgMjM0LjMzMSAxMDQuOTIyTDIzNC4wMjEgMTA1LjMxNFpNMjM2LjE3IDEwOS4wOTJMMjM1LjY3MyAxMDkuMTQ5TDIzNS45NzMgMTExLjczMkwyMzYuNDcgMTExLjY3NEwyMzYuOTY2IDExMS42MTdMMjM2LjY2NyAxMDkuMDM0TDIzNi4xNyAxMDkuMDkyWk0yMzYuNDcgMTExLjY3NEwyMzUuOTczIDExMS43MzJDMjM2LjEzMyAxMTMuMTE3IDIzNS43NjcgMTE0LjM3NSAyMzQuODUgMTE1LjUzM0wyMzUuMjQyIDExNS44NDNMMjM1LjYzNCAxMTYuMTU0QzIzNi43MDcgMTE0Ljc5OSAyMzcuMTU5IDExMy4yNzggMjM2Ljk2NiAxMTEuNjE3TDIzNi40NyAxMTEuNjc0Wk0yMzUuMjQyIDExNS44NDNMMjM0Ljg0IDExNS41NDVDMjMzLjk5MiAxMTYuNjg1IDIzMi44OSAxMTcuMzI0IDIzMS41MDYgMTE3LjQ4NUwyMzEuNTYzIDExNy45ODFMMjMxLjYyMSAxMTguNDc4QzIzMy4yODMgMTE4LjI4NSAyMzQuNjMzIDExNy40OTkgMjM1LjY0MyAxMTYuMTQyTDIzNS4yNDIgMTE1Ljg0M1pNMjMxLjU2MyAxMTcuOTgxTDIzMS41MDYgMTE3LjQ4NUwyMjguMTI4IDExNy44NzZMMjI4LjE4NiAxMTguMzczTDIyOC4yNDQgMTE4Ljg3TDIzMS42MjEgMTE4LjQ3OEwyMzEuNTYzIDExNy45ODFaTTIyOC4xODYgMTE4LjM3M0wyMjguMTI4IDExNy44NzZDMjI2Ljc0NCAxMTguMDM3IDIyNS40ODUgMTE3LjY3IDIyNC4zMjcgMTE2Ljc1M0wyMjQuMDE3IDExNy4xNDVMMjIzLjcwNiAxMTcuNTM3QzIyNS4wNjEgMTE4LjYxIDIyNi41ODIgMTE5LjA2MiAyMjguMjQ0IDExOC44N0wyMjguMTg2IDExOC4zNzNaTTIyNC4wMTcgMTE3LjE0NUwyMjQuMzM5IDExNi43NjNDMjIzLjI0NSAxMTUuODQxIDIyMi42MjUgMTE0LjY5MyAyMjIuNDYzIDExMy4yOThMMjIxLjk2NyAxMTMuMzU2TDIyMS40NyAxMTMuNDE0QzIyMS42NjIgMTE1LjA2NSAyMjIuNDA5IDExNi40NDQgMjIzLjY5NSAxMTcuNTI3TDIyNC4wMTcgMTE3LjE0NVpNMjIxLjk2NyAxMTMuMzU2TDIyMi40NjMgMTEzLjI5OEwyMjIuMTY0IDExMC43MTZMMjIxLjY2NyAxMTAuNzczTDIyMS4xNzEgMTEwLjgzMUwyMjEuNDcgMTEzLjQxNEwyMjEuOTY3IDExMy4zNTZaTTIyMS42NjcgMTEwLjc3M0wyMjIuMTY0IDExMC43MTZDMjIyLjAwMiAxMDkuMzIxIDIyMi4zNDMgMTA4LjA2MiAyMjMuMTk3IDEwNi45MTRMMjIyLjc5NiAxMDYuNjE2TDIyMi4zOTUgMTA2LjMxN0MyMjEuMzkxIDEwNy42NjYgMjIwLjk3OSAxMDkuMTc5IDIyMS4xNzEgMTEwLjgzMUwyMjEuNjY3IDExMC43NzNaTTIyMi43OTYgMTA2LjYxNkwyMjMuMTg4IDEwNi45MjZDMjI0LjEwNSAxMDUuNzY5IDIyNS4yNDYgMTA1LjEyNCAyMjYuNjMxIDEwNC45NjNMMjI2LjU3MyAxMDQuNDY2TDIyNi41MTYgMTAzLjk3QzIyNC44NTQgMTA0LjE2MiAyMjMuNDc3IDEwNC45NSAyMjIuNDA0IDEwNi4zMDVMMjIyLjc5NiAxMDYuNjE2Wk0yMjYuNTczIDEwNC40NjZMMjI2LjYzMSAxMDQuOTYzTDIzMC4wMDggMTA0LjU3MUwyMjkuOTUxIDEwNC4wNzVMMjI5Ljg5MyAxMDMuNTc4TDIyNi41MTYgMTAzLjk3TDIyNi41NzMgMTA0LjQ2NlpNMjQ1LjI2MiA5Mi42MzQ2TDI0NC45NTIgOTMuMDI2NUwyNDQuOTU4IDkzLjAzMTJMMjQ0Ljk2NCA5My4wMzU4TDI0NS4yNjIgOTIuNjM0NlpNMjQzLjAwMiA4OC43NjkzTDI0Mi41MDggODguODQ1NkwyNDIuNTA4IDg4Ljg0NzVMMjQzLjAwMiA4OC43NjkzWk0yMzYuOTUgNDkuNjA1N0wyMzcuNDQ0IDQ5LjUyOTNMMjM3LjQ0NCA0OS41MjY1TDIzNi45NSA0OS42MDU3Wk0yMzcuOTc5IDQ1LjQ1OTVMMjM3LjU3OCA0NS4xNjFMMjM3LjU3OCA0NS4xNjFMMjM3Ljk3OSA0NS40NTk1Wk0yNTEuMDkxIDQzLjkzODlMMjUxLjQxNCA0My41NTY2VjQzLjU1NjZMMjUxLjA5MSA0My45Mzg5Wk0yNTMuMDQyIDQ3LjczOTVMMjUyLjU0NCA0Ny43NzU1TDI1Mi41NDQgNDcuNzc4MkwyNTMuMDQyIDQ3LjczOTVaTTI1Ni4xMTQgODcuMjQ4N0wyNTYuNjEzIDg3LjIxMThMMjU2LjYxMiA4Ny4yMDk5TDI1Ni4xMTQgODcuMjQ4N1pNMjU0Ljc5OCA5MS41Mjg3TDI1NS4xODEgOTEuODUwOUwyNTUuMTg1IDkxLjg0NTFMMjU1LjE5IDkxLjgzOTJMMjU0Ljc5OCA5MS41Mjg3Wk0yNTYuODk1IDEwMi42NjFMMjU2LjU3MiAxMDMuMDQ0TDI1Ni41NzggMTAzLjA0OUwyNTYuNTg0IDEwMy4wNTNMMjU2Ljg5NSAxMDIuNjYxWk0yNTguMTE2IDExMy4xOTFMMjU3LjcyNCAxMTIuODhMMjU3LjcxOSAxMTIuODg2TDI1Ny43MTUgMTEyLjg5MkwyNTguMTE2IDExMy4xOTFaTTI0Ni44OTEgMTE0LjQ5MkwyNDYuNTY5IDExNC44NzVMMjQ2LjU3NSAxMTQuODhMMjQ2LjU4IDExNC44ODRMMjQ2Ljg5MSAxMTQuNDkyWk0yNDUuNjcgMTAzLjk2M0wyNDUuMjc4IDEwMy42NTNMMjQ1LjI3MyAxMDMuNjU5TDI0NS4yNjkgMTAzLjY2NUwyNDUuNjcgMTAzLjk2M1pNMjQ5LjUxOSA5My43NTE2TDI0OS40NjIgOTMuMjU1QzI0Ny45OTUgOTMuNDI1MSAyNDYuNzA0IDkzLjA4NDEgMjQ1LjU2MSA5Mi4yMzM0TDI0NS4yNjIgOTIuNjM0NkwyNDQuOTY0IDkzLjAzNThDMjQ2LjMxOCA5NC4wNDMxIDI0Ny44NjUgOTQuNDQ2OSAyNDkuNTc3IDk0LjI0ODNMMjQ5LjUxOSA5My43NTE2Wk0yNDUuMjYyIDkyLjYzNDZMMjQ1LjU3MyA5Mi4yNDI3QzI0NC40MTQgOTEuMzI1MiAyNDMuNzI3IDkwLjE0ODcgMjQzLjQ5NiA4OC42OTFMMjQzLjAwMiA4OC43NjkzTDI0Mi41MDggODguODQ3NUMyNDIuNzc4IDkwLjU1MzEgMjQzLjU5NyA5MS45NTM1IDI0NC45NTIgOTMuMDI2NUwyNDUuMjYyIDkyLjYzNDZaTTI0My4wMDIgODguNzY5M0wyNDMuNDk2IDg4LjY5MjlMMjM3LjQ0NCA0OS41MjkzTDIzNi45NSA0OS42MDU3TDIzNi40NTYgNDkuNjgyTDI0Mi41MDggODguODQ1NkwyNDMuMDAyIDg4Ljc2OTNaTTIzNi45NSA0OS42MDU3TDIzNy40NDQgNDkuNTI2NUMyMzcuMjIzIDQ4LjE0NjQgMjM3LjUzMSA0Ni45MDAxIDIzOC4zOCA0NS43NThMMjM3Ljk3OSA0NS40NTk1TDIzNy41NzggNDUuMTYxQzIzNi41NyA0Ni41MTYyIDIzNi4xOTIgNDguMDM0IDIzNi40NTYgNDkuNjg0OEwyMzYuOTUgNDkuNjA1N1pNMjM3Ljk3OSA0NS40NTk1TDIzOC4zOCA0NS43NThDMjM5LjIyOSA0NC42MTggMjQwLjMzMSA0My45Nzg3IDI0MS43MTUgNDMuODE4MkwyNDEuNjU4IDQzLjMyMTZMMjQxLjYgNDIuODI0OUMyMzkuOTM4IDQzLjAxNzcgMjM4LjU4OCA0My44MDM3IDIzNy41NzggNDUuMTYxTDIzNy45NzkgNDUuNDU5NVpNMjQxLjY1OCA0My4zMjE2TDI0MS43MTUgNDMuODE4MkwyNDcuMDc5IDQzLjE5NjJMMjQ3LjAyMiA0Mi42OTk1TDI0Ni45NjQgNDIuMjAyOEwyNDEuNiA0Mi44MjQ5TDI0MS42NTggNDMuMzIxNlpNMjQ3LjAyMiA0Mi42OTk1TDI0Ny4wNzkgNDMuMTk2MkMyNDguNDYzIDQzLjAzNTcgMjQ5LjY4MyA0My40MDU3IDI1MC43NjkgNDQuMzIxM0wyNTEuMDkxIDQzLjkzODlMMjUxLjQxNCA0My41NTY2QzI1MC4xMiA0Mi40NjYzIDI0OC42MjYgNDIuMDEgMjQ2Ljk2NCA0Mi4yMDI4TDI0Ny4wMjIgNDIuNjk5NVpNMjUxLjA5MSA0My45Mzg5TDI1MC43NjkgNDQuMzIxM0MyNTEuODU4IDQ1LjIzODYgMjUyLjQ0MyA0Ni4zODEzIDI1Mi41NDQgNDcuNzc1NUwyNTMuMDQyIDQ3LjczOTVMMjUzLjU0MSA0Ny43MDM1QzI1My40MjEgNDYuMDM2MSAyNTIuNzA1IDQ0LjY0NSAyNTEuNDE0IDQzLjU1NjZMMjUxLjA5MSA0My45Mzg5Wk0yNTMuMDQyIDQ3LjczOTVMMjUyLjU0NCA0Ny43NzgyTDI1NS42MTUgODcuMjg3NEwyNTYuMTE0IDg3LjI0ODdMMjU2LjYxMiA4Ny4yMDk5TDI1My41NDEgNDcuNzAwN0wyNTMuMDQyIDQ3LjczOTVaTTI1Ni4xMTQgODcuMjQ4N0wyNTUuNjE1IDg3LjI4NTVDMjU1LjcyNCA4OC43NTc0IDI1NS4zMjQgOTAuMDYwMSAyNTQuNDA2IDkxLjIxODNMMjU0Ljc5OCA5MS41Mjg3TDI1NS4xOSA5MS44MzkyQzI1Ni4yNjMgOTAuNDg0NiAyNTYuNzQgODguOTM0IDI1Ni42MTMgODcuMjExOEwyNTYuMTE0IDg3LjI0ODdaTTI1NC43OTggOTEuNTI4N0wyNTQuNDE2IDkxLjIwNjVDMjUzLjQ5OCA5Mi4yOTYzIDI1Mi4zMTkgOTIuOTIzNiAyNTAuODUyIDkzLjA5MzdMMjUwLjkxIDkzLjU5MDRMMjUwLjk2NyA5NC4wODdDMjUyLjY3OSA5My44ODg1IDI1NC4wOTMgOTMuMTQxNCAyNTUuMTgxIDkxLjg1MDlMMjU0Ljc5OCA5MS41Mjg3Wk0yNTAuOTEgOTMuNTkwNEwyNTAuODUyIDkzLjA5MzdMMjQ5LjQ2MiA5My4yNTVMMjQ5LjUxOSA5My43NTE2TDI0OS41NzcgOTQuMjQ4M0wyNTAuOTY3IDk0LjA4N0wyNTAuOTEgOTMuNTkwNFpNMjUyLjgyNSAxMDEuNDIyTDI1Mi44ODIgMTAxLjkxOUMyNTQuMjY2IDEwMS43NTggMjU1LjQ4NiAxMDIuMTI4IDI1Ni41NzIgMTAzLjA0NEwyNTYuODk1IDEwMi42NjFMMjU3LjIxNyAxMDIuMjc5QzI1NS45MjMgMTAxLjE4OSAyNTQuNDMgMTAwLjczMiAyNTIuNzY3IDEwMC45MjVMMjUyLjgyNSAxMDEuNDIyWk0yNTYuODk1IDEwMi42NjFMMjU2LjU4NCAxMDMuMDUzQzI1Ny43NDIgMTAzLjk3IDI1OC4zODcgMTA1LjExMiAyNTguNTQ3IDEwNi40OTZMMjU5LjA0NCAxMDYuNDM5TDI1OS41NDEgMTA2LjM4MUMyNTkuMzQ4IDEwNC43MiAyNTguNTYgMTAzLjM0MyAyNTcuMjA1IDEwMi4yNjlMMjU2Ljg5NSAxMDIuNjYxWk0yNTkuMDQ0IDEwNi40MzlMMjU4LjU0NyAxMDYuNDk2TDI1OC44NDcgMTA5LjA3OUwyNTkuMzQ0IDEwOS4wMjJMMjU5Ljg0IDEwOC45NjRMMjU5LjU0MSAxMDYuMzgxTDI1OS4wNDQgMTA2LjQzOVpNMjU5LjM0NCAxMDkuMDIyTDI1OC44NDcgMTA5LjA3OUMyNTkuMDA4IDExMC40NjQgMjU4LjY0MSAxMTEuNzIzIDI1Ny43MjQgMTEyLjg4TDI1OC4xMTYgMTEzLjE5MUwyNTguNTA4IDExMy41MDFDMjU5LjU4MSAxMTIuMTQ2IDI2MC4wMzMgMTEwLjYyNSAyNTkuODQgMTA4Ljk2NEwyNTkuMzQ0IDEwOS4wMjJaTTI1OC4xMTYgMTEzLjE5MUwyNTcuNzE1IDExMi44OTJDMjU2Ljg2NiAxMTQuMDMyIDI1NS43NjQgMTE0LjY3MiAyNTQuMzggMTE0LjgzMkwyNTQuNDM3IDExNS4zMjlMMjU0LjQ5NSAxMTUuODI1QzI1Ni4xNTcgMTE1LjYzMyAyNTcuNTA3IDExNC44NDcgMjU4LjUxNyAxMTMuNDg5TDI1OC4xMTYgMTEzLjE5MVpNMjU0LjQzNyAxMTUuMzI5TDI1NC4zOCAxMTQuODMyTDI1MS4wMDMgMTE1LjIyNEwyNTEuMDYgMTE1LjcyTDI1MS4xMTggMTE2LjIxN0wyNTQuNDk1IDExNS44MjVMMjU0LjQzNyAxMTUuMzI5Wk0yNTEuMDYgMTE1LjcyTDI1MS4wMDMgMTE1LjIyNEMyNDkuNjE4IDExNS4zODQgMjQ4LjM1OSAxMTUuMDE4IDI0Ny4yMDEgMTE0LjEwMUwyNDYuODkxIDExNC40OTJMMjQ2LjU4IDExNC44ODRDMjQ3LjkzNSAxMTUuOTU4IDI0OS40NTYgMTE2LjQxIDI1MS4xMTggMTE2LjIxN0wyNTEuMDYgMTE1LjcyWk0yNDYuODkxIDExNC40OTJMMjQ3LjIxMyAxMTQuMTFDMjQ2LjExOSAxMTMuMTg4IDI0NS40OTkgMTEyLjA0IDI0NS4zMzcgMTEwLjY0NkwyNDQuODQxIDExMC43MDNMMjQ0LjM0NCAxMTAuNzYxQzI0NC41MzYgMTEyLjQxMyAyNDUuMjgzIDExMy43OTEgMjQ2LjU2OSAxMTQuODc1TDI0Ni44OTEgMTE0LjQ5MlpNMjQ0Ljg0MSAxMTAuNzAzTDI0NS4zMzcgMTEwLjY0NkwyNDUuMDM4IDEwOC4wNjNMMjQ0LjU0MSAxMDguMTIxTDI0NC4wNDUgMTA4LjE3OEwyNDQuMzQ0IDExMC43NjFMMjQ0Ljg0MSAxMTAuNzAzWk0yNDQuNTQxIDEwOC4xMjFMMjQ1LjAzOCAxMDguMDYzQzI0NC44NzYgMTA2LjY2OSAyNDUuMjE3IDEwNS40MSAyNDYuMDcxIDEwNC4yNjJMMjQ1LjY3IDEwMy45NjNMMjQ1LjI2OSAxMDMuNjY1QzI0NC4yNjUgMTA1LjAxNCAyNDMuODUzIDEwNi41MjcgMjQ0LjA0NSAxMDguMTc4TDI0NC41NDEgMTA4LjEyMVpNMjQ1LjY3IDEwMy45NjNMMjQ2LjA2MiAxMDQuMjc0QzI0Ni45NzkgMTAzLjExNiAyNDguMTIgMTAyLjQ3MSAyNDkuNTA1IDEwMi4zMUwyNDkuNDQ3IDEwMS44MTRMMjQ5LjM5IDEwMS4zMTdDMjQ3LjcyOCAxMDEuNTEgMjQ2LjM1MSAxMDIuMjk4IDI0NS4yNzggMTAzLjY1M0wyNDUuNjcgMTAzLjk2M1pNMjQ5LjQ0NyAxMDEuODE0TDI0OS41MDUgMTAyLjMxTDI1Mi44ODIgMTAxLjkxOUwyNTIuODI1IDEwMS40MjJMMjUyLjc2NyAxMDAuOTI1TDI0OS4zOSAxMDEuMzE3TDI0OS40NDcgMTAxLjgxNFoiIGZpbGw9IndoaXRlIiBtYXNrPSJ1cmwoI3BhdGgtNC1vdXRzaWRlLTFfNDRfMTQ0KSIvPgo8L3N2Zz4K" /><div class="img-resize-handle"></div></div>
<div><br></div>
<div>have fun!</div>
<div><br></div>
"#.to_string(),
            color: "#FFD6E8".to_string(),
            rotation: -1.5,
        };
        write_notes(app, &vec![intro]);
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

            seed_intro_note_if_needed(app.handle());

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
            show_main_window,
            show_note_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}