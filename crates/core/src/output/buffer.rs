//! A small editable text buffer with a cursor and a selection.
//!
//! It backs the in-app preview so that cursor keys, the space-bar swipe and
//! Shift-selection can be seen working before platform sinks exist. Positions are
//! byte offsets that always sit on `char` boundaries; the cursor moves by `char`,
//! not by grapheme cluster.

use std::ops::Range;

/// Where a cursor movement goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    Start,
    End,
}

/// Text with a cursor and an optional selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextBuffer {
    text: String,
    /// Byte offset of the cursor.
    cursor: usize,
    /// The other end of the selection, when one is being made.
    anchor: Option<usize>,
}

/// The buffer split for drawing: text before, inside and after the selection.
///
/// The cursor is at the start of `selected` when `cursor_at_start`, otherwise at
/// its end. Without a selection, `selected` is empty and the cursor sits between
/// `before` and `after`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferView<'a> {
    pub before: &'a str,
    pub selected: &'a str,
    pub after: &'a str,
    pub cursor_at_start: bool,
}

impl TextBuffer {
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Byte offset of the cursor.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The selected byte range, if it is not empty.
    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let range = anchor.min(self.cursor)..anchor.max(self.cursor);
        (!range.is_empty()).then_some(range)
    }

    pub fn view(&self) -> BufferView<'_> {
        let range = self.selection().unwrap_or(self.cursor..self.cursor);
        BufferView {
            before: &self.text[..range.start],
            selected: &self.text[range.clone()],
            after: &self.text[range.end..],
            cursor_at_start: self.cursor == range.start,
        }
    }

    /// Inserts text at the cursor, replacing the selection.
    pub fn insert(&mut self, text: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    /// Deletes the selection, or the character before the cursor.
    pub fn backspace(&mut self) {
        if !self.delete_selection() {
            let start = self.prev_char(self.cursor);
            self.text.replace_range(start..self.cursor, "");
            self.cursor = start;
        }
    }

    /// Deletes the selection, or the character after the cursor.
    pub fn delete(&mut self) {
        if !self.delete_selection() {
            let end = self.next_char(self.cursor);
            self.text.replace_range(self.cursor..end, "");
        }
    }

    /// Moves the cursor. With `extend`, the selection grows or shrinks with it;
    /// without, a selection collapses (to its near edge for Left and Right).
    pub fn move_cursor(&mut self, motion: Motion, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else if let Some(range) = self.selection() {
            self.anchor = None;
            match motion {
                Motion::Left => return self.cursor = range.start,
                Motion::Right => return self.cursor = range.end,
                _ => {}
            }
        } else {
            self.anchor = None;
        }
        self.cursor = self.target(motion);
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    fn delete_selection(&mut self) -> bool {
        let selection = self.selection();
        self.anchor = None;
        let Some(range) = selection else {
            return false;
        };
        self.cursor = range.start;
        self.text.replace_range(range, "");
        true
    }

    fn target(&self, motion: Motion) -> usize {
        let at = self.cursor;
        match motion {
            Motion::Left => self.prev_char(at),
            Motion::Right => self.next_char(at),
            Motion::Up => self.vertical(at, false),
            Motion::Down => self.vertical(at, true),
            Motion::WordLeft => self.word_left(at),
            Motion::WordRight => self.word_right(at),
            Motion::LineStart => self.line_start(at),
            Motion::LineEnd => self.line_end(at),
            Motion::Start => 0,
            Motion::End => self.text.len(),
        }
    }

    fn prev_char(&self, at: usize) -> usize {
        self.text[..at]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }

    fn next_char(&self, at: usize) -> usize {
        self.text[at..]
            .chars()
            .next()
            .map_or(at, |c| at + c.len_utf8())
    }

    fn line_start(&self, at: usize) -> usize {
        self.text[..at].rfind('\n').map_or(0, |i| i + 1)
    }

    fn line_end(&self, at: usize) -> usize {
        self.text[at..]
            .find('\n')
            .map_or(self.text.len(), |i| at + i)
    }

    /// The same column on the line above or below, clamped to that line's end.
    fn vertical(&self, at: usize, down: bool) -> usize {
        let start = self.line_start(at);
        let column = self.text[start..at].chars().count();
        let target_start = if down {
            let end = self.line_end(at);
            if end == self.text.len() {
                return end;
            }
            end + 1
        } else {
            if start == 0 {
                return 0;
            }
            self.line_start(start - 1)
        };
        let target_end = self.line_end(target_start);
        self.text[target_start..target_end]
            .char_indices()
            .nth(column)
            .map_or(target_end, |(i, _)| target_start + i)
    }

    fn word_left(&self, at: usize) -> usize {
        let mut chars = self.text[..at].char_indices().rev().peekable();
        while chars.next_if(|(_, c)| c.is_whitespace()).is_some() {}
        let Some(&(mut start, first)) = chars.peek() else {
            return 0;
        };
        let class = CharClass::of(first);
        while let Some((i, _)) = chars.next_if(|(_, c)| CharClass::of(*c) == class) {
            start = i;
        }
        start
    }

    fn word_right(&self, at: usize) -> usize {
        let rest = &self.text[at..];
        let mut chars = rest.char_indices().peekable();
        while chars.next_if(|(_, c)| c.is_whitespace()).is_some() {}
        let Some(&(_, first)) = chars.peek() else {
            return self.text.len();
        };
        let class = CharClass::of(first);
        while chars.next_if(|(_, c)| CharClass::of(*c) == class).is_some() {}
        chars.peek().map_or(self.text.len(), |(i, _)| at + i)
    }
}

/// Word motions stop where the kind of character changes, as in most editors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Word,
    Space,
    Punctuation,
}

