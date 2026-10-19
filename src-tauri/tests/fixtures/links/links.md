# Images and links

Manual test page for #5. Open it in mdr: the three images below must show,
the red one must not (it lives outside this folder).

PNG: ![green](img/green.png)

GIF: <img src="./img/blue.gif" width="160" height="60" alt="blue">

SVG with a space in its name: ![orange](img/orange%20circle.svg)

Outside the document's folder (must stay broken): ![red](../outside/secret.png)

## Links

- [External link](https://example.com): opens in the browser
- [Mail](mailto:someone@example.com): opens the mail app
- [Another document](../lists.md): opens in a new mdr window
- [This section](#links): scrolls here
- [A non-Markdown file](img/green.png): does nothing
