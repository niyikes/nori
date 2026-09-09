import { useEffect, useRef, useState } from "react";
import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Note = {
  id: string;
  text: string;
  color: string;
  rotation: number;
};

function stripHtml(html: string) {
  const div = document.createElement("div");
  div.innerHTML = html;
  return div.textContent || div.innerText || "";
}

function App() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [query, setQuery] = useState("");
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [ghostPos, setGhostPos] = useState({ x: 0, y: 0 });
  const isDragging = useRef(false);
  const currentNotes = useRef<Note[]>([]);
  const win = getCurrentWindow();

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

  async function deleteNote(id: string, e: React.MouseEvent) {
    e.stopPropagation();
    await invoke("delete_note", { id });
    refreshNotes();
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

  const filteredNotes = query.trim()
    ? notes.filter((n) => stripHtml(n.text).toLowerCase().includes(query.toLowerCase()))
    : notes;

  return (
    <div
      className="app"
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerLeave={handlePointerUp}
    >
      <div className="titlebar" data-tauri-drag-region>
        <span className="titlebar-title">nori</span>
        <div className="titlebar-controls">
          <button onClick={() => win.minimize()}>-</button>
          <button onClick={() => win.toggleMaximize()}>o</button>
          <button className="titlebar-close" onClick={() => win.close()}>x</button>
        </div>
      </div>

      <div className="content">
        <h1>nori</h1>
        <button className="add-btn" onClick={openNewNote}>+ new note</button>

        <input
          className="search-input"
          type="text"
          placeholder="search notes..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />

        {filteredNotes.length === 0 ? (
          <div className="empty-state">
            {query ? "no matching notes" : "no notes yet, make one"}
          </div>
        ) : (
          <div className="notes-grid">
            {filteredNotes.map((note, i) => (
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
                <button className="thumb-delete" onClick={(e) => deleteNote(note.id, e)}>x</button>
                {stripHtml(note.text) || <span className="empty">empty note</span>}
              </div>
            ))}
          </div>
        )}
      </div>

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
          {stripHtml(draggedNote.text) || <span className="empty">empty note</span>}
        </div>
      )}
    </div>
  );
}

export default App;