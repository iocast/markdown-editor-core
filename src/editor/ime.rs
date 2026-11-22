//! Text input from on-screen keyboards that sync whole values.
//!
//! Mobile input methods (Android, iOS) keep their own copy of the text
//! around the caret, edit that copy (typed letters, backspace, autocorrect,
//! word suggestions) and send the whole new state. [`EditorCore::ime_text`]
//! is the text they are given; [`Input::ImeSync`](super::Input::ImeSync)
//! applies what they send back.
//!
//! The text is the block with the caret, between two line breaks: deleting
//! the first one is Backspace at the start of the block (which merges it
//! with the previous one, or removes a list bullet), deleting the last one
//! is Delete at its end.

use super::{EditKind, EditorCore, Task};
use crate::doc::{Pos, Selection};

/// The text an input method edits: the block with the caret, with a line
/// break before and after it. Offsets count `char`s.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImeText {
    pub text: String,
    /// Selection: anchor and head. Equal for a caret.
    pub selection: (usize, usize),
    /// The text the input method is still composing (start, end).
    pub composing: Option<(usize, usize)>,
}

/// Characters before the block in [`ImeText::text`].
const LEAD: usize = 1;

impl EditorCore {
    /// The text an input method works on, with the selection and the
    /// composed text. A selection that leaves the block is shown as a caret
    /// at its head.
    pub fn ime_text(&self) -> ImeText {
        let head = self.sel.head;
        let Some(block) = self.doc.blocks.get(head.block) else {
            return ImeText::default();
        };
        let mut text = String::with_capacity(block.len() + 2);
        text.push('\n');
        text.extend(block.chars.iter());
        text.push('\n');

        let window = |pos: Pos| LEAD + pos.offset.min(block.len());
        let anchor = if self.sel.anchor.block == head.block {
            window(self.sel.anchor)
        } else {
            window(head)
        };
        let composing = self
            .preedit
            .filter(|(start, end)| {
                start.block == head.block && end.block == head.block && start < end
            })
            .map(|(start, end)| (window(start), window(end)));
        ImeText {
            text,
            selection: (anchor, window(head)),
            composing,
        }
    }

    /// Applies the input method's new state: the difference to
    /// [`EditorCore::ime_text`] becomes typing, deleting, Backspace or
    /// Delete; then the selection and the composed text are taken over.
    pub(crate) fn ime_sync(&mut self, new: ImeText) -> Task {
        if self.link_url.is_some() || !self.focused {
            self.preedit = None;
            return Task::none();
        }
        let old = self.ime_text();
        let old_chars: Vec<char> = old.text.chars().collect();
        let new_chars: Vec<char> = new.text.chars().collect();
        let block = self.sel.head.block;
        let len = self.block_len(block);

        let mut task = Task::none();
        let changed = old_chars != new_chars;
        if changed {
            let (old_lo, _) = ordered(old.selection);
            let (new_lo, _) = ordered(new.selection);
            // Edits happen at the caret: with repeated letters ("aa" to
            // "aaa") the insertion is placed there, not at the end.
            let prefix = common_prefix(&old_chars, &new_chars)
                .min(old_lo)
                .min(new_lo);
            let suffix = common_suffix(&old_chars[prefix..], &new_chars[prefix..]);
            let removed = (prefix, old_chars.len() - suffix);
            let inserted: String = new_chars[prefix..new_chars.len() - suffix].iter().collect();
            self.preedit = None;

            if self.sel.anchor.block != block {
                // The selection leaves the block: an edit replaces it, as a
                // typed letter or Backspace would.
                task = if inserted.is_empty() {
                    self.backspace(false)
                } else {
                    self.commit(&inserted)
                };
            } else {
                task = self.ime_replace(block, len, removed, &inserted);
            }
        }

        // The selection and the composed text, if the document now reads
        // as the input method expects (an autoformat may have changed it).
        let now = self.ime_text();
        let head_block = self.sel.head.block;
        let head_len = self.block_len(head_block);
        let to_pos =
            |offset: usize| Pos::new(head_block, offset.saturating_sub(LEAD).min(head_len));
        if now.text == new.text {
            let selection = Selection {
                anchor: to_pos(new.selection.0),
                head: to_pos(new.selection.1),
            };
            if selection != self.sel {
                self.sel = selection;
                self.goal_x = None;
                if !changed {
                    self.pending = None;
                    self.last_edit = EditKind::None;
                    self.touch();
                    task = self.scroll_to_caret();
                }
            }
            self.preedit = new
                .composing
                .map(|(start, end)| (to_pos(start), to_pos(end)))
                .filter(|(start, end)| start < end);
        } else {
            self.preedit = None;
        }
        task
    }

