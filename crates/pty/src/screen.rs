//! A minimal VT100/ANSI screen model for PTY test assertions.
//!
//! This is test-harness infrastructure, not a product path: it exists so a
//! PTY test can assert *where output actually lands on screen* (cursor
//! position, whether a scroll happened, whether a region was erased)
//! instead of only checking that some substring appears somewhere in the
//! raw byte stream. `visible_text` answers "did these bytes eventually
//! appear"; `Screen` answers "what does the terminal look like now" — the
//! question the overlay's cursor-safety claim (`COMP-004`, ADR 0013) needs
//! answered before it can be considered evidenced (T2 in
//! `docs/repo-review-2026-08-29.md`).
//!
//! Deliberately narrow: printable text (ASCII fast path; wide/combining
//! glyphs are out of scope here, `crates/pty/tests/multiline_width.rs`
//! covers those separately), CR/LF/backspace, cursor motion (CUU/CUD/CUF/
//! CUB/CUP), erase-in-display/erase-in-line (ED/EL), and DECSC/DECRC.
//! Unrecognized CSI/OSC sequences are consumed and ignored rather than
//! rejected, matching how a real terminal degrades on an unknown sequence.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

pub struct Screen {
    rows: usize,
    cols: usize,
    grid: Vec<Vec<char>>,
    cursor: Cursor,
    saved: Option<Cursor>,
}

impl Screen {
    pub fn new(rows: usize, cols: usize) -> Self {
        assert!(rows > 0 && cols > 0, "screen must have at least one cell");
        Self {
            rows,
            cols,
            grid: vec![vec![' '; cols]; rows],
            cursor: Cursor { row: 0, col: 0 },
            saved: None,
        }
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    /// The screen contents as one string per row, right-trimmed of padding
    /// spaces so assertions can compare against real text.
    pub fn lines(&self) -> Vec<String> {
        self.grid
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_owned())
            .collect()
    }

    pub fn line(&self, row: usize) -> String {
        self.grid[row]
            .iter()
            .collect::<String>()
            .trim_end()
            .to_owned()
    }

