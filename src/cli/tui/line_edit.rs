//! Minimal single-line text editing over a `String` plus a char-index
//! cursor. Shared by the search box and the command-palette query so both
//! get the same readline-style behavior.
//!
//! The cursor is a *char* index (0..=char count), never a byte index, so
//! callers can move it without worrying about UTF-8 boundaries.

/// Clamp a cursor to the text's char count.
pub fn clamp(text: &str, cursor: usize) -> usize {
    cursor.min(text.chars().count())
}

/// Cursor position at the end of the text.
pub fn end(text: &str) -> usize {
    text.chars().count()
}

fn byte_index(text: &str, cursor: usize) -> usize {
    text.char_indices()
        .nth(cursor)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

pub fn insert(text: &mut String, cursor: &mut usize, ch: char) {
    *cursor = clamp(text, *cursor);
    text.insert(byte_index(text, *cursor), ch);
    *cursor += 1;
}

/// Delete the char before the cursor. Returns whether the text changed.
pub fn backspace(text: &mut String, cursor: &mut usize) -> bool {
    *cursor = clamp(text, *cursor);
    if *cursor == 0 {
        return false;
    }
    *cursor -= 1;
    text.remove(byte_index(text, *cursor));
    true
}

/// Delete the char under the cursor (forward delete). Returns whether the
/// text changed.
pub fn delete_forward(text: &mut String, cursor: &mut usize) -> bool {
    *cursor = clamp(text, *cursor);
    if *cursor >= text.chars().count() {
        return false;
    }
    text.remove(byte_index(text, *cursor));
    true
}

/// Delete the word before the cursor (and the whitespace between it and the
/// cursor), like readline Ctrl+W. Returns whether the text changed.
pub fn delete_word_back(text: &mut String, cursor: &mut usize) -> bool {
    *cursor = clamp(text, *cursor);
    if *cursor == 0 {
        return false;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut new_cursor = *cursor;
    while new_cursor > 0 && chars[new_cursor - 1].is_whitespace() {
        new_cursor -= 1;
    }
    while new_cursor > 0 && !chars[new_cursor - 1].is_whitespace() {
        new_cursor -= 1;
    }
    let start = byte_index(text, new_cursor);
    let end = byte_index(text, *cursor);
    text.replace_range(start..end, "");
    *cursor = new_cursor;
    true
}

/// Clear the whole line. Returns whether the text changed.
pub fn clear(text: &mut String, cursor: &mut usize) -> bool {
    *cursor = 0;
    if text.is_empty() {
        return false;
    }
    text.clear();
    true
}

pub fn move_left(cursor: &mut usize) {
    *cursor = cursor.saturating_sub(1);
}

pub fn move_right(text: &str, cursor: &mut usize) {
    *cursor = (*cursor + 1).min(text.chars().count());
}

pub fn move_home(cursor: &mut usize) {
    *cursor = 0;
}

pub fn move_end(text: &str, cursor: &mut usize) {
    *cursor = text.chars().count();
}

/// Split the text for caret rendering: (before cursor, char under cursor,
/// after that char). The caret is drawn as a reversed cell over the "under"
/// char, or as a bar when the cursor sits at the end.
pub fn split_at_cursor(text: &str, cursor: usize) -> (String, Option<char>, String) {
    let cursor = clamp(text, cursor);
    let mut chars = text.chars();
    let before: String = chars.by_ref().take(cursor).collect();
    let under = chars.next();
    let after: String = chars.collect();
    (before, under, after)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_at_cursor_positions() {
        let mut text = String::from("hllo");
        let mut cursor = 1;
        insert(&mut text, &mut cursor, 'e');
        assert_eq!(text, "hello");
        assert_eq!(cursor, 2);

        let mut cursor = end(&text);
        insert(&mut text, &mut cursor, '!');
        assert_eq!(text, "hello!");
        assert_eq!(cursor, 6);
    }

    #[test]
    fn backspace_and_delete_are_directional() {
        let mut text = String::from("abc");
        let mut cursor = 1; // between a and b
        assert!(backspace(&mut text, &mut cursor));
        assert_eq!((text.as_str(), cursor), ("bc", 0));
        assert!(!backspace(&mut text, &mut cursor));

        let mut text = String::from("abc");
        let mut cursor = 1;
        assert!(delete_forward(&mut text, &mut cursor));
        assert_eq!((text.as_str(), cursor), ("ac", 1));
        cursor = end(&text);
        assert!(!delete_forward(&mut text, &mut cursor));
    }

    #[test]
    fn delete_word_back_respects_cursor() {
        let mut text = String::from("foo bar baz");
        let mut cursor = 7; // end of "bar"
        assert!(delete_word_back(&mut text, &mut cursor));
        assert_eq!((text.as_str(), cursor), ("foo  baz", 4));

        let mut text = String::from("foo bar  ");
        let mut cursor = end(&text);
        assert!(delete_word_back(&mut text, &mut cursor));
        assert_eq!((text.as_str(), cursor), ("foo ", 4));
    }

    #[test]
    fn multibyte_text_is_edited_by_chars_not_bytes() {
        let mut text = String::from("héllo");
        let mut cursor = 2; // after é
        assert!(backspace(&mut text, &mut cursor));
        assert_eq!((text.as_str(), cursor), ("hllo", 1));

        let mut text = String::from("日本語");
        let mut cursor = 1;
        insert(&mut text, &mut cursor, 'x');
        assert_eq!(text, "日x本語");
        assert_eq!(cursor, 2);

        let (before, under, after) = split_at_cursor("日本語", 1);
        assert_eq!(
            (before.as_str(), under, after.as_str()),
            ("日", Some('本'), "語")
        );
    }

    #[test]
    fn movement_clamps_to_bounds() {
        let text = "ab";
        let mut cursor = 0;
        move_left(&mut cursor);
        assert_eq!(cursor, 0);
        move_right(text, &mut cursor);
        move_right(text, &mut cursor);
        move_right(text, &mut cursor);
        assert_eq!(cursor, 2);
        move_home(&mut cursor);
        assert_eq!(cursor, 0);
        move_end(text, &mut cursor);
        assert_eq!(cursor, 2);
    }
}
