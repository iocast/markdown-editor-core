# markdown-editor-core

The UI-independent core of a Word-style WYSIWYG CommonMark editor. It powers
[`iced-markdown-editor`](https://github.com/iocast/iced-markdown-editor) and
[`frust-markdown-editor`](https://github.com/iocast/frust-markdown-editor).

The core holds the document model, Markdown import and export, layout and
pagination, editing, selection, undo, source mode and tables. It depends on no
UI framework and ships no fonts, icons or translations. Those belong to each
editor.

## Connecting a UI framework

1. Create an `EditorCore` with the `Fonts` the UI draws with.
2. Turn framework events into `Input`s and pass them to `EditorCore::update`.
3. Carry out the returned `Effect`s and report the `Event`s.
4. Replay `EditorCore::content_scene`, a list of `Op`s, onto the framework's
   canvas.

`Effect` and `Op` are exhaustive on purpose: a new variant is a compile error
in each editor rather than something silently ignored.

## Features

| Feature | Purpose |
|---|---|
| `syntect` | Ready-made syntax highlighter for code blocks |
| `images` | Decode images for display and paste them from the clipboard |

## Developing with the editors

To work on the core and an editor together, check them out side by side and add
this to the editor's gitignored `.cargo/config.toml`:

```toml
[patch.crates-io]
markdown-editor-core = { path = "../markdown-editor-core" }
```

## License

MIT or Apache-2.0, at your option.
