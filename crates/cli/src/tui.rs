//! Modal full-screen history picker (ADR 0016). `mbx tui search` takes over
//! the terminal via the alternate screen, filters the local history database
//! interactively, and writes the selected command — and only that command —
//! to stdout for Bash to insert. The pure pieces (key decoding, window math,
//! width truncation, query tiers) are separated so they can be tested without
//! a terminal; `crates/pty/tests/tui_search.rs` drives the live loop.

use crate::history::HistorySearch;
use crate::term::{SavedMode, Terminal, Wait};
use unicode_width::UnicodeWidthChar;

/// Rows fetched per keystroke. Bounded by design: prefix queries hit the
/// covering index, fuzzy scans the bounded recent pool (ADR 0008/`HIST-009`).
const RESULT_LIMIT: usize = 200;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    Up,
    Down,
    Enter,
    /// Tab also accepts the selection.
    Accept,
    Backspace,
    Escape,
    Char(char),
}

/// Decodes the front of `buffer`. Returns the key (if any), how many bytes it
/// consumed, and whether the sequence is incomplete — an escape or partial
/// multi-byte character that needs the next read to disambiguate.
pub fn decode(buffer: &[u8]) -> (Option<Key>, usize, bool) {
    match buffer.first() {
        None => (None, 0, false),
        Some(0x1b) => decode_escape(buffer),
        Some(0x03) => (Some(Key::Escape), 1, false),
        Some(0x08 | 0x7f) => (Some(Key::Backspace), 1, false),
        Some(0x09) => (Some(Key::Accept), 1, false),
        Some(0x0a | 0x0d) => (Some(Key::Enter), 1, false),
        Some(0x10) => (Some(Key::Up), 1, false),
        Some(0x0e) => (Some(Key::Down), 1, false),
        Some(byte @ 0x20..=0x7e) => (Some(Key::Char(*byte as char)), 1, false),
        Some(byte) if byte.is_ascii() => (None, 1, false),
        Some(_) => decode_utf8(buffer),
    }
}

fn decode_escape(buffer: &[u8]) -> (Option<Key>, usize, bool) {
    match buffer.get(1) {
        // A lone escape byte: cancel if nothing follows, otherwise wait.
        None => (None, 0, true),
        Some(b'[') => {
            let final_at = buffer[2..]
                .iter()
                .position(|byte| (0x40..=0x7e).contains(byte))
                .map(|index| index + 2);
            match final_at {
                None => (None, 0, true),
                Some(end) => {
                    let key = match buffer[end] {
                        b'A' => Some(Key::Up),
                        b'B' => Some(Key::Down),
                        _ => None,
                    };
                    (key, end + 1, false)
                }
            }
        }
        Some(b'O') => match buffer.get(2) {
            None => (None, 0, true),
            Some(b'A') => (Some(Key::Up), 3, false),
            Some(b'B') => (Some(Key::Down), 3, false),
            Some(_) => (None, 3, false),
        },
        // ESC followed by any other byte: the escape itself is the key.
        Some(_) => (Some(Key::Escape), 1, false),
    }
}

fn decode_utf8(buffer: &[u8]) -> (Option<Key>, usize, bool) {
    let lead = buffer[0];
    let len = if lead >= 0xF0 {
        4
    } else if lead >= 0xE0 {
        3
    } else if lead >= 0xC0 {
        2
    } else {
        1
    };
    if buffer.len() < len {
        return (None, 0, true);
    }
    match std::str::from_utf8(&buffer[..len]) {
        Ok(text) => match text.chars().next() {
            Some(ch) => (Some(Key::Char(ch)), len, false),
            None => (None, 1, false),
        },
        Err(_) => (None, 1, false),
    }
}

/// The visible slice of the result list for a selection. Minimal scrolling:
/// the list only moves when the selection would leave the window.
pub fn window(len: usize, selected: usize, view: usize) -> (usize, usize) {
    if view == 0 || len == 0 {
        return (0, 0);
    }
    let view = view.min(len);
    let start = selected.min(len - 1).saturating_sub(view - 1);
    (start, start + view)
}

/// Truncates to at most `max_columns` display columns, never splitting a
/// wide or combining sequence.
pub fn truncate_width(text: &str, max_columns: usize) -> &str {
    let mut width = 0;
    for (index, ch) in text.char_indices() {
        let ch_width = ch.width().unwrap_or(0);
        if width + ch_width > max_columns {
            return &text[..index];
        }
        width += ch_width;
    }
    text
}

