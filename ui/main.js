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

async function render({ reload }) {
  let doc;
  try {
    doc = await invoke("get_document");
  } catch (error) {
    // A deleted or half-written file: keep showing the last version.
    if (!reload || !main.firstElementChild) showMessage(error);
    return;
  }
  if (!doc) {
    showMessage("Open a Markdown file (⌘O)");
    return;
  }

  const page = document.documentElement;
  const top = page.scrollTop;
  const atEnd = top > 0 && top + innerHeight >= page.scrollHeight - 2;

  document.title = doc.title;
  // Sanitized in Rust (render.rs).
  main.innerHTML = doc.html;

  // Edits elsewhere in the file don't move the view; at the end, follow it.
  if (reload) page.scrollTop = atEnd ? page.scrollHeight : top;
}

await render({ reload: false });

// Sent by documents.rs (restyle) when the config file changes: replaces the
// variables injected by the init script (config.rs).
window.__TAURI__.event.listen("config-changed", ({ payload }) => {
  document.getElementById("config").textContent = payload;
});

// Sent by watch.rs when the file changes on disk.
window.__TAURI__.webviewWindow
  .getCurrentWebviewWindow()
  .listen("document-changed", () => render({ reload: true }));
