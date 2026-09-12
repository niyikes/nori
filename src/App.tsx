import { useEffect, useRef, useState } from "react";
import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import nori1 from "./assets/cat/nori7.svg";
import nori2 from "./assets/cat/nori2.svg";
import nori3 from "./assets/cat/nori3.svg";
import nori4 from "./assets/cat/nori4.svg";
import nori5 from "./assets/cat/nori5.svg";
import nori6 from "./assets/cat/nori6.svg";

type Note = {
  id: string;
  text: string;
  color: string;
  rotation: number;
};

const HEADER_COLORS = ["#D6E8FF", "#FFD6E8", "#D6F5D6", "#F0D6FF", "#FFF6C9", "#FFE8CC"];

const CAT_FACES = {
  tired: nori1,
  dizzy: nori2,
  shock: nori3,
  normal: nori4,
  sleep: nori5,
  cute: nori6,
};

const DAY_MESSAGES = [
  { face: "normal", text: "let's make a note!" },
  { face: "shock", text: "new note time?" },
  { face: "dizzy", text: "i forgot what i was gna say" },
  { face: "tired", text: "these sticky notes arent really sticky." },
  { face: "cute", text: "yay a new note!" },
  { face: "normal", text: "hiiiiiii im nori!!!!" },
  { face: "shock", text: "alt-N for a new note!" },
  { face: "dizzy", text: "dang look at all these cool notes" },
  { face: "tired", text: "clean up on aisle nori!" },
  { face: "cute", text: "she sticky on my note till i nori" },
];

const NIGHT_MESSAGES = [
  { face: "sleep", text: "zzz... it's late" },
  { face: "sleep", text: "still up? me too" },
  { face: "sleep", text: "gahdamn go to bed..." },
];

function isNightTime() {
  const hour = new Date().getHours();
  return hour >= 24 || hour < 6;
}

function randomMessage() {
  const pool = isNightTime() ? NIGHT_MESSAGES : DAY_MESSAGES;
  return pool[Math.floor(Math.random() * pool.length)];
}

function randomHeaderColor() {
  return HEADER_COLORS[Math.floor(Math.random() * HEADER_COLORS.length)];
}

function CatFace({ face }: { face: string }) {
  const src = CAT_FACES[face as keyof typeof CAT_FACES] || nori1;
  return <img src={src} className="cat-face" alt="" />;
}

function stripHtml(html: string) {
  const div = document.createElement("div");
  div.innerHTML = html;

  div.querySelectorAll(".todo-item, .bullet-item").forEach((el) => {
    el.insertAdjacentText("afterbegin", "• ");
  });

  div.querySelectorAll("div, p, br").forEach((el) => {
    el.insertAdjacentText("beforebegin", "\n");
  });

  return (div.textContent || div.innerText || "").replace(/\n\s*\n/g, "\n").trim();
}

function App() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [query, setQuery] = useState("");
  const [headerColor] = useState(randomHeaderColor);
  const [current, setCurrent] = useState(randomMessage);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const win = getCurrentWindow();

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
    setCurrent(randomMessage());
    invoke("open_note_window", { noteId: null });
  }

  function openExisting(id: string) {
    invoke("open_note_window", { noteId: id });
  }

  async function deleteNote(id: string, e: React.MouseEvent) {
    e.stopPropagation();
    setDeletingId(id);
    setTimeout(async () => {
      await invoke("delete_note", { id });
      setDeletingId(null);
      refreshNotes();
    }, 250);
  }

  const filteredNotes = query.trim()
    ? notes.filter((n) => stripHtml(n.text).toLowerCase().includes(query.toLowerCase()))
    : notes;

  return (
    <div className="app">
      <div className="mascot-header" style={{ background: headerColor }}>
        <div className="titlebar" data-tauri-drag-region>
          <span className="titlebar-title">nori</span>

          <div className="titlebar-controls">
            <button className="titlebar-min" onClick={() => win.minimize()}>
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round">
                <path d="M5 12h14" />
              </svg>
            </button>
            <button className="titlebar-close" onClick={() => win.close()}>
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>
        <div className="mascot-row">
          <CatFace face={current.face} />
          <div className="speech-bubble">{current.text}</div>
        </div>
      </div>

      <div className="content">
        <div className="toolbar">
          <input
            className="search-input"
            type="text"
            placeholder="search notes..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button className="add-btn" onClick={openNewNote} title="new note">+</button>
        </div>

        {filteredNotes.length === 0 ? (
          <div className="empty-state">
            {query ? "nothing matches that" : "no notes yet, make one!"}
          </div>
        ) : (
          <div className="notes-grid">
            {filteredNotes.map((note, i) => (
              <div
                key={note.id}
                className={`note-thumb ${note.id === deletingId ? "deleting" : ""}`}
                onClick={() => {
                  if (note.id !== deletingId) openExisting(note.id);
                }}
                style={{
                  background: note.color,
                  transform: `rotate(${note.rotation}deg)`,
                  animationDelay: `${i * 0.04}s`,
                }}
              >
                <button className="thumb-delete" onClick={(e) => deleteNote(note.id, e)}>x</button>
                <div className="thumb-text">
                  {stripHtml(note.text) || <span className="empty">empty note</span>}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default App;