pub struct Picker {
    pub query: String,
    pub rows: Vec<String>,
    pub selected: usize,
}

impl Picker {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            rows: Vec::new(),
            selected: 0,
        }
    }

    pub fn set_rows(&mut self, rows: Vec<String>) {
        self.rows = rows;
        self.selected = 0;
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.rows.len() {
            self.selected += 1;
        }
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.rows.get(self.selected).map(String::as_str)
    }
}

impl Default for Picker {
    fn default() -> Self {
        Self::new()
    }
}

/// Mirrors the chord widget's tiers (`bash/search.bash` `_mbx_search_query`):
/// exact prefix scoped to the current directory first, then global prefix,
/// then fuzzy in the cwd, then global fuzzy. An empty query is the recent
/// list.
pub fn query_rows(search: &dyn HistorySearch, cwd: &str, query: &str) -> Vec<String> {
    let take = |result: Result<Vec<crate::history::HistoryEntry>, crate::history::HistoryError>| {
        result
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|entry| entry.command_text)
                    .collect()
            })
            .unwrap_or_default()
    };
    if query.is_empty() {
        return take(search.recent(RESULT_LIMIT));
    }
    for rows in [
        take(search.exact_prefix_in_cwd(query, cwd, RESULT_LIMIT)),
        take(search.exact_prefix(query, RESULT_LIMIT)),
        take(search.fuzzy_in_cwd(query, cwd, RESULT_LIMIT)),
        take(search.fuzzy(query, RESULT_LIMIT)),
    ] {
        if !rows.is_empty() {
            return rows;
        }
    }
    Vec::new()
}

fn draw(term: &Terminal, picker: &Picker, title: &str) -> Result<(), String> {
    let (rows_total, columns) = term.size().unwrap_or((24, 80));
    let columns = usize::from(columns).max(8);
    let view = usize::from(rows_total).saturating_sub(3).max(1);
    let (start, end) = window(picker.rows.len(), picker.selected, view);

    let mut frame = String::from("\x1b[H");
    let heading = format!(
        " {title} - {} match{} - query: {}",
        picker.rows.len(),
        if picker.rows.len() == 1 { "" } else { "es" },
        picker.query
    );
    frame.push_str(truncate_width(&heading, columns - 1));
    frame.push_str("\x1b[K\r\n");
    let query_line = format!("> {}_", picker.query);
    frame.push_str(truncate_width(&query_line, columns - 1));
    frame.push_str("\x1b[K\r\n");
    for (index, row) in picker.rows[start..end].iter().enumerate() {
        let selected = start + index == picker.selected;
        let prefix = if selected { "\x1b[7m > " } else { "   " };
        frame.push_str(prefix);
        frame.push_str(truncate_width(row, columns - 4));
        if selected {
            frame.push_str("\x1b[0m");
        }
        frame.push_str("\x1b[K\r\n");
    }
    frame.push_str(
        "\x1b[K Up/Down: select   Enter: insert   Esc/Ctrl-C: cancel   Tab: insert\x1b[J",
    );
    term.write(frame.as_bytes())
}

/// Runs the picker to completion. `Ok(Some(command))` is the user's
/// selection; `Ok(None)` is a cancel. The terminal is restored on every path
/// (including panics, via the guard's `Drop`) before the result is returned.
pub fn run_history(
    seed: Option<&str>,
    search: &dyn HistorySearch,
    cwd: &str,
) -> Result<Option<String>, String> {
    run(seed, &|query| query_rows(search, cwd, query), "MBX history")
}

/// Filters a fixed candidate list (the completion picker's source): an empty
/// query keeps everything, otherwise a case-insensitive substring match.
pub fn filter_lines(lines: &[String], query: &str) -> Vec<String> {
    if query.is_empty() {
        return lines.to_vec();
    }
    let needle = query.to_lowercase();
    lines
        .iter()
        .filter(|line| line.to_lowercase().contains(&needle))
        .cloned()
        .collect()
}

/// Acceptance bounds for `tui complete` candidate intake: completion
/// snapshots are small, so a bounded reader is both a resource bound and a
/// protocol bound (BST-006 discipline).
pub const MAX_COMPLETE_CANDIDATES: usize = 512;
pub const MAX_COMPLETE_LINE_BYTES: usize = 4096;