impl CharClass {
    fn of(c: char) -> Self {
        if c.is_alphanumeric() || c == '_' {
            Self::Word
        } else if c.is_whitespace() {
            Self::Space
        } else {
            Self::Punctuation
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(text: &str, cursor: usize) -> TextBuffer {
        TextBuffer {
            text: text.to_owned(),
            cursor,
            anchor: None,
        }
    }

    fn moved(text: &str, cursor: usize, motion: Motion) -> usize {
        let mut b = buffer(text, cursor);
        b.move_cursor(motion, false);
        b.cursor()
    }

    #[test]
    fn inserts_and_deletes_around_the_cursor() {
        let mut b = TextBuffer::default();
        b.insert("fn mian");
        // "fn mi|an": delete the `i`, step over the `a`, put the `i` back.
        b.move_cursor(Motion::Left, false);
        b.move_cursor(Motion::Left, false);
        b.backspace();
        b.move_cursor(Motion::Right, false);
        b.insert("i");
        assert_eq!(b.text(), "fn main");
        // "fn |main": Delete removes the character after the cursor.
        b.move_cursor(Motion::WordLeft, false);
        b.delete();
        b.insert("m");
        assert_eq!(b.text(), "fn main");
        b.move_cursor(Motion::Start, false);
        b.backspace();
        b.move_cursor(Motion::End, false);
        b.delete();
        assert_eq!(b.text(), "fn main");
    }

    #[test]
    fn moves_by_character_not_byte() {
        let mut b = TextBuffer::default();
        b.insert("aé→");
        b.move_cursor(Motion::Left, false);
        assert_eq!(b.cursor(), "aé".len());
        b.backspace();
        assert_eq!(b.text(), "a→");
    }

    #[test]
    fn shift_motions_select_and_typing_replaces_the_selection() {
        let mut b = TextBuffer::default();
        b.insert("let x = 1;");
        b.move_cursor(Motion::Left, false);
        b.move_cursor(Motion::Left, true);
        assert_eq!(b.selection(), Some(8..9));
        let view = b.view();
        assert_eq!(
            (view.before, view.selected, view.after),
            ("let x = ", "1", ";")
        );
        assert!(view.cursor_at_start);
        b.insert("42");
        assert_eq!(b.text(), "let x = 42;");
        assert_eq!(b.selection(), None);
    }

    #[test]
    fn plain_left_and_right_collapse_a_selection_to_its_edge() {
        let mut b = buffer("abcdef", 2);
        b.move_cursor(Motion::Right, true);
        b.move_cursor(Motion::Right, true);
        b.move_cursor(Motion::Left, false);
        assert_eq!((b.cursor(), b.selection()), (2, None));

        let mut b = buffer("abcdef", 2);
        b.move_cursor(Motion::Right, true);
        b.move_cursor(Motion::Right, false);
        assert_eq!(b.cursor(), 3);
    }

    #[test]
    fn backspace_deletes_the_selection() {
        let mut b = buffer("hello world", 11);
        b.move_cursor(Motion::WordLeft, true);
        b.backspace();
        assert_eq!(b.text(), "hello ");
    }

    #[test]
    fn line_and_vertical_motions() {
        let text = "fn main() {\n    x\n}";
        assert_eq!(moved(text, 14, Motion::LineStart), 12);
        assert_eq!(moved(text, 14, Motion::LineEnd), 17);
        // Up from column 2 of line 2 lands on column 2 of line 1.
        assert_eq!(moved(text, 14, Motion::Up), 2);
        // Down from column 5 of line 2 is clamped to the end of the short line 3.
        assert_eq!(moved(text, 17, Motion::Down), text.len());
        assert_eq!(moved(text, 2, Motion::Up), 0);
        assert_eq!(moved(text, 18, Motion::Down), text.len());
    }

    #[test]
    fn word_motions_stop_at_character_class_changes() {
        let text = "let foo_bar = x.len();";
        assert_eq!(moved(text, 0, Motion::WordRight), 3);
        assert_eq!(moved(text, 3, Motion::WordRight), 11);
        assert_eq!(moved(text, 11, Motion::WordRight), 13);
        assert_eq!(moved(text, 16, Motion::WordLeft), 15);
        assert_eq!(moved(text, 11, Motion::WordLeft), 4);
        assert_eq!(moved(text, text.len(), Motion::WordLeft), 19);
        assert_eq!(moved("   ", 3, Motion::WordLeft), 0);
        assert_eq!(moved("a   ", 1, Motion::WordRight), 4);
    }
}
