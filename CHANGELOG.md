# Changelog

All notable changes to this project are documented in this file.
Format: Keep a Changelog (https://keepachangelog.com), versioning: Semantic Versioning (https://semver.org).

## [Unreleased]

### Added
- Text input from mobile keyboards that sync whole values (Android, iOS): `EditorCore::ime_text` is the text they edit (the block with the caret, with the selection and the composed word), `Input::ImeSync` applies what they send back. Typing, Backspace (also at the start of a block, Word-style), autocorrect and word suggestions become ordinary edits

## [0.1.2] - 2026-11-19

### Added
- Read-only documents: `EditorCore::read_only` / `set_read_only` / `is_read_only`. The user can select, copy, scroll, zoom and switch the mode, but not change the document; a plain click follows a link, task checkboxes do not react and rendered blocks stay rendered
- `Command::edits` and `EditorCore::allows`, to tell which commands do anything with the current features and read-only state (e.g. to disable buttons)
- `Layout::content_height`: the height the content needs, without stretching to the viewport in full-width mode

## [0.1.1] - 2026-11-18

### Fixed
- Dragging a selection to the edge of the view now scrolls along

## [0.1.0] - 2026-11-18

### Added
- The UI-independent core of `iced-markdown-editor` and `frust-markdown-editor`: document model, Markdown import and export, layout and pagination, editing, selection, undo, source mode, tables and a display list

[unreleased]: https://github.com/iocast/markdown-editor-core/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/iocast/markdown-editor-core/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/iocast/markdown-editor-core/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/iocast/markdown-editor-core/releases/tag/v0.1.0
