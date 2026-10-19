const { invoke } = window.__TAURI__.core;

const main = document.querySelector("main");

function showMessage(text) {
  const message = document.createElement("p");
  message.className = "message";
  message.textContent = text;
  main.replaceChildren(message);
}

// Links never navigate the window: anchors scroll, everything else is
// handed to Rust (documents.rs, follow_link).
document.addEventListener("click", (event) => {
  const link = event.target.closest("a[href]");
  if (!link) return;
  event.preventDefault();
  const href = link.getAttribute("href");
  if (href.startsWith("#")) {
    document.getElementById(decodeURIComponent(href.slice(1)))?.scrollIntoView();
  } else {
    invoke("follow_link", { href });
  }
});

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