    /// Replaces `removed` (offsets in [`EditorCore::ime_text`]) in `block`
    /// (`len` characters) with `inserted`.
    fn ime_replace(
        &mut self,
        block: usize,
        len: usize,
        removed: (usize, usize),
        inserted: &str,
    ) -> Task {
        let (start, end) = removed;
        let inside = |offset: usize| offset.saturating_sub(LEAD).min(len);
        let before = start < LEAD;
        let after = end > LEAD + len;
        let (a, b) = (Pos::new(block, inside(start)), Pos::new(block, inside(end)));

        let mut task = Task::none();
        if a < b {
            self.sel = Selection { anchor: a, head: b };
            if !inserted.is_empty() && !before && !after {
                // Replaced text (autocorrect, a suggestion): one edit.
                return self.commit(inserted);
            }
            task = self.backspace(false);
        } else {
            self.sel = Selection::caret(a);
        }
        if before {
            // The line break before the block: Backspace at its start.
            self.sel = Selection::caret(Pos::new(self.sel.head.block, 0));
            task = self.backspace(false);
        }
        if after {
            // The line break after the block: Delete at its end.
            let head = self.sel.head.block;
            self.sel = Selection::caret(Pos::new(head, self.block_len(head)));
            task = self.delete_forward(false);
        }
        if !inserted.is_empty() {
            task = self.commit(inserted);
        }
        task
    }
}

fn ordered((a, b): (usize, usize)) -> (usize, usize) {
    (a.min(b), a.max(b))
}

