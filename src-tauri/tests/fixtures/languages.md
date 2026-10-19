# Languages

```rust
/// Renders a document.
pub fn render(markdown: &str) -> Result<String, Error> {
    let count = markdown.lines().count(); // a comment
    Ok(format!("{count} lines"))
}
```

```ts
interface Doc { title: string; lines: number }
export const open = async (path: string): Promise<Doc> => ({ title: path, lines: 42 });
```

```python
@dataclass
class Doc:
    title: str = "untitled"  # default
    def lines(self) -> int:
        return len(self.title.splitlines())
```

```sh
#!/bin/sh
for f in *.md; do echo "Opening $f"; open -a mdr "$f"; done
```

```json
{ "name": "mdr", "version": "0.1.0", "private": true, "tags": ["markdown", null] }
```

```yaml
name: CI
on: [push]
jobs:
  check: { runs-on: macos-latest }  # comment
```

```toml
[themes.mine]
background = "#f7f3ea"
size = 16
enabled = true
```

```diff
@@ -1,3 +1,3 @@
 unchanged line
-removed line
+added line
```

```made-up-language
plain text, no highlighting
```
