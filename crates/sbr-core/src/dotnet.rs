//! Small helpers reproducing .NET BCL parsing semantics.

/// `int.Parse` / `int.TryParse` with the default `NumberStyles.Integer`.
pub fn int_parse(s: &str) -> Option<i32> {
    let t = s.trim_matches(|c: char| c.is_whitespace());
    let t = t.strip_prefix('+').unwrap_or(t);
    if t.is_empty() || t.starts_with('+') {
        return None;
    }
    t.parse().ok()
}
