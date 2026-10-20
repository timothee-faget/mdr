# Performance

mdr aims to feel instant and stay light. This page records how it's measured, the latest numbers, and where the time and memory go.

## Targets

| | Target | Measured | |
|---|---|---|---|
| Startup, typical document | < 400 ms | 385 ms | ✅ |
| Startup, 10,000 lines | < 600 ms | 408 ms | ✅ |
| Memory, typical document | < 120 MB | 93 MB | ✅ |
| `.app` size | < 15 MB | 6.2 MB | ✅ |

The first targets were 200 ms and 80 MB. The measurements below show that macOS, Tauri and WebKit alone take ~350 ms and ~65 MB before mdr renders anything, so the targets were revised on 2026-10-19 to what a system WebView allows, then reached with [#42](https://github.com/timothee-faget/mdr/issues/42).

The typical document is close to its limit: from one series of launches to the next, its median moves between 385 and 416 ms, with no change to mdr.

## Method

```sh
just bench        # builds a release .app, then runs scripts/bench.sh 10
just bench 5      # fewer launches
```

`scripts/bench.sh` opens real mdr windows, so don't use the Mac while it runs (about 3 minutes).

- **Startup**: from `open -a mdr.app <file>`, what a double-click in the Finder does, to the first frame painted with the document. With `MDR_BENCH` set, the page reports that frame (two `requestAnimationFrame` after inserting the document) and mdr prints `mdr: painted` on stderr (`src-tauri/src/bench.rs`). Without the variable, none of this runs. Each document is launched 10 times; the first launch is reported apart because it pays for cold caches.
- **Memory**: `phys_footprint` from `footprint`, which is the "Memory" column of Activity Monitor. It's summed over mdr and the WebKit processes it spawned (WebContent, GPU, Networking), 3 s after the paint.
- **Documents**, copied to a temporary folder outside `~/Documents` (no privacy prompt), with an empty config:
  - *empty*: no file, the empty window;
  - *readme*: this repository's `README.md` (~60 lines);
  - *long*: `kitchen_sink.md` and `languages.md` repeated to 10,010 lines, including ~180 highlighted code blocks. Each copy gets its own footnote label: comrak moves everything between two definitions of the same label into the footnotes, which made the page one huge list item until #42.

## Results

mdr 1.0.0 with #42, macOS 26.5.2, Apple M4 Pro, 2026-10-20.

| Document | First launch | Median | Min | Max | Memory |
|---|---|---|---|---|---|
| empty | 358 ms | 353 ms | 340 ms | 741 ms | 107 MB |
| readme | 384 ms | 385 ms | 369 ms | 403 ms | 93 MB |
| long | 414 ms | 408 ms | 386 ms | 426 ms | 310 MB |

App size: 6.2 MB (`.app`), 6.0 MB (binary).

Before #42 (1.0.0), in the same session: empty 349 ms, readme 388 ms, long 830 ms (medians).

## Where the time goes

Timestamps from a temporarily instrumented build, in ms after `open`, warm caches:

| Step | readme | long |
|---|---|---|
| macOS starts the process (`open` → `main`) | 85 | 85 |
| Tauri builds the app (event loop, plugins, menu) | 65 | 65 |
| AppKit finishes launching (→ `Ready`) | 45 | 45 |
| Window and WKWebView created | 65 | 65 |
| WebKit starts the page | 45 | 45 |
| Page loaded, scripts waiting for the main thread | 50 | 50 |
| Document received and first screens inserted | 8 | 45 |
| Two frames to the paint | 32 | 18 |
| **Total** | **~395** | **~420** |

- About 350 ms are spent before the page's script runs, whatever the document. They're shared by macOS, Tauri, WebKit and the window manager: the page's scripts wait for the app's main thread, which is busy until the window is placed (here by AeroSpace, a tiling window manager). mdr's own setup takes 7 ms of it: the config (1 ms) and the menu (6 ms).
- The document is rendered in Rust (comrak, ammonia, syntect: 3 ms for the README, 130 ms for 10,000 lines) while the window starts, so it's ready when the page asks for it.
- The page inserts what fills its first two screens, then the rest once they're painted. A long document is then laid out by WebKit without delaying the first frame. On 10,000 lines, the 45 ms are spent sending the 2.3 MB of HTML (20 ms), parsing it (17 ms) and inserting the first screens.

## Where the memory goes

| Process | empty | readme | long |
|---|---|---|---|
| mdr | 27 MB | 29 MB | 72 MB |
| WebKit WebContent | 56 MB | 38 MB | 181 MB |
| WebKit GPU | 21 MB | 19 MB | 23 MB |
| WebKit Networking | 5 MB | 6 MB | 6 MB |

Measured on 0.2.0, with the old *long* document (281 MB in total, 310 MB now).

The WebKit processes alone take about 65 MB for the smallest page. That's the cost of the system WebView, so 80 MB in total isn't reachable while mdr renders through it. On long documents, WebContent grows with the DOM (every highlighted token is a `<span>`), and mdr keeps syntect's grammars loaded after the first code block.
