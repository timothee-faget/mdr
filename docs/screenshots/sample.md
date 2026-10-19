# Field notes

A quiet place to **read Markdown**. Open a file and mdr shows it, nothing else: no toolbar, no sidebar, just the text in your colors.

## What's in a page

- Headings, *emphasis*, `inline code` and [links](https://github.com/timothee-faget/mdr)
- [x] Task lists
- [ ] Tables, quotes and footnotes[^1]

```rust
fn main() {
    let doc = std::fs::read_to_string("notes.md")?;
    println!("{} lines", doc.lines().count());
}
```

| Shortcut | Action |
|---|---|
| ⌘O | Open a file |
| ⌘F | Find in the page |
| ⌘, | Settings |

> The file reloads by itself when you save it in your editor.

[^1]: Rendered by comrak, highlighted by syntect.
