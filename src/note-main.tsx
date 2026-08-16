import React from "react";
import ReactDOM from "react-dom/client";

function NoteApp() {
  return (
    <div style={{
      background: "#fff176",
      width: "100vw",
      height: "100vh",
      boxSizing: "border-box",
      padding: 10,
    }}>
      <textarea
        placeholder="type your note..."
        style={{
          width: "100%",
          height: "100%",
          border: "none",
          background: "transparent",
          resize: "none",
          outline: "none",
          fontSize: 16,
          fontFamily: "sans-serif",
        }}
      />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <NoteApp />
  </React.StrictMode>
);