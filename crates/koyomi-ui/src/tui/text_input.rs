#[derive(Debug, Clone)]
pub(super) struct TextInput {
    content: String,
    cursor: usize,
}

impl TextInput {
    pub(super) fn new(initial: &str) -> Self {
        let cursor = initial.chars().count();
        Self { content: initial.to_string(), cursor }
    }

    pub(super) fn content(&self) -> &str {
        &self.content
    }

    pub(super) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(super) fn insert(&mut self, ch: char) {
        let byte_pos = self.byte_offset(self.cursor);
        self.content.insert(byte_pos, ch);
        self.cursor += 1;
    }

    pub(super) fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.cursor - 1;
        let start = self.byte_offset(prev);
        let end = self.byte_offset(self.cursor);
        self.content.replace_range(start..end, "");
        self.cursor = prev;
    }

    pub(super) fn delete(&mut self) {
        let len = self.content.chars().count();
        if self.cursor >= len {
            return;
        }
        let start = self.byte_offset(self.cursor);
        let end = self.byte_offset(self.cursor + 1);
        self.content.replace_range(start..end, "");
    }

    pub(super) fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub(super) fn move_right(&mut self) {
        let len = self.content.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    pub(super) fn home(&mut self) {
        self.cursor = 0;
    }

    pub(super) fn end(&mut self) {
        self.cursor = self.content.chars().count();
    }

    fn byte_offset(&self, char_index: usize) -> usize {
        self.content.char_indices().nth(char_index).map(|(i, _)| i).unwrap_or(self.content.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty() {
        let ti = TextInput::new("");
        assert_eq!(ti.content(), "");
        assert_eq!(ti.cursor(), 0);
    }

    #[test]
    fn new_with_initial() {
        let ti = TextInput::new("hello");
        assert_eq!(ti.content(), "hello");
        assert_eq!(ti.cursor(), 5);
    }

    #[test]
    fn insert_at_end() {
        let mut ti = TextInput::new("ab");
        ti.insert('c');
        assert_eq!(ti.content(), "abc");
        assert_eq!(ti.cursor(), 3);
    }

    #[test]
    fn insert_at_beginning() {
        let mut ti = TextInput::new("bc");
        ti.home();
        ti.insert('a');
        assert_eq!(ti.content(), "abc");
        assert_eq!(ti.cursor(), 1);
    }

    #[test]
    fn insert_in_middle() {
        let mut ti = TextInput::new("ac");
        ti.home();
        ti.move_right();
        ti.insert('b');
        assert_eq!(ti.content(), "abc");
        assert_eq!(ti.cursor(), 2);
    }

    #[test]
    fn backspace_removes_char_before_cursor() {
        let mut ti = TextInput::new("abc");
        ti.backspace();
        assert_eq!(ti.content(), "ab");
        assert_eq!(ti.cursor(), 2);
    }

    #[test]
    fn backspace_at_start_does_nothing() {
        let mut ti = TextInput::new("abc");
        ti.home();
        ti.backspace();
        assert_eq!(ti.content(), "abc");
        assert_eq!(ti.cursor(), 0);
    }

    #[test]
    fn delete_removes_char_at_cursor() {
        let mut ti = TextInput::new("abc");
        ti.home();
        ti.delete();
        assert_eq!(ti.content(), "bc");
        assert_eq!(ti.cursor(), 0);
    }

    #[test]
    fn delete_at_end_does_nothing() {
        let mut ti = TextInput::new("abc");
        ti.delete();
        assert_eq!(ti.content(), "abc");
        assert_eq!(ti.cursor(), 3);
    }

    #[test]
    fn move_left_decrements_cursor() {
        let mut ti = TextInput::new("abc");
        ti.move_left();
        assert_eq!(ti.cursor(), 2);
    }

    #[test]
    fn move_left_saturates_at_zero() {
        let mut ti = TextInput::new("");
        ti.move_left();
        assert_eq!(ti.cursor(), 0);
    }

    #[test]
    fn move_right_increments_cursor() {
        let mut ti = TextInput::new("abc");
        ti.home();
        ti.move_right();
        assert_eq!(ti.cursor(), 1);
    }

    #[test]
    fn move_right_stops_at_end() {
        let mut ti = TextInput::new("abc");
        ti.move_right();
        assert_eq!(ti.cursor(), 3);
    }

    #[test]
    fn home_moves_to_start() {
        let mut ti = TextInput::new("abc");
        ti.home();
        assert_eq!(ti.cursor(), 0);
    }

    #[test]
    fn end_moves_to_end() {
        let mut ti = TextInput::new("abc");
        ti.home();
        ti.end();
        assert_eq!(ti.cursor(), 3);
    }

    #[test]
    fn unicode_insert_and_navigation() {
        let mut ti = TextInput::new("あい");
        assert_eq!(ti.cursor(), 2);
        ti.insert('う');
        assert_eq!(ti.content(), "あいう");
        assert_eq!(ti.cursor(), 3);
    }

    #[test]
    fn unicode_backspace() {
        let mut ti = TextInput::new("あいう");
        ti.backspace();
        assert_eq!(ti.content(), "あい");
        assert_eq!(ti.cursor(), 2);
    }

    #[test]
    fn unicode_delete() {
        let mut ti = TextInput::new("あいう");
        ti.home();
        ti.delete();
        assert_eq!(ti.content(), "いう");
        assert_eq!(ti.cursor(), 0);
    }

    #[test]
    fn mixed_ascii_unicode() {
        let mut ti = TextInput::new("aあb");
        ti.home();
        ti.move_right(); // cursor at 1 (after 'a')
        ti.insert('X');
        assert_eq!(ti.content(), "aXあb");
        assert_eq!(ti.cursor(), 2);
    }
}
