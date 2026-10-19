const { invoke } = window.__TAURI__.core;

const main = document.querySelector("main");

function showMessage(text) {
  const message = document.createElement("p");
  message.className = "message";
  message.textContent = text;
  main.replaceChildren(message);
}

try {
  const doc = await invoke("get_document");
  if (doc) {
    document.title = doc.title;
    // Sanitized in Rust (render.rs).
    main.innerHTML = doc.html;
  } else {
    showMessage("Open a Markdown file (⌘O)");
  }
} catch (error) {
  showMessage(error);
}