/// Reads newline-separated candidates from `reader`, bounded. Over-long
/// lines and rows past the cap are dropped, not truncated (no silent
/// mutation of candidate bytes).
pub fn read_candidates(mut reader: impl std::io::BufRead) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    let mut buf = String::new();
    while lines.len() < MAX_COMPLETE_CANDIDATES {
        buf.clear();
        let read = reader
            .read_line(&mut buf)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        let line = buf.trim_end_matches(['\n', '\r']);
        if line.is_empty() || line.len() > MAX_COMPLETE_LINE_BYTES || line.contains('\0') {
            continue;
        }
        lines.push(line.to_owned());
    }
    Ok(lines)
}

/// The completion picker: same loop over a fixed candidate list read from
/// stdin by the caller (ADR 0016 follow-up; C1 in `docs/next-steps-todo.md`).
pub fn run_over_lines(seed: Option<&str>, lines: &[String]) -> Result<Option<String>, String> {
    let owned = lines.to_vec();
    run(
        seed,
        &|query| filter_lines(&owned, query),
        "MBX completions",
    )
}

/// Runs the picker to completion against `refill`, which produces the row
/// list for the current query. The terminal is restored on every path
/// (including panics, via the guard's `Drop`) before the result is returned.
pub fn run(
    seed: Option<&str>,
    refill: &dyn Fn(&str) -> Vec<String>,
    title: &str,
) -> Result<Option<String>, String> {
    let term = Terminal::open()?;
    let mut saved: SavedMode = term.enable_raw()?;
    term.write(b"\x1b[?1049h\x1b[?25l\x1b[H\x1b[2J")?;
    saved.entered_alt_screen();

    let mut picker = Picker::new();
    picker.query = seed.unwrap_or_default().to_string();
    picker.set_rows(refill(&picker.query));

    let mut pending: Vec<u8> = Vec::new();
    let mut chunk = [0_u8; 1024];
    // Redraw only when something changed: idle polls must not write frames.
    let mut dirty = true;
    'picker: loop {
        if dirty {
            draw(&term, &picker, title)?;
            dirty = false;
        }
        match term.wait_event(200)? {
            Wait::Input => {}
            Wait::Signal => {
                if term.resize_requested() {
                    dirty = true;
                    continue;
                }
                break 'picker Ok(None);
            }
            Wait::Timeout => continue,
        }
        let count = term.read_input(&mut chunk)?;
        pending.extend_from_slice(&chunk[..count]);
        while !pending.is_empty() {
            let (key, consumed, incomplete) = decode(&pending);
            if incomplete {
                if term.input_pending() {
                    break;
                }
                // Nothing follows: a lone escape byte cancels, a truncated
                // multi-byte tail is garbage to drop.
                if pending == [0x1b] {
                    pending.clear();
                    break 'picker Ok(None);
                }
                pending.clear();
                break;
            }
            pending.drain(..consumed);
            match key {
                None => {}
                Some(Key::Char(ch)) => {
                    picker.query.push(ch);
                    picker.set_rows(refill(&picker.query));
                    dirty = true;
                }
                Some(Key::Backspace) => {
                    picker.query.pop();
                    picker.set_rows(refill(&picker.query));
                    dirty = true;
                }
                Some(Key::Up) => {
                    picker.move_up();
                    dirty = true;
                }
                Some(Key::Down) => {
                    picker.move_down();
                    dirty = true;
                }
                Some(Key::Enter | Key::Accept) => {
                    break 'picker Ok(picker.selected_text().map(str::to_owned));
                }
                Some(Key::Escape) => break 'picker Ok(None),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{HistoryEntry, HistoryError, HistoryErrorKind};
    use std::sync::Mutex;

    fn key_of(bytes: &[u8]) -> (Option<Key>, usize, bool) {
        decode(bytes)
    }

    #[test]
    fn decodes_arrows_enter_backspace_tab_and_ctrl_motion() {
        assert_eq!(key_of(b"\x1b[A"), (Some(Key::Up), 3, false));
        assert_eq!(key_of(b"\x1b[B"), (Some(Key::Down), 3, false));
        assert_eq!(key_of(b"\x1bOA"), (Some(Key::Up), 3, false));
        assert_eq!(key_of(b"\x1b[1;5A"), (Some(Key::Up), 6, false));
        assert_eq!(key_of(b"\x1b[Z"), (None, 3, false));
        assert_eq!(key_of(b"\r"), (Some(Key::Enter), 1, false));
        assert_eq!(key_of(b"\n"), (Some(Key::Enter), 1, false));
        assert_eq!(key_of(b"\x7f"), (Some(Key::Backspace), 1, false));
        assert_eq!(key_of(b"\t"), (Some(Key::Accept), 1, false));
        assert_eq!(key_of(b"\x10"), (Some(Key::Up), 1, false));
        assert_eq!(key_of(b"\x0e"), (Some(Key::Down), 1, false));
        assert_eq!(key_of(b"\x03"), (Some(Key::Escape), 1, false));
        assert_eq!(key_of(b"x"), (Some(Key::Char('x')), 1, false));
    }

    #[test]
    fn lone_or_partial_escapes_are_incomplete() {
        assert_eq!(key_of(&[0x1b]), (None, 0, true));
        assert_eq!(key_of(b"\x1b["), (None, 0, true));
        assert_eq!(key_of(b"\x1b[12"), (None, 0, true));
        // ESC followed by an unrelated byte is a completed escape key press.
        assert_eq!(key_of(b"\x1bx"), (Some(Key::Escape), 1, false));
    }

    #[test]
    fn decodes_multibyte_characters_and_reports_partial_tails() {
        let mut bytes = "é".to_owned().into_bytes();
        assert_eq!(decode(&bytes), (Some(Key::Char('é')), 2, false));
        let han = "日".to_owned().into_bytes();
        assert_eq!(decode(&han), (Some(Key::Char('日')), 3, false));
        bytes.pop();
        assert_eq!(decode(&bytes), (None, 0, true));
        // Invalid lead byte: skip exactly one.
        assert_eq!(decode(&[0x80]), (None, 1, false));
    }

    #[test]
    fn window_keeps_selection_visible_with_minimal_scroll() {
        assert_eq!(window(0, 0, 5), (0, 0));
        assert_eq!(window(3, 0, 5), (0, 3));
        assert_eq!(window(10, 0, 4), (0, 4));
        assert_eq!(window(10, 3, 4), (0, 4));
        assert_eq!(window(10, 4, 4), (1, 5));
        assert_eq!(window(10, 9, 4), (6, 10));
        assert_eq!(window(10, 5, 0), (0, 0));
        assert_eq!(window(10, 99, 4), (6, 10));
    }

    #[test]
    fn truncation_never_splits_a_wide_sequence() {
        assert_eq!(truncate_width("abcdef", 4), "abcd");
        assert_eq!(truncate_width("abcd", 9), "abcd");
        // CJK characters are two columns; '日' fits exactly once in 2.
        assert_eq!(truncate_width("日日", 2), "日");
        assert_eq!(truncate_width("日日", 3), "日");
        // A combining mark adds zero columns and stays attached.
        assert_eq!(truncate_width("e\u{301}x", 1), "e\u{301}");
        assert_eq!(truncate_width("", 5), "");
    }

    struct FakeSearch {
        calls: Mutex<Vec<String>>,
        rows: Vec<String>,
    }

    impl FakeSearch {
        fn new(rows: Vec<String>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                rows,
            }
        }

        fn record(&self, call: &str) -> Vec<String> {
            self.calls
                .lock()
                .expect("fake search lock")
                .push(call.to_owned());
            self.rows.clone()
        }
    }

    fn entry(text: &str) -> HistoryEntry {
        HistoryEntry {
            session_id: "s".to_owned(),
            event_sequence: 1,
            history_number: None,
            command_text: text.to_owned(),
            start_cwd: "/w".to_owned(),
            completed_at: "t".to_owned(),
            status: 0,
            duration_ms: None,
            host: "h".to_owned(),
            user: "u".to_owned(),
            repo_root: None,
            repo_branch: None,
        }
    }

    impl HistorySearch for FakeSearch {
        fn recent(&self, _limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(self
                .record("recent")
                .into_iter()
                .map(|text| entry(&text))
                .collect())
        }

        fn exact_prefix(
            &self,
            prefix: &str,
            _limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(self
                .record(&format!("prefix:{prefix}"))
                .into_iter()
                .map(|text| entry(&text))
                .collect())
        }

        fn exact_prefix_in_cwd(
            &self,
            prefix: &str,
            cwd: &str,
            _limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(self
                .record(&format!("prefix-cwd:{prefix}:{cwd}"))
                .into_iter()
                .map(|text| entry(&text))
                .collect())
        }

        fn by_cwd(&self, _cwd: &str, _limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(self
                .record("cwd")
                .into_iter()
                .map(|text| entry(&text))
                .collect())
        }

        fn by_repo(
            &self,
            _repo_root: &str,
            _limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(self
                .record("repo")
                .into_iter()
                .map(|text| entry(&text))
                .collect())
        }

        fn by_branch(
            &self,
            _repo_branch: &str,
            _limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(self
                .record("branch")
                .into_iter()
                .map(|text| entry(&text))
                .collect())
        }

        fn fuzzy(&self, needle: &str, _limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(self
                .record(&format!("fuzzy:{needle}"))
                .into_iter()
                .map(|text| entry(&text))
                .collect())
        }

        fn fuzzy_in_cwd(
            &self,
            needle: &str,
            cwd: &str,
            _limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(self
                .record(&format!("fuzzy-cwd:{needle}:{cwd}"))
                .into_iter()
                .map(|text| entry(&text))
                .collect())
        }

        fn failed(&self, _limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
            Err(HistoryError::new(
                HistoryErrorKind::Read,
                "unused by the picker",
            ))
        }
    }

    #[test]
    fn empty_query_uses_recent_and_typed_query_walks_the_tiers() {
        let search = FakeSearch::new(vec!["echo one".to_owned()]);
        assert_eq!(query_rows(&search, "/w", ""), vec!["echo one"]);
        assert_eq!(
            search.calls.lock().expect("fake search lock").as_slice(),
            ["recent"]
        );

        let search = FakeSearch::new(vec!["git status".to_owned()]);
        assert_eq!(query_rows(&search, "/w", "git"), vec!["git status"]);
        assert_eq!(
            search.calls.lock().expect("fake search lock").as_slice(),
            [
                "prefix-cwd:git:/w",
                "prefix:git",
                "fuzzy-cwd:git:/w",
                "fuzzy:git"
            ]
        );
    }

    #[test]
    fn first_nonempty_tier_wins() {
        let search = FakeSearch::new(vec![]);
        assert!(query_rows(&search, "/w", "zz").is_empty());
        assert_eq!(
            search.calls.lock().expect("fake search lock").as_slice(),
            [
                "prefix-cwd:zz:/w",
                "prefix:zz",
                "fuzzy-cwd:zz:/w",
                "fuzzy:zz"
            ]
        );
    }

    #[test]
    fn filter_lines_matches_case_insensitively_and_empty_keeps_all() {
        let lines = vec!["echo alpha-one".to_owned(), "Git status".to_owned()];
        assert_eq!(filter_lines(&lines, ""), lines);
        assert_eq!(filter_lines(&lines, "alpha"), ["echo alpha-one"]);
        assert_eq!(filter_lines(&lines, "GIT"), ["Git status"]);
        assert!(filter_lines(&lines, "zzz").is_empty());
    }

    #[test]
    fn read_candidates_is_bounded_and_drops_hostile_rows() {
        let input = b"a\nb\n\n".as_slice();
        assert_eq!(read_candidates(input).unwrap(), ["a", "b"]);
        // Over the candidate cap: extras are dropped, not truncated.
        let many: Vec<u8> = (0..600)
            .map(|i| format!("row{i}\n"))
            .collect::<Vec<_>>()
            .concat()
            .into_bytes();
        let read = read_candidates(&many[..]).unwrap();
        assert_eq!(read.len(), MAX_COMPLETE_CANDIDATES);
        assert_eq!(read[511], "row511");
        // An over-long line is dropped whole, not split mid-sequence.
        let long_line = "x".repeat(MAX_COMPLETE_LINE_BYTES + 1);
        let mixed = format!("{long_line}\nok\n");
        assert_eq!(read_candidates(mixed.as_bytes()).unwrap(), ["ok"]);
    }

    #[test]
    fn picker_selection_moves_within_bounds() {
        let mut picker = Picker::new();
        picker.set_rows(vec!["a".into(), "b".into(), "c".into()]);
        picker.move_up();
        assert_eq!(picker.selected, 0);
        picker.move_down();
        picker.move_down();
        assert_eq!(picker.selected, 2);
        picker.move_down();
        assert_eq!(picker.selected, 2);
        assert_eq!(picker.selected_text(), Some("c"));
        picker.set_rows(vec!["z".into()]);
        assert_eq!(picker.selected, 0);
        assert_eq!(picker.selected_text(), Some("z"));
        picker.set_rows(Vec::new());
        assert_eq!(picker.selected_text(), None);
    }
}
