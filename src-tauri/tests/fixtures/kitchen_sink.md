# Kitchen sink

A paragraph of ordinary prose, long enough to wrap over several lines so that the measure, the line height and the rhythm between blocks can be judged. It has **bold**, *italic*, ~~strikethrough~~, `inline code`, a [link](https://example.com), <kbd>⌘</kbd> <kbd>F</kbd>, and a footnote.[^1]

A very long URL that must wrap instead of widening the page: https://example.com/a/very/long/path/that/keeps/going/and/going/without/any/space/to/break/on/at/all/index.html

## Lists

- First item
- Second item with a longer text that wraps onto a second line when the window is narrow enough
  - Nested item
    - Deeper item
- Third item

1. Step one
2. Step two
   1. Sub-step
3. Step three

- [x] Task done
- [ ] Task to do
  - [ ] Nested task

## Quote

> Simplicity is prerequisite for reliability.
>
> > A nested quote, a little quieter.

## Code

```rust
fn main() {
    let document = std::fs::read_to_string("README.md").expect("readable file");
    println!("{}", mdr_lib::render::to_html(&document)); // a long line that has to scroll horizontally inside the block
}
```

    An indented code block.

## Table

| Command | Shortcut | Notes |
|:--------|:--------:|------:|
| Open | ⌘O | Native dialog |
| Find | ⌘F | Highlights every match |
| Reload | ⌘R | Rarely needed, files reload by themselves |

| a | very | wide | table | with | many | columns | that | must | scroll | on | its | own | instead | of | the | page |
|---|------|------|-------|------|------|---------|------|------|--------|----|-----|-----|---------|----|-----|------|
| 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 |

## Details

<details>
<summary>More information</summary>

Hidden content with a [link](#kitchen-sink).

</details>

---

### Heading 3

#### Heading 4

##### Heading 5

###### Heading 6

Last paragraph.

[^1]: A footnote, set smaller and quieter at the end of the document.
