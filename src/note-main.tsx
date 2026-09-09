import React, { useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./note.css";

const PASTELS = ["#FFE8CC", "#FFD6E8", "#D6F5D6", "#D6E8FF", "#F0D6FF", "#FFF6C9", "#FFDDE1", "#E0FFF4"];

const HIGHLIGHT_MAP: Record<string, string> = {
  "#FFE8CC": "#E8A855",
  "#FFD6E8": "#E85D9A",
  "#D6F5D6": "#5CB85C",
  "#D6E8FF": "#4A90D9",
  "#F0D6FF": "#A855D8",
  "#FFF6C9": "#D9B800",
  "#FFDDE1": "#E8607B",
  "#E0FFF4": "#3AAE8F",
};

function randomPastel() {
  return PASTELS[Math.floor(Math.random() * PASTELS.length)];
}

function randomTilt() {
  return (Math.random() * 5) - 2.5;
}

function darkenColor(hex: string, amount: number) {
  const num = parseInt(hex.replace("#", ""), 16);
  const r = Math.max(0, (num >> 16) - amount);
  const g = Math.max(0, ((num >> 8) & 0x00ff) - amount);
  const b = Math.max(0, (num & 0x0000ff) - amount);
  return `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
}

function lightenColor(hex: string, amount: number) {
  const num = parseInt(hex.replace("#", ""), 16);
  const r = Math.min(255, (num >> 16) + amount);
  const g = Math.min(255, ((num >> 8) & 0x00ff) + amount);
  const b = Math.min(255, (num & 0x0000ff) + amount);
  return `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
}

function getContrastText(hex: string) {
  const num = parseInt(hex.replace("#", ""), 16);
  const r = (num >> 16) & 0xff;
  const g = (num >> 8) & 0xff;
  const b = num & 0xff;
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance > 0.6 ? "#4a4a4a" : "#ffffff";
}

const initialColor = randomPastel();
const initialRotation = randomTilt();

document.documentElement.style.background = initialColor;
document.body.style.background = initialColor;

const BUTTON_BASE = 16;
const BUTTON_MAX = 24;
const MAGNIFY_RADIUS = 40;

function NoteApp() {
  const thisWindow = getCurrentWindow();
  const noteId = thisWindow.label;

  const [color, setColor] = useState(initialColor);
  const [rotation] = useState(initialRotation);
  const [loaded, setLoaded] = useState(false);
  const [toolbarPos, setToolbarPos] = useState<{ x: number; y: number } | null>(null);
  const [scales, setScales] = useState([1, 1, 1]);
  const editorRef = useRef<HTMLDivElement>(null);
  const toolbarRef = useRef<HTMLDivElement>(null);
  const htmlRef = useRef("");
  const saveTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<{ id: string; text: string; color: string; rotation: number }[]>("get_notes").then((notes) => {
      const existing = notes.find((n) => n.id === noteId);
      if (existing) {
        htmlRef.current = existing.text;
        setColor(existing.color);
        document.documentElement.style.background = existing.color;
        document.body.style.background = existing.color;
        if (editorRef.current) editorRef.current.innerHTML = existing.text;
      } else {
        invoke("save_note", { id: noteId, text: "", color: initialColor, rotation: initialRotation });
      }
      setLoaded(true);
    });
  }, []);

  useEffect(() => {
    const selectionColor = HIGHLIGHT_MAP[color] || darkenColor(color, 60);
    const styleId = "note-selection-style";
    let styleEl = document.getElementById(styleId) as HTMLStyleElement | null;
    if (!styleEl) {
      styleEl = document.createElement("style");
      styleEl.id = styleId;
      document.head.appendChild(styleEl);
    }
    styleEl.textContent = `
      .note-editor::selection, .note-editor *::selection {
        background: ${selectionColor};
        color: white;
      }
    `;
  }, [color]);

  function scheduleSave(value: string) {
    htmlRef.current = value;
    if (saveTimeout.current) clearTimeout(saveTimeout.current);
    saveTimeout.current = setTimeout(() => {
      invoke("save_note", { id: noteId, text: value, color, rotation });
    }, 300);
  }

  function handleInput() {
    const value = editorRef.current?.innerHTML || "";
    scheduleSave(value);
  }

  function updateToolbarPosition() {
    const selection = window.getSelection();
    if (!selection || selection.isCollapsed || selection.rangeCount === 0) {
      setToolbarPos(null);
      return;
    }
    const range = selection.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0) {
      setToolbarPos(null);
      return;
    }
    setToolbarPos({
      x: rect.left + rect.width / 2,
      y: rect.top - 10,
    });
    setScales([1, 1, 1]);
  }

  function applyFormat(command: string) {
    editorRef.current?.focus();
    document.execCommand(command);
    handleInput();
    updateToolbarPosition();
  }

  function handleToolbarMouseMove(e: React.MouseEvent) {
    if (!toolbarRef.current) return;
    const buttons = Array.from(toolbarRef.current.querySelectorAll(".format-btn"));
    const newScales = buttons.map((btn) => {
      const rect = btn.getBoundingClientRect();
      const centerX = rect.left + rect.width / 2;
      const dist = Math.abs(e.clientX - centerX);
      const t = Math.max(0, 1 - dist / MAGNIFY_RADIUS);
      const scale = 1 + t * (BUTTON_MAX / BUTTON_BASE - 1);
      return scale;
    });
    setScales(newScales);
  }

  function handleToolbarMouseLeave() {
    setScales([1, 1, 1]);
  }

  async function handleClose() {
    if (saveTimeout.current) clearTimeout(saveTimeout.current);
    await invoke("save_note", { id: noteId, text: htmlRef.current, color: color, rotation: rotation });
    await thisWindow.close();
  }

  function handleMenu() {
    invoke("show_main_window");
  }

  const toolbarText = getContrastText(color);

  return (
    <div className="note-paper" style={{ background: color, opacity: loaded ? 1 : 0 }}>
      <div className="note-header" data-tauri-drag-region>
        <button className="note-menu" onClick={handleMenu} title="menu">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
            <path d="M3 6h18M3 12h18M3 18h18" />
          </svg>
        </button>
        <button className="note-close" onClick={handleClose} title="close">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div
        ref={editorRef}
        className="note-editor"
        contentEditable
        suppressContentEditableWarning
        onInput={handleInput}
        onMouseUp={updateToolbarPosition}
        onKeyUp={updateToolbarPosition}
        onBlur={() => setTimeout(() => setToolbarPos(null), 150)}
        data-placeholder="type your note..."
      />

      {toolbarPos && (
        <div
          ref={toolbarRef}
          className="selection-toolbar"
          style={{ left: toolbarPos.x, top: toolbarPos.y, background: lightenColor(color,20), color: toolbarText }}
          onMouseMove={handleToolbarMouseMove}
          onMouseLeave={handleToolbarMouseLeave}
        >
          <button
            className="format-btn"
            style={{ transform: `scale(${scales[0]})` }}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => applyFormat("bold")}
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M6 4h8a4 4 0 0 1 0 8H6zm0 8h9a4 4 0 0 1 0 8H6z" />
            </svg>
          </button>
          <button
            className="format-btn"
            style={{ transform: `scale(${scales[1]})` }}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => applyFormat("italic")}
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M19 4h-9M5 20h9M15 4L9 20" />
            </svg>
          </button>
          <button
            className="format-btn"
            style={{ transform: `scale(${scales[2]})` }}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => applyFormat("underline")}
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M6 4v6a6 6 0 0 0 12 0V4M4 20h16" />
            </svg>
          </button>
        </div>
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <NoteApp />
  </React.StrictMode>
);