fn common_prefix(a: &[char], b: &[char]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

fn common_suffix(a: &[char], b: &[char]) -> usize {
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{Command, Input};
    use crate::keyboard::{Key, Modifiers, Named};

    /// A focused editor with the caret at `pos`.
    fn editor(markdown: &str, pos: Pos) -> EditorCore {
        let mut editor = EditorCore::with_markdown(markdown);
        editor.update(Input::Command(Command::Focus));
        editor.sel = Selection::caret(pos);
        editor
    }

    /// What a keyboard sends: the text with `|` as the caret and `[` `]`
    /// around the composed text.
    fn state(marked: &str) -> ImeText {
        let mut text = String::new();
        let mut caret = 0;
        let mut composing = (None, None);
        for c in marked.chars() {
            match c {
                '|' => caret = text.chars().count(),
                '[' => composing.0 = Some(text.chars().count()),
                ']' => composing.1 = Some(text.chars().count()),
                c => text.push(c),
            }
        }
        ImeText {
            text,
            selection: (caret, caret),
            composing: composing.0.zip(composing.1),
        }
    }

    fn sync(editor: &mut EditorCore, marked: &str) {
        editor.update(Input::ImeSync(state(marked)));
    }

    #[test]
    fn the_keyboard_sees_the_block_around_the_caret() {
        let editor = editor("first\n\nsecond\n", Pos::new(1, 3));
        assert_eq!(editor.ime_text(), state("\nsec|ond\n"));
    }

    #[test]
    fn typed_letters_and_backspace() {
        let mut editor = editor("hello\n", Pos::new(0, 5));
        sync(&mut editor, "\nhellox|\n");
        assert_eq!(editor.markdown(), "hellox\n");
        assert_eq!(editor.sel, Selection::caret(Pos::new(0, 6)));
        sync(&mut editor, "\nhello|\n");
        assert_eq!(editor.markdown(), "hello\n");
        // Repeated letters: inserted at the caret.
        let mut editor = editor_at("aa\n", 1);
        sync(&mut editor, "\naa|a\n");
        assert_eq!(editor.sel, Selection::caret(Pos::new(0, 2)));
    }

    fn editor_at(markdown: &str, offset: usize) -> EditorCore {
        editor(markdown, Pos::new(0, offset))
    }

    #[test]
    fn composed_words_and_autocorrect() {
        let mut editor = editor_at(
            "say
", 3,
        );
        sync(
            &mut editor,
            "
say [h]|
",
        );
        sync(
            &mut editor,
            "
say [hel]|
",
        );
        assert_eq!(
            editor.markdown(),
            "say hel
"
        );
        assert_eq!(
            editor.ime_text(),
            state(
                "
say [hel]|
"
            )
        );
        // Autocorrect replaces the word and adds a space.
        sync(
            &mut editor,
            "
say hello |
",
        );
        assert_eq!(editor.doc.blocks[0].text(), "say hello ");
        assert_eq!(
            editor.ime_text(),
            state(
                "
say hello |
"
            )
        );
        assert!(editor.preedit.is_none());
        // Typing is one undo step.
        editor.update(Input::Command(Command::Undo));
        assert_eq!(
            editor.markdown(),
            "say
"
        );
    }

    #[test]
    fn styles_continue_while_typing() {
        let mut editor = editor_at("**bold**\n", 4);
        sync(&mut editor, "\nbolder|\n");
        assert_eq!(editor.markdown(), "**bolder**\n");
    }

    #[test]
    fn backspace_at_the_start_of_a_block_merges_it() {
        let mut editor = editor("one\n\ntwo\n", Pos::new(1, 0));
        sync(&mut editor, "|two\n");
        assert_eq!(editor.markdown(), "onetwo\n");
        assert_eq!(editor.sel, Selection::caret(Pos::new(0, 3)));
        // The keyboard gets the merged block.
        assert_eq!(editor.ime_text(), state("\none|two\n"));
    }

    #[test]
    fn backspace_at_the_start_of_a_list_item_removes_the_bullet() {
        let mut editor = editor("- item\n", Pos::new(0, 0));
        sync(&mut editor, "|item\n");
        assert_eq!(editor.markdown(), "item\n");
        assert_eq!(editor.ime_text(), state("\n|item\n"));
    }

    #[test]
    fn delete_at_the_end_of_a_block_merges_the_next() {
        let mut editor = editor("one\n\ntwo\n", Pos::new(0, 3));
        sync(&mut editor, "\none|");
        assert_eq!(editor.markdown(), "onetwo\n");
    }

    #[test]
    fn a_selection_is_replaced() {
        let mut editor = editor_at("hello world\n", 0);
        sync(&mut editor, "\nhello |world\n");
        let mut moved = state("\nhello world\n");
        moved.selection = (7, 12);
        editor.update(Input::ImeSync(moved));
        assert_eq!(editor.sel.range(), (Pos::new(0, 6), Pos::new(0, 11)));
        sync(&mut editor, "\nhello there|\n");
        assert_eq!(editor.markdown(), "hello there\n");
    }

    #[test]
    fn a_selection_across_blocks_is_replaced() {
        let mut editor = editor("one\n\ntwo\n", Pos::new(0, 1));
        editor.sel.head = Pos::new(1, 1);
        assert_eq!(editor.ime_text(), state("\nt|wo\n"));
        sync(&mut editor, "\ntx|wo\n");
        assert_eq!(editor.markdown(), "oxwo\n");
    }

    #[test]
    fn a_new_line_from_the_keyboard_splits_the_block() {
        let mut editor = editor_at("onetwo\n", 3);
        sync(&mut editor, "\none\n|two\n");
        assert_eq!(editor.markdown(), "one\n\ntwo\n");
        assert_eq!(editor.ime_text(), state("\n|two\n"));
    }

    #[test]
    fn autoformat_still_works() {
        let mut editor = editor_at("", 0);
        sync(
            &mut editor,
            "
-|
",
        );
        sync(
            &mut editor,
            "
- |
",
        );
        assert!(
            editor.markdown().starts_with('-'),
            "{:?}",
            editor.markdown()
        );
        // The keyboard is told the block's text is now empty.
        assert_eq!(
            editor.ime_text(),
            state(
                "
|
"
            )
        );
    }

    #[test]
    fn ignored_when_read_only_or_not_focused() {
        let mut editor = EditorCore::with_markdown("hello\n");
        sync(&mut editor, "\nhellox|\n");
        assert_eq!(editor.markdown(), "hello\n");

        let mut editor = editor_at("hello\n", 5).read_only(true);
        sync(&mut editor, "\nhellox|\n");
        assert_eq!(editor.markdown(), "hello\n");
    }

    #[test]
    fn other_input_ends_the_composition() {
        let mut editor = editor_at("\n", 0);
        sync(&mut editor, "\n[hi]|\n");
        assert!(editor.preedit.is_some());
        editor.update(Input::Key(
            Key::Named(Named::ArrowLeft),
            Modifiers::empty(),
            None,
        ));
        assert!(editor.ime_text().composing.is_none());
        assert_eq!(editor.markdown(), "hi\n");
    }
}
