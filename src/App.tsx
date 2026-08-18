import { useEffect, useRef, useState } from "react";
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
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [ghostPos, setGhostPos] = useState({ x: 0, y: 0 });
  const isDragging = useRef(false);
  const currentNotes = useRef<Note[]>([]);

  useEffect(() => {
    currentNotes.current = notes;
  }, [notes]);

  async function refreshNotes() {
    if (isDragging.current) return;
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

  function handlePointerDown(e: React.PointerEvent, id: string) {
    isDragging.current = true;
    setDraggingId(id);
    setGhostPos({ x: e.clientX, y: e.clientY });
  }

  function handlePointerMove(e: React.PointerEvent) {
    if (!isDragging.current) return;
    setGhostPos({ x: e.clientX, y: e.clientY });

    const el = document.elementFromPoint(e.clientX, e.clientY);
    const thumb = el?.closest(".note-thumb") as HTMLElement | null;
    const overId = thumb?.dataset.id;

    if (overId && overId !== draggingId) {
      setNotes((prev) => {
        const fromIndex = prev.findIndex((n) => n.id === draggingId);
        const toIndex = prev.findIndex((n) => n.id === overId);
        if (fromIndex === -1 || toIndex === -1) return prev;
        const updated = [...prev];
        const [moved] = updated.splice(fromIndex, 1);
        updated.splice(toIndex, 0, moved);
        return updated;
      });
    }
  }

  function handlePointerUp() {
    if (!isDragging.current) return;
    isDragging.current = false;
    setDraggingId(null);
    invoke("reorder_notes", { orderedIds: currentNotes.current.map((n) => n.id) });
  }

  const draggedNote = notes.find((n) => n.id === draggingId);

  return (
    <div
      className="app"
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerLeave={handlePointerUp}
    >
      <h1>nori</h1>
      <button className="add-btn" onClick={openNewNote}>+ new note</button>

      {notes.length === 0 ? (
        <div className="empty-state">no notes yet, make one</div>
      ) : (
        <div className="notes-grid">
          {notes.map((note, i) => (
            <div
              key={note.id}
              data-id={note.id}
              className={`note-thumb ${note.id === draggingId ? "placeholder" : ""}`}
              onPointerDown={(e) => handlePointerDown(e, note.id)}
              onClick={() => {
                if (!isDragging.current) openExisting(note.id);
              }}
              style={{
                background: note.color,
                transform: `rotate(${note.rotation}deg)`,
                animationDelay: `${i * 0.04}s`,
              }}
            >
              {note.text || <span className="empty">empty note</span>}
            </div>
          ))}
        </div>
      )}

      {draggedNote && (
        <div
          className="note-thumb ghost"
          style={{
            background: draggedNote.color,
            left: ghostPos.x,
            top: ghostPos.y,
            transform: `translate(-50%, -50%) rotate(${draggedNote.rotation}deg) scale(1.1)`,
          }}
        >
          {draggedNote.text || <span className="empty">empty note</span>}
        </div>
      )}
    </div>
  );
}

export default App;