import React, { useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./note.css";
import { getCurrentWindow } from "@tauri-apps/api/window";

function NoteApp() {
    const thisWindow = getCurrentWindow();
    const noteId = thisWindow.label;

    const [text, setText] = useState("");
    const [color, setColor] = useState("#FFF6C9");
    const saveTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

    useEffect(() => {
        invoke<{id: String; text: String; color: String; rotation: Number }[]>("get_notes").then((notes) => {
            const existing = notes.find((n) => n.id ===noteId);
            if (existing) {
                setText(existing.text);
                setColor(existing.color)
            }
        });
    }, [noteId]);

    function handleChange(value: string) {
        setText(value);
        if (saveTimeout.current) clearTimeout(saveTimeout.current);
        saveTimeout.current = setTimeout(() => {
            invoke("save_note", {id: noteId, text: value});
        }, 400);
    }

    return (
        <div
        className="note-paper"
        style={{ background: color }}
        >
        <textarea
            autoFocus
            value={text}
            placeholder="type your note..."
            onChange={(e) => handleChange(e.target.value)}
        />
        </div>
    );
    }

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <NoteApp />
  </React.StrictMode>
);