use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi;
use parking_lot::Mutex;
use std::fmt;
use std::sync::Arc;

// Initial terminal size
const DEFAULT_COLS: usize = 80;
const DEFAULT_ROWS: usize = 24;

#[derive(Clone, Debug)]
struct VoidListener;

impl EventListener for VoidListener {
    fn send_event(&self, _event: alacritty_terminal::event::Event) {}
}

#[derive(Clone)]
pub struct TerminalEmulator {
    term: Arc<Mutex<Term<VoidListener>>>,
    parser: Arc<Mutex<ansi::Processor>>,
    scroll_accumulator: Arc<Mutex<f32>>,
}

impl fmt::Debug for TerminalEmulator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TerminalEmulator")
    }
}

// Helper struct for Term dimensions
struct TermDimensions {
    cols: usize,
    rows: usize,
}

impl alacritty_terminal::grid::Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

impl Default for TerminalEmulator {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalEmulator {
    pub fn new() -> Self {
        let mut config = Config::default();
        config.scrolling_history = 10000; // Set explicit history size

        let size = TermDimensions {
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
        };

        let term = Term::new(config, &size, VoidListener);

        Self {
            term: Arc::new(Mutex::new(term)),
            parser: Arc::new(Mutex::new(ansi::Processor::new())),
            scroll_accumulator: Arc::new(Mutex::new(0.0)),
        }
    }

    /// Process input byte (from SSH stream)
    pub fn process_input(&mut self, byte: u8) {
        let mut term = self.term.lock();
        let mut parser = self.parser.lock();

        // Feed the byte to the parser, which updates the terminal state
        // Term implements Handler, so we pass it as the handler.
        // process takes a byte slice.
        parser.advance(&mut *term, &[byte]);
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let size = TermDimensions { cols, rows };
        let mut term = self.term.lock();
        term.resize(size);
    }

    pub fn scroll(&self, delta: f32) {
        let mut accumulator = self.scroll_accumulator.lock();
        *accumulator += delta;

        let steps = (*accumulator) as i32;
        if steps != 0 {
            *accumulator -= steps as f32;
            let mut term = self.term.lock();
            term.scroll_display(alacritty_terminal::grid::Scroll::Delta(steps));
        }
    }

    pub fn render_grid<F>(&self, mut func: F)
    where
        F: FnMut(usize, usize, char, alacritty_terminal::vte::ansi::Color),
    {
        let term = self.term.lock();
        let content = term.renderable_content();
        let grid = term.grid();
        let cols = grid.columns();
        let rows = grid.screen_lines();
        let display_offset = grid.display_offset();

        for item in content.display_iter {
            let cell = item.cell;
            let c = cell.c;
            let fg = cell.fg;

            // Grid coordinates can be negative (history)
            let line_raw = item.point.line.0 as isize;
            let col_raw = item.point.column.0 as isize;

            // Apply display_offset to map grid coordinates to screen coordinates
            // display_offset shifts the view "up" into history.
            // A line at -N in grid becomes (-N + offset) on screen.
            let line = line_raw + display_offset as isize;
            let col = col_raw;

            if line >= 0 && line < rows as isize && col >= 0 && col < cols as isize {
                func(col as usize, line as usize, c, fg);
            }
        }
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        let term = self.term.lock();
        let content = term.renderable_content();
        let cursor = content.cursor;
        (cursor.point.column.0 as usize, cursor.point.line.0 as usize)
    }

    /// Returns (total_lines, view_offset, screen_lines)
    /// view_offset is the number of lines from the bottom of history to the bottom of the viewport.
    /// 0 means we are at the bottom.
    pub fn get_scroll_state(&self) -> (usize, usize, usize) {
        let term = self.term.lock();
        let grid = term.grid();

        let screen_lines = grid.screen_lines();
        let history_size = grid.history_size();
        let total_lines = history_size + screen_lines;

        // Alacritty's display_offset is 0 at the bottom, increasing as we scroll up.
        // It represents how many lines back into history we are viewing.
        let display_offset = grid.display_offset();

        (total_lines, display_offset, screen_lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_accumulator() {
        let emulator = TerminalEmulator::new();

        // Initial state: accumulator is 0.0

        // Scroll 0.4 - should not trigger scroll
        emulator.scroll(0.4);
        {
            let acc = emulator.scroll_accumulator.lock();
            assert!(
                (*acc - 0.4).abs() < 0.0001,
                "Accumulator should be 0.4, got {}",
                *acc
            );
        }

        // Scroll 0.4 again - total 0.8 - still no scroll
        emulator.scroll(0.4);
        {
            let acc = emulator.scroll_accumulator.lock();
            assert!(
                (*acc - 0.8).abs() < 0.0001,
                "Accumulator should be 0.8, got {}",
                *acc
            );
        }

        // Scroll 0.3 - total 1.1 - should trigger scroll of 1, remain 0.1
        emulator.scroll(0.3);
        {
            let acc = emulator.scroll_accumulator.lock();
            assert!(
                (*acc - 0.1).abs() < 0.0001,
                "Accumulator should be 0.1, got {}",
                *acc
            );
        }

        // Scroll negative -0.6 - total -0.5 - no scroll
        emulator.scroll(-0.6);
        {
            let acc = emulator.scroll_accumulator.lock();
            assert!(
                (*acc - -0.5).abs() < 0.0001,
                "Accumulator should be -0.5, got {}",
                *acc
            );
        }

        // Scroll negative -0.6 - total -1.1 - scroll -1, remain -0.1
        emulator.scroll(-0.6);
        {
            let acc = emulator.scroll_accumulator.lock();
            assert!(
                (*acc - -0.1).abs() < 0.0001,
                "Accumulator should be -0.1, got {}",
                *acc
            );
        }
    }
}
