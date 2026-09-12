import React, { useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./note.css";

const PASTELS = ["#FFE8CC", "#FFD6E8", "#D6F5D6", "#D6E8FF", "#F0D6FF", "#FFF6C9", "#FFDDE1", "#E0FFF4"];

const HIGHLIGHT_MAP: Record<string, string> = {
  "#FFE8CC": "#f0c896",
  "#FFD6E8": "#e4abc3",
  "#D6F5D6": "#93d193",
  "#D6E8FF": "#9ec1e6",
  "#F0D6FF": "#cba8e0",
  "#FFF6C9": "#e8d98a",
  "#FFDDE1": "#eda3b0",
  "#E0FFF4": "#8fd4bd",
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

function getContrastText(hex: string) {
  const num = parseInt(hex.replace("#", ""), 16);
  const r = (num >> 16) & 0xff;
  const g = (num >> 8) & 0xff;
  const b = num & 0xff;
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance > 0.6 ? "#4a4a4a" : "#ffffff";
}

function makeListElement(id: string, kind: "todo" | "bullet") {
  const wrap = document.createElement("div");
  wrap.className = kind === "todo" ? "todo-item" : "bullet-item";
  wrap.id = id;
  wrap.contentEditable = "false";

  const marker = document.createElement("span");
  marker.className = kind === "todo" ? "todo-box" : "bullet-dot";

  const text = document.createElement("span");
  text.className = "todo-text";
  text.contentEditable = "true";

  wrap.appendChild(marker);
  wrap.appendChild(text);
  return wrap;
}

function focusListText(itemEl: HTMLElement, atEnd: boolean) {
  const textSpan = itemEl.classList.contains("todo-text") ? itemEl : itemEl.querySelector(".todo-text");
  if (!textSpan) return;
  const el = textSpan as HTMLElement;
  setTimeout(() => {
    el.focus();
    const range = document.createRange();
    range.selectNodeContents(el);
    range.collapse(!atEnd);
    const sel = window.getSelection();
    sel?.removeAllRanges();
    sel?.addRange(range);
  }, 0);
}

function placeCursorInPlainDiv(div: HTMLElement, atEnd: boolean) {
  setTimeout(() => {
    const range = document.createRange();
    range.selectNodeContents(div);
    range.collapse(!atEnd);
    const sel = window.getSelection();
    sel?.removeAllRanges();
    sel?.addRange(range);
  }, 0);
}

function revertItemToPlainLine(item: Element, atEnd: boolean) {
  const textSpan = item.querySelector(".todo-text");
  const div = document.createElement("div");
  div.innerHTML = textSpan && textSpan.innerHTML ? textSpan.innerHTML : "<br>";
  item.replaceWith(div);
  placeCursorInPlainDiv(div, atEnd);
  return div;
}

function getCurrentBlock(node: Node | null, editor: HTMLElement | null) {
  if (!node || !editor) return null;
  let el: Node | null = node;
  while (el && el.parentElement !== editor) {
    el = el.parentElement;
  }
  return el as HTMLElement | null;
}

function isCaretAtStart(el: HTMLElement, selection: Selection) {
  const range = selection.getRangeAt(0).cloneRange();
  range.selectNodeContents(el);
  range.setEnd(selection.anchorNode as Node, selection.anchorOffset);
  return range.toString().length === 0;
}

const initialColor = randomPastel();
const initialRotation = randomTilt();

document.documentElement.style.background = initialColor;
document.body.style.background = initialColor;

const BUTTON_BASE = 16;
const BUTTON_MAX = 24;
const MAGNIFY_RADIUS = 40;

let todoCounter = 0;

function NoteApp() {
  const thisWindow = getCurrentWindow();
  const noteId = thisWindow.label;

  const [color, setColor] = useState(initialColor);
  const [rotation] = useState(initialRotation);
  const [loaded, setLoaded] = useState(false);
  const [toolbarPos, setToolbarPos] = useState<{ x: number; y: number } | null>(null);
  const [scales, setScales] = useState([1, 1, 1, 1]);
  const editorRef = useRef<HTMLDivElement>(null);
  const toolbarRef = useRef<HTMLDivElement>(null);
  const htmlRef = useRef("");
  const saveTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  const historyRef = useRef<string[]>([]);
  const historyIndexRef = useRef(-1);
  const skipHistoryRef = useRef(false);
  const historyTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<{ id: string; text: string; color: string; rotation: number }[]>("get_notes").then((notes) => {
      const existing = notes.find((n) => n.id === noteId);
      if (existing) {
        htmlRef.current = existing.text;
        setColor(existing.color);
        document.documentElement.style.background = existing.color;
        document.body.style.background = existing.color;
        if (editorRef.current) {
          editorRef.current.innerHTML = existing.text;
          rebindImageHandles();
        }
      } else {
        invoke("save_note", { id: noteId, text: "", color: initialColor, rotation: initialRotation });
      }
      setLoaded(true);
      invoke("show_note_window", { id: noteId });
      setTimeout(() => {
        if (editorRef.current) {
          historyRef.current = [editorRef.current.innerHTML];
          historyIndexRef.current = 0;
        }
      }, 0);
    });
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => {
      editorRef.current?.focus();
    }, 100);
    return () => clearTimeout(timer);
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

  useEffect(() => {
    function handleTodoClick(e: MouseEvent) {
      const target = e.target as HTMLElement;
      if (target.classList.contains("todo-box")) {
        const item = target.closest(".todo-item");
        if (item) {
          item.classList.toggle("checked");
          handleInput();
        }
      }
    }
    const el = editorRef.current;
    el?.addEventListener("click", handleTodoClick);
    return () => el?.removeEventListener("click", handleTodoClick);
  }, []);

  function rebindImageHandles() {
    editorRef.current?.querySelectorAll(".img-resize-handle").forEach((handle) => {
      const wrap = handle.parentElement as HTMLElement;
      handle.addEventListener("mousedown", (e) => startResize(e as MouseEvent, wrap));
    });
  }

  function scheduleSave(value: string) {
    htmlRef.current = value;
    if (saveTimeout.current) clearTimeout(saveTimeout.current);
    saveTimeout.current = setTimeout(() => {
      invoke("save_note", { id: noteId, text: value, color, rotation });
    }, 300);
  }

  function pushHistory() {
    if (!editorRef.current) return;
    const html = editorRef.current.innerHTML;
    historyRef.current = historyRef.current.slice(0, historyIndexRef.current + 1);
    historyRef.current.push(html);
    if (historyRef.current.length > 100) {
      historyRef.current.shift();
    }
    historyIndexRef.current = historyRef.current.length - 1;
  }

  function handleInput() {
    const value = editorRef.current?.innerHTML || "";
    scheduleSave(value);

    if (skipHistoryRef.current) {
      skipHistoryRef.current = false;
      return;
    }

    if (historyTimeout.current) clearTimeout(historyTimeout.current);
    historyTimeout.current = setTimeout(() => {
      pushHistory();
    }, 400);
  }

  function undo() {
    if (historyIndexRef.current <= 0) return;
    historyIndexRef.current -= 1;
    const html = historyRef.current[historyIndexRef.current];
    if (editorRef.current && html !== undefined) {
      skipHistoryRef.current = true;
      editorRef.current.innerHTML = html;
      handleInput();
      rebindImageHandles();
    }
  }

  function redo() {
    if (historyIndexRef.current >= historyRef.current.length - 1) return;
    historyIndexRef.current += 1;
    const html = historyRef.current[historyIndexRef.current];
    if (editorRef.current && html !== undefined) {
      skipHistoryRef.current = true;
      editorRef.current.innerHTML = html;
      handleInput();
      rebindImageHandles();
    }
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
    setScales([1, 1, 1, 1]);
  }

  function applyFormat(command: string) {
    editorRef.current?.focus();
    document.execCommand(command);
    handleInput();
    updateToolbarPosition();
  }


  function insertResizableImage(src: string) {
    const wrap = document.createElement("div");
    wrap.className = "img-wrap";
    wrap.contentEditable = "false";
    wrap.style.width = "220px";

    const img = document.createElement("img");
    img.src = src;
    img.className = "pasted-img";

    const handle = document.createElement("div");
    handle.className = "img-resize-handle";

    wrap.appendChild(img);
    wrap.appendChild(handle);

    const selection = window.getSelection();
    if (selection && selection.rangeCount > 0 && editorRef.current?.contains(selection.anchorNode)) {
      const range = selection.getRangeAt(0);
      range.deleteContents();
      range.insertNode(wrap);
    } else {
      editorRef.current?.appendChild(wrap);
    }

    const afterBreak = document.createElement("div");
    afterBreak.innerHTML = "<br>";
    wrap.after(afterBreak);

    handle.addEventListener("mousedown", (e) => startResize(e as MouseEvent, wrap));
    handleInput();
  }

  function startResize(e: MouseEvent, wrap: HTMLElement) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = wrap.offsetWidth;

    function onMove(moveEvent: MouseEvent) {
      const newWidth = startWidth + (moveEvent.clientX - startX);
      if (newWidth > 60) {
        wrap.style.width = `${newWidth}px`;
      }
    }

    function onUp() {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      handleInput();
    }

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function handleImagePaste(e: React.ClipboardEvent) {
    const items = e.clipboardData?.items;
    if (!items) return;

    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.type.startsWith("image/")) {
        e.preventDefault();
        const file = item.getAsFile();
        if (!file) continue;

        const reader = new FileReader();
        reader.onload = () => {
          const dataUrl = reader.result as string;
          insertResizableImage(dataUrl);
        };
        reader.readAsDataURL(file);
        return;
      }
    }
  }

  function handleSpaceConvert(e: React.KeyboardEvent) {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0 || !selection.isCollapsed) return;
    const node = selection.anchorNode;
    if (!node) return;

    const insideItem = (node instanceof Element ? node : node.parentElement)?.closest(".todo-item, .bullet-item");
    if (insideItem) return;

    const block = getCurrentBlock(node, editorRef.current);
    if (!block) return;

    const range = selection.getRangeAt(0).cloneRange();
    range.selectNodeContents(block);
    range.setEnd(node, selection.anchorOffset);
    const textBefore = range.toString();

    if (textBefore === "--") {
      e.preventDefault();
      todoCounter += 1;
      const id = `todo-${Date.now()}-${todoCounter}`;
      const item = makeListElement(id, "todo");
      block.replaceWith(item);
      focusListText(item, false);
      handleInput();
      return;
    }

    if (textBefore === "-") {
      e.preventDefault();
      todoCounter += 1;
      const id = `bullet-${Date.now()}-${todoCounter}`;
      const item = makeListElement(id, "bullet");
      block.replaceWith(item);
      focusListText(item, false);
      handleInput();
    }
  }

  function handleEnterKey(e: React.KeyboardEvent) {
    const target = e.target as HTMLElement;
    if (!target.classList.contains("todo-text")) return;
    const item = target.closest(".todo-item, .bullet-item");
    if (!item) return;

    e.preventDefault();
    const isEmpty = target.textContent?.trim() === "";

    if (isEmpty) {
      revertItemToPlainLine(item, false);
      handleInput();
      return;
    }

    const kind = item.classList.contains("todo-item") ? "todo" : "bullet";
    todoCounter += 1;
    const id = `${kind}-${Date.now()}-${todoCounter}`;
    const newItem = makeListElement(id, kind);
    item.after(newItem);
    focusListText(newItem, false);
    handleInput();
  }

  function handleBackspaceKey(e: React.KeyboardEvent) {
    const target = e.target as HTMLElement;
    const selection = window.getSelection();
    if (!selection || !selection.isCollapsed) return;

    if (target.classList.contains("todo-text")) {
      const item = target.closest(".todo-item, .bullet-item");
      if (!item) return;
      if (!isCaretAtStart(target, selection)) return;
      e.preventDefault();
      revertItemToPlainLine(item, false);
      handleInput();
      return;
    }

    const block = getCurrentBlock(selection.anchorNode, editorRef.current);
    if (!block) return;
    if (!isCaretAtStart(block, selection)) return;

    const prev = block.previousElementSibling;
    if (!prev) return;

    if (prev.classList.contains("todo-item") || prev.classList.contains("bullet-item")) {
      e.preventDefault();
      const textSpan = prev.querySelector(".todo-text") as HTMLElement | null;
      if (!textSpan) return;
      const leftoverHTML = block.innerHTML === "<br>" ? "" : block.innerHTML;
      textSpan.innerHTML = textSpan.innerHTML + leftoverHTML;
      block.remove();
      focusListText(prev as HTMLElement, true);
      handleInput();
    }
  }

  function handleEditorKeyDown(e: React.KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      if (!editorRef.current) return;
      const range = document.createRange();
      range.selectNodeContents(editorRef.current);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(range);
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z" && !e.shiftKey) {
      e.preventDefault();
      undo();
      return;
    }

    if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === "y" || (e.key.toLowerCase() === "z" && e.shiftKey))) {
      e.preventDefault();
      redo();
      return;
    }

    if (e.altKey && e.key.toLowerCase() === "w") {
      e.preventDefault();
      handleClose();
      return;
    }

    if (e.key === "Backspace") {
      const selection = window.getSelection();
      if (selection && !selection.isCollapsed && editorRef.current) {
        const full = document.createRange();
        full.selectNodeContents(editorRef.current);
        const range = selection.getRangeAt(0);
        const wholeEditorSelected =
          range.compareBoundaryPoints(Range.START_TO_START, full) === 0 &&
          range.compareBoundaryPoints(Range.END_TO_END, full) === 0;
        if (wholeEditorSelected) {
          e.preventDefault();
          editorRef.current.innerHTML = "<div><br></div>";
          placeCursorInPlainDiv(editorRef.current.firstElementChild as HTMLElement, false);
          handleInput();
          return;
        }
      }
    }

    if (e.key === " ") {
      handleSpaceConvert(e);
      return;
    }
    if (e.key === "Enter") {
      handleEnterKey(e);
      return;
    }
    if (e.key === "Backspace") {
      handleBackspaceKey(e);
    }
  }

  function handleRightClick(e: React.MouseEvent) {
    e.preventDefault();
    editorRef.current?.focus();
    setToolbarPos({ x: e.clientX, y: e.clientY });
    setScales([1, 1, 1, 1]);
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
    setScales([1, 1, 1, 1]);
  }

  async function handleClose() {
    if (saveTimeout.current) clearTimeout(saveTimeout.current);
    await invoke("save_note", { id: noteId, text: htmlRef.current, color: color, rotation: rotation });
    await thisWindow.close();
  }

  function handleMenu() {
    invoke("show_main_window");
  }


  const toolbarText = getContrastText(darkenColor(color, 35));

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
        onKeyDown={handleEditorKeyDown}
        onContextMenu={handleRightClick}
        onPaste={handleImagePaste}
        spellCheck={false}
        onBlur={() => setTimeout(() => setToolbarPos(null), 150)}
        data-placeholder="type your note..."
      />

      {toolbarPos && (
        <div
          ref={toolbarRef}
          className="selection-toolbar"
          style={{ left: toolbarPos.x, top: toolbarPos.y, background: darkenColor(color, 35), color: toolbarText }}
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