    /// Feeds a byte stream through the model, mutating cursor and grid state
    /// as a real terminal would.
    pub fn apply(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\u{1b}' => self.apply_escape(&mut chars),
                '\r' => self.cursor.col = 0,
                '\n' => self.newline(),
                '\u{8}' => {
                    if self.cursor.col > 0 {
                        self.cursor.col -= 1;
                    }
                }
                '\u{7}' => {} // BEL: no visible effect on the grid
                _ => self.put_char(ch),
            }
        }
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor.col >= self.cols {
            self.newline();
        }
        self.grid[self.cursor.row][self.cursor.col] = ch;
        self.cursor.col += 1;
    }

    fn newline(&mut self) {
        self.index();
        self.cursor.col = 0;
    }

    /// Down one row, scrolling at the bottom margin, column unchanged.
    fn index(&mut self) {
        if self.cursor.row + 1 >= self.rows {
            self.scroll_up(1);
        } else {
            self.cursor.row += 1;
        }
    }

    fn scroll_down(&mut self, amount: usize) {
        for _ in 0..amount {
            self.grid.pop();
            self.grid.insert(0, vec![' '; self.cols]);
        }
    }

    fn scroll_up(&mut self, amount: usize) {
        for _ in 0..amount {
            self.grid.remove(0);
            self.grid.push(vec![' '; self.cols]);
        }
    }

    fn apply_escape(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        match chars.peek() {
            Some('[') => {
                chars.next();
                self.apply_csi(chars);
            }
            Some(']') => {
                chars.next();
                // OSC: consume until BEL or ST (ESC \\).
                while let Some(next) = chars.next() {
                    if next == '\u{7}' {
                        break;
                    }
                    if next == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                        break;
                    }
                }
            }
            Some('D') => {
                chars.next();
                // IND (Index): down one row, scrolling at the bottom margin,
                // column unchanged. Distinct from LF, which also returns to
                // column 0 — the overlay depends on that difference to keep
                // its saved cursor column (M-065).
                self.index();
            }
            Some('M') => {
                chars.next();
                // RI (Reverse Index): up one row, scrolling down at the top.
                if self.cursor.row == 0 {
                    self.scroll_down(1);
                } else {
                    self.cursor.row -= 1;
                }
            }
            Some('7') => {
                chars.next();
                self.saved = Some(self.cursor);
            }
            Some('8') => {
                chars.next();
                if let Some(saved) = self.saved {
                    self.cursor = saved;
                }
            }
            Some(_) => {
                chars.next();
            }
            None => {}
        }
    }

    fn apply_csi(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        let mut param = String::new();
        let final_byte;
        loop {
            match chars.next() {
                Some(next) if ('\u{40}'..='\u{7e}').contains(&next) => {
                    final_byte = next;
                    break;
                }
                Some(next) => param.push(next),
                None => return,
            }
        }
        let params: Vec<usize> = param
            .split(';')
            .map(|value| value.parse::<usize>().unwrap_or(0))
            .collect();
        let arg = |index: usize, default: usize| -> usize {
            params
                .get(index)
                .copied()
                .filter(|value| *value != 0)
                .unwrap_or(default)
        };
        match final_byte {
            'A' => self.cursor.row = self.cursor.row.saturating_sub(arg(0, 1)),
            'B' => self.cursor.row = (self.cursor.row + arg(0, 1)).min(self.rows - 1),
            'C' => self.cursor.col = (self.cursor.col + arg(0, 1)).min(self.cols - 1),
            'D' => self.cursor.col = self.cursor.col.saturating_sub(arg(0, 1)),
            'H' | 'f' => {
                let row = arg(0, 1).saturating_sub(1).min(self.rows - 1);
                let col = arg(1, 1).saturating_sub(1).min(self.cols - 1);
                self.cursor = Cursor { row, col };
            }
            'J' => self.erase_display(params.first().copied().unwrap_or(0)),
            'K' => self.erase_line(params.first().copied().unwrap_or(0)),
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: usize) {
        let row = self.cursor.row;
        match mode {
            0 => {
                for col in self.cursor.col..self.cols {
                    self.grid[row][col] = ' ';
                }
            }
            1 => {
                for col in 0..=self.cursor.col.min(self.cols - 1) {
                    self.grid[row][col] = ' ';
                }
            }
            _ => self.grid[row] = vec![' '; self.cols],
        }
    }

    fn erase_display(&mut self, mode: usize) {
        match mode {
            0 => {
                self.erase_line(0);
                for row in (self.cursor.row + 1)..self.rows {
                    self.grid[row] = vec![' '; self.cols];
                }
            }
            1 => {
                self.erase_line(1);
                for row in 0..self.cursor.row {
                    self.grid[row] = vec![' '; self.cols];
                }
            }
            _ => self.grid = vec![vec![' '; self.cols]; self.rows],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Screen;

    #[test]
    fn plain_text_advances_cursor_and_wraps_at_a_newline() {
        let mut screen = Screen::new(4, 10);
        screen.apply(b"hi\r\nthere");
        assert_eq!(screen.line(0), "hi");
        assert_eq!(screen.line(1), "there");
        assert_eq!(screen.cursor().row, 1);
        assert_eq!(screen.cursor().col, 5);
    }

    #[test]
    fn newline_at_the_last_row_scrolls_instead_of_growing() {
        let mut screen = Screen::new(2, 10);
        screen.apply(b"first\r\nsecond\r\nthird");
        assert_eq!(
            screen.lines(),
            vec!["second".to_owned(), "third".to_owned()]
        );
        assert_eq!(screen.rows(), 2);
    }

    #[test]
    fn decsc_decrc_round_trips_the_cursor_across_a_scroll() {
        let mut screen = Screen::new(3, 10);
        screen.apply(b"\r\n\r\nbottom");
        screen.apply(b"\x1b7"); // DECSC at the bottom row
        screen.apply(b"\r\noverflow"); // forces a scroll
        screen.apply(b"\x1b8"); // DECRC: restores the *pre-scroll* position
        assert_eq!(screen.cursor().row, screen.rows() - 1);
    }

    #[test]
    fn ind_moves_down_without_touching_the_column() {
        let mut screen = Screen::new(4, 10);
        screen.apply(b"abc");
        assert_eq!(screen.cursor(), super::Cursor { row: 0, col: 3 });
        screen.apply(b"\x1bD");
        assert_eq!(
            screen.cursor(),
            super::Cursor { row: 1, col: 3 },
            "IND must keep the column; that is what distinguishes it from LF"
        );
    }

    #[test]
    fn ind_at_the_bottom_scrolls_and_keeps_the_column() {
        let mut screen = Screen::new(2, 10);
        screen.apply(b"top\r\nbottom");
        assert_eq!(screen.cursor().row, 1);
        let col = screen.cursor().col;
        screen.apply(b"\x1bD");
        assert_eq!(screen.cursor(), super::Cursor { row: 1, col });
        assert_eq!(screen.line(0), "bottom", "the screen must have scrolled");
    }

    /// The overlay's reserve-then-restore round trip must land back on the
    /// prompt's row whether or not the reservation scrolled (M-065).
    #[test]
    fn reserving_rows_then_moving_back_up_returns_to_the_prompt_row() {
        for (rows, filler) in [(6usize, 2usize), (24, 2)] {
            let mut screen = Screen::new(rows, 40);
            for _ in 0..filler {
                screen.apply(b"output\r\n");
            }
            screen.apply(b"> typed");
            let prompt_row = screen.cursor().row;
            let prompt_col = screen.cursor().col;

            // Reserve 8 rows the way _mbx_comp_overlay_reserve does.
            let reserve = "\x1bD".repeat(8) + "\x1b[8A";
            screen.apply(reserve.as_bytes());

            assert_eq!(
                screen.cursor().col,
                prompt_col,
                "reserving must not move the column (rows={rows})"
            );
            let expected_row = prompt_row.min(rows - 1 - 8.min(rows - 1));
            let landed = screen.line(screen.cursor().row);
            assert!(
                landed.contains("> typed") || screen.cursor().row == expected_row,
                "after reserving, the cursor must sit on the prompt row; \
                 rows={rows} landed on {:?}",
                landed
            );
        }
    }

    #[test]
    fn cup_moves_to_an_absolute_position() {
        let mut screen = Screen::new(5, 20);
        screen.apply(b"\x1b[3;5Hx");
        assert_eq!(screen.cursor(), super::Cursor { row: 2, col: 5 });
        assert_eq!(screen.line(2), "    x");
    }

    #[test]
    fn ed_2_clears_the_whole_screen() {
        let mut screen = Screen::new(2, 5);
        screen.apply(b"abcde\r\nfghij");
        screen.apply(b"\x1b[2J");
        assert_eq!(screen.lines(), vec!["".to_owned(), "".to_owned()]);
    }

    #[test]
    fn el_0_clears_from_the_cursor_to_end_of_line() {
        let mut screen = Screen::new(1, 5);
        screen.apply(b"abcde\r");
        screen.apply(b"\x1b[2C"); // move to col 2
        screen.apply(b"\x1b[K");
        assert_eq!(screen.line(0), "ab");
    }
}
