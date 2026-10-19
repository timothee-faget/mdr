# Performance

mdr aims to feel instant and stay light. This page records how it's measured, the latest numbers, and where the time and memory go.

## Targets

| | Target | Measured (0.2.0) | |
|---|---|---|---|
| Startup, typical document | < 400 ms | 423 ms | ❌ |
| Startup, 10,000 lines | < 600 ms | 847 ms | ❌ |
| Memory, typical document | < 120 MB | 96 MB | ✅ |
| `.app` size | < 15 MB | 6.2 MB | ✅ |

The first targets were 200 ms and 80 MB. The measurements below show that macOS, Tauri and WebKit alone take ~365 ms and ~65 MB before mdr renders anything, so the targets were revised on 2026-10-19 to what a system WebView allows. Optimizations for the missed ones: [#42](https://github.com/timothee-faget/mdr/issues/42).

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
  - *long*: `kitchen_sink.md` and `languages.md` repeated to 10,010 lines, including ~180 highlighted code blocks.

## Results

mdr 0.2.0, macOS 26.5.2, Apple M4 Pro, 2026-10-19.

| Document | First launch | Median | Min | Max | Memory |
|---|---|---|---|---|---|
| empty | 697 ms | 382 ms | 365 ms | 402 ms | 108 MB |
| readme | 402 ms | 423 ms | 391 ms | 468 ms | 96 MB |
| long | 861 ms | 847 ms | 812 ms | 867 ms | 281 MB |

App size: 6.2 MB (`.app`), 6.0 MB (binary).

## Where the time goes

Timestamps from a temporarily instrumented build, in ms after `open`, warm caches:

| Step | readme | long |
|---|---|---|
| macOS starts the process (`open` → `main`) | 85 | 85 |
| Tauri setup (`main` → `Ready`: menu, plugins, event loop) | 110 | 110 |
| Window and WKWebView created | 70 | 70 |
| WebKit page starts | 45 | 45 |
| HTML, CSS and JS loaded, script running | 55 | 55 |
| Rendering in Rust (`get_document`: comrak, ammonia, syntect) | 3 | 117 |
| HTML sent to the page and inserted | 0 | 35 |
| Layout and first paint | 34 | 315 |
| **Total** | **~400** | **~840** |

- About 365 ms are spent before mdr has rendered anything, whatever the document. They're shared by macOS, Tauri and WebKit. The largest step mdr may control is `main` → `Ready` (110 ms).
- On long documents, WebKit's layout of the whole page dominates (315 ms), followed by syntax highlighting (part of the 117 ms in Rust).

## Where the memory goes

| Process | empty | readme | long |
|---|---|---|---|
| mdr | 27 MB | 29 MB | 72 MB |
| WebKit WebContent | 56 MB | 38 MB | 181 MB |
| WebKit GPU | 21 MB | 19 MB | 23 MB |
| WebKit Networking | 5 MB | 6 MB | 6 MB |

The WebKit processes alone take about 65 MB for the smallest page. That's the cost of the system WebView, so 80 MB in total isn't reachable while mdr renders through it. On long documents, WebContent grows with the DOM (every highlighted token is a `<span>`), and mdr keeps syntect's grammars loaded after the first code block.
