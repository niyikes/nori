import { useState } from "react";
import "./App.css";
import { invoke } from "@tauri-apps/api/core";

type Note = {
  id: number;
  text: string;
};

function App() {
  const [notes] = useState<Note[]>([]);

  function openNewNote() {
    invoke("open_note_window");
  }

  return (
    <div className="app">
      <h1>nori</h1>
      <button onClick={openNewNote}>add</button>
      <div className="notes-grid">
        {notes.map((note) => (
          <div key={note.id} className="note">{note.text}</div>
        ))}
      </div>
    </div>
  );
}

export default App;