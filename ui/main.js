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

// Inserts what fills the first two screens, then the rest once they're on
// screen, two frames later: WebKit takes ~300 ms to lay out a 10,000-line
// page, which would delay the first frame. The rest goes below the view, so
// nothing moves.
async function insertFirstScreens(html) {
  const template = document.createElement("template");
  template.innerHTML = html;
  const blocks = template.content;
  main.replaceChildren();
  while (blocks.firstChild && main.offsetHeight < 2 * innerHeight) {
    for (let i = 0; i < 20 && blocks.firstChild; i++) main.append(blocks.firstChild);
  }
  if (!blocks.firstChild) return;
  const shown = main.lastChild;
  await new Promise((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(resolve))),
  );
  // Unless a live reload replaced the page in the meantime.
  if (shown.isConnected) main.append(blocks);
}

function rendered() {
  document.dispatchEvent(new Event("document-rendered"));
}

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
  if (reload) {
    main.innerHTML = doc.html;
    // Edits elsewhere in the file don't move the view; at the end, follow it.
    page.scrollTop = atEnd ? page.scrollHeight : top;
    rendered();
  } else {
    insertFirstScreens(doc.html).then(rendered);
  }
}

await render({ reload: false });

// `MDR_BENCH` (scripts/bench.sh): report the first frame painted with the
// document, two frames after it's inserted.
if (window.__MDR_BENCH__) {
  requestAnimationFrame(() => requestAnimationFrame(() => invoke("bench_painted")));
}

// Sent by documents.rs (restyle) when the config file changes: replaces the
// variables injected by the init script (config.rs).
window.__TAURI__.event.listen("config-changed", ({ payload }) => {
  document.getElementById("config").textContent = payload;
});

// Sent by watch.rs when the file changes on disk.
window.__TAURI__.webviewWindow
  .getCurrentWebviewWindow()
  .listen("document-changed", () => render({ reload: true }));
