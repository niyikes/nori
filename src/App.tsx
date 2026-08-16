import { useState } from "react";
import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Note = {
  id: number;
  text: string;
};

function App() {
  const thisWindow = getCurrentWindow();
  const isNoteWindow = thisWindow.label.startsWith("note-");

  const [notes, setNotes] = useState<Note[]>([]);

  function openNewNote() {
    invoke("open_note_window");
  }

  function deleteNote(id: number) {
    setNotes(notes.filter((note) => note.id !== id));
  }

  return (
    <div className="app">
      {isNoteWindow ? (
        <div className="note-window">
          <textarea placeholder="type your note..." />
        </div>
      ) : (
        <>
          <h1>nori</h1>
          <button onClick={openNewNote}>add</button>
          <div className="notes-grid">
            {notes.map((note) => (
              <div key={note.id} className="note">
                {note.text}
                <button onClick={() => deleteNote(note.id)}>x</button>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

export default App;