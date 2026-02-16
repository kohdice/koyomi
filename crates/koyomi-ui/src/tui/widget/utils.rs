use unicode_width::UnicodeWidthStr;

/// Truncate a string to fit within `max_width` display columns,
/// appending "…" if truncated.
pub fn truncate_str(s: &str, max_width: usize) -> String {
    let width = UnicodeWidthStr::width(s);
    if width <= max_width {
        return s.to_string();
    }

    if max_width == 0 {
        return String::new();
    }

    let mut result = String::new();
    let mut current_width = 0;
    let ellipsis_width = 1; // "…" is 1 column wide in most terminals

    for ch in s.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width + ellipsis_width > max_width {
            break;
        }
        result.push(ch);
        current_width += ch_width;
    }

    result.push('…');
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_width_unchanged() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let result = truncate_str("hello world", 6);
        assert!(result.ends_with('…'));
        assert!(UnicodeWidthStr::width(result.as_str()) <= 6);
    }

    #[test]
    fn truncate_zero_width() {
        assert_eq!(truncate_str("hello", 0), "");
    }

    #[test]
    fn truncate_japanese_text() {
        let result = truncate_str("こんにちは世界", 6);
        // Each CJK character is 2 columns wide
        // 6 columns = 2 chars (4 cols) + ellipsis (1 col) = 5 cols
        assert!(UnicodeWidthStr::width(result.as_str()) <= 6);
        assert!(result.ends_with('…'));
    }
}
