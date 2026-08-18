import { useEffect, useState } from "react";
import "./App.css";
import { invoke } from "@tauri-apps/api/core";

type Note = {
  id: string;
  text: string;
  color: string;
  rotation: number;
};

function App() {
  const [notes, setNotes] = useState<Note[]>([]);

  async function refreshNotes() {
    const result = await invoke<Note[]>("get_notes");
    setNotes(result);
  }

  useEffect(() => {
    refreshNotes();
    const interval = setInterval(refreshNotes, 1000);
    return () => clearInterval(interval);
  }, []);

  function openNewNote() {
    invoke("open_note_window", { noteId: null });
  }

  function openExisting(id: string) {
    invoke("open_note_window", { noteId: id });
  }

  return (
    <div className="app">
      <h1>nori</h1>
      <button className="add-btn" onClick={openNewNote}>add</button>
      <div className="notes-grid">
        {notes.map((note) => (
          <div
            key={note.id}
            className="note-thumb"
            style={{
              background: note.color,
              transform: `rotate(${note.rotation}deg)`,
            }}
            onClick={() => openExisting(note.id)}
          >
            {note.text || <span className="empty">empty note</span>}
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;