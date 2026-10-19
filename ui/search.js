// In-page search (⌘F). Matches are painted with the CSS Custom Highlight API,
// so the document's DOM is never modified: selection and live reload keep
// working. Actions come from the native menu (menu.rs) as `find` events.
//
// Only the matches near the screen are painted: WebKit takes about a second
// to paint tens of thousands of highlighted ranges at once.
//
// Both highlights are registered once and only their ranges change: this
// WebKit doesn't repaint when a highlight is deleted from the registry or
// added again (WebKit bugs 306396 and 307491), which left matches painted
// after Esc.

const main = document.querySelector("main");
const nearby = new Highlight();
const selected = new Highlight();
CSS.highlights.set("search", nearby);
CSS.highlights.set("search-current", selected);

let bar = null;
let input = null;
let counter = null;
let matches = [];
let current = -1;
let typing;
let scrolling;

function open() {
  if (!bar) {
    bar = document.createElement("div");
    bar.className = "search";
    input = document.createElement("input");
    input.type = "search";
    input.placeholder = "Find";
    input.spellcheck = false;
    counter = document.createElement("span");
    counter.className = "search-count";
    bar.append(input, counter);
    document.body.append(bar);

    input.addEventListener("input", () => {
      clearTimeout(typing);
      typing = setTimeout(() => search({ fromView: true }), 60);
    });
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        step(event.shiftKey ? -1 : 1);
      }
    });
  }
  bar.hidden = false;
  input.focus();
  input.select();
}

function close() {
  if (!bar || bar.hidden) return;
  clearTimeout(typing);
  clearTimeout(scrolling);
  bar.hidden = true;
  matches = [];
  current = -1;
  nearby.clear();
  selected.clear();
}

function isOpen() {
  return bar && !bar.hidden;
}

// Finds every case-insensitive occurrence of the query in the document's
// text nodes. `fromView` starts from the first match on screen.
function search({ fromView }) {
  const query = input.value.toLowerCase();
  matches = [];
  if (query) {
    const walker = document.createTreeWalker(main, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const text = node.data.toLowerCase();
      for (let at = text.indexOf(query); at !== -1; at = text.indexOf(query, at + query.length)) {
        const range = new Range();
        range.setStart(node, at);
        range.setEnd(node, at + query.length);
        matches.push(range);
      }
    }
  }
  if (fromView) {
    current = matches.length ? firstBelow(0) % matches.length : -1;
  } else {
    current = matches.length ? Math.min(Math.max(current, 0), matches.length - 1) : -1;
  }
  show();
}

// Index of the first match whose bottom is below `y` (viewport coordinates),
// or `matches.length`. Matches are in document order, so their positions
// only grow.
function firstBelow(y) {
  let low = 0;
  let high = matches.length;
  while (low < high) {
    const middle = (low + high) >> 1;
    if (matches[middle].getBoundingClientRect().bottom < y) low = middle + 1;
    else high = middle;
  }
  return low;
}

// Paints the matches from one screen above to one screen below the view.
function paintNearby() {
  const end = Math.min(firstBelow(2 * innerHeight), matches.length);
  nearby.clear();
  for (let i = firstBelow(-innerHeight); i < end; i++) nearby.add(matches[i]);
}

function step(direction) {
  if (!matches.length) return;
  current = (current + direction + matches.length) % matches.length;
  show();
}

// Paints the current match, scrolls it into view and updates the counter.
function show() {
  counter.textContent = matches.length ? `${current + 1}/${matches.length}` : input.value ? "0/0" : "";
  selected.clear();
  if (current === -1) {
    paintNearby();
    return;
  }
  const range = matches[current];
  selected.add(range);

  // Keep clear of the title bar and the search bar at the top.
  const rect = range.getBoundingClientRect();
  const top = bar.getBoundingClientRect().bottom + 8;
  if (rect.top < top || rect.bottom > innerHeight - 8) {
    scrollBy(0, rect.top - innerHeight / 3);
  }
  paintNearby();
}

window.__TAURI__.webviewWindow
  .getCurrentWebviewWindow()
  .listen("find", ({ payload }) => {
    if (payload === "open") open();
    else if (!isOpen()) {
      open();
      search({ fromView: true });
    } else step(payload === "previous" ? -1 : 1);
  });

addEventListener("scroll", () => {
  if (!isOpen() || !matches.length) return;
  clearTimeout(scrolling);
  scrolling = setTimeout(paintNearby, 30);
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && isOpen()) {
    event.preventDefault();
    close();
  }
});

// Sent by main.js after each render, including live reloads: the old ranges
// point to nodes that no longer exist.
document.addEventListener("document-rendered", () => {
  if (isOpen() && input.value) search({ fromView: false });
});
