/// Truncates `s` to at most `max_len` characters, appending `…` if it was cut.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        format!("{}…", s.chars().take(max_len).collect::<String>())
    } else {
        s.to_owned()
    }
}
