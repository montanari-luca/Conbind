pub mod channel;
pub mod crash_reporter;

pub fn trim_log_tail(log: &str, max_bytes: usize) -> String {
    if log.len() <= max_bytes {
        return log.to_string();
    }
    // Keep the last `max_bytes` bytes, snapped to a char boundary.
    let start = log.len() - max_bytes;
    let start = (start..log.len())
        .find(|&i| log.is_char_boundary(i))
        .unwrap_or(log.len());
    log[start..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_log_tail_to_the_last_max_bytes() {
        let log = "abcdefghij"; // 10 bytes
        assert_eq!(trim_log_tail(log, 4), "ghij");
        assert_eq!(trim_log_tail(log, 100), "abcdefghij");
    }

    #[test]
    fn trims_log_tail_snaps_past_multibyte_char_boundary() {
        // "😀" is 4 bytes; max_bytes=3 lands inside the only char → "" (never a partial char).
        assert_eq!(trim_log_tail("😀", 3), "");
        // "😀🎉" is 8 bytes; max_bytes=5 snaps forward to the next boundary → "🎉".
        assert_eq!(trim_log_tail("😀🎉", 5), "🎉");
    }
}
