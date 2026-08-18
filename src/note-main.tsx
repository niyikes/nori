import React, { useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./note.css";

const PASTELS = ["#FFE8CC", "#FFD6E8", "#D6F5D6", "#D6E8FF", "#F0D6FF", "#FFF6C9", "#FFDDE1", "#E0FFF4"];

function randomPastel() {
  return PASTELS[Math.floor(Math.random() * PASTELS.length)];
}

function randomTilt() {
  return (Math.random() * 5) - 2.5;
}

function NoteApp() {
  const thisWindow = getCurrentWindow();
  const noteId = thisWindow.label;

  const [text, setText] = useState("");
  const [color, setColor] = useState(randomPastel());
  const [rotation] = useState(randomTilt());
  const textRef = useRef("");
  const saveTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<{ id: string; text: string; color: string; rotation: number }[]>("get_notes").then((notes) => {
      const existing = notes.find((n) => n.id === noteId);
      if (existing) {
        setText(existing.text);
        setColor(existing.color);
        textRef.current = existing.text;
      } else {
        invoke("save_note", { id: noteId, text: "", color, rotation });
      }
    });
  }, []);

  function handleChange(value: string) {
    setText(value);
    textRef.current = value;
    if (saveTimeout.current) clearTimeout(saveTimeout.current);
    saveTimeout.current = setTimeout(() => {
      invoke("save_note", { id: noteId, text: value, color, rotation });
    }, 300);
  }

  async function handleClose() {
    if (saveTimeout.current) clearTimeout(saveTimeout.current);
    await invoke("save_note", { id: noteId, text: textRef.current, color, rotation });
    await thisWindow.close();
  }

  return (
    <div className="note-paper" style={{ background: color }}>
      <div className="note-header" data-tauri-drag-region>
        <span className="note-dot" />
        <button className="note-close" onClick={handleClose}>×</button>
      </div>
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