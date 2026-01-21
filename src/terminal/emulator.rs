use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi;
use parking_lot::Mutex;
use std::fmt;
use std::sync::Arc;
use tokio::sync::mpsc;

// Initial terminal size
const DEFAULT_COLS: usize = 80;
const DEFAULT_ROWS: usize = 24;

/// EventListener that forwards terminal output (like cursor position reports) to a channel
#[derive(Clone)]
struct EventWriter {
    tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl EventListener for EventWriter {
    fn send_event(&self, event: Event) {
        match event {
            Event::PtyWrite(ref s) => {
                // Terminal wants to write something back to PTY (e.g., cursor position report)
                let _ = self.tx.send(s.as_bytes().to_vec());
            }
            _ => {
                // Ignore other events for now
            }
        }
    }
}

#[derive(Clone)]
pub struct TerminalEmulator {
    term: Arc<Mutex<Term<EventWriter>>>,
    parser: Arc<Mutex<ansi::Processor>>,
    scroll_accumulator: Arc<Mutex<f32>>,
    selection_start: Option<alacritty_terminal::index::Point>,
    /// Receiver for terminal output responses (like CPR)
    output_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<Vec<u8>>>>>,
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

        let (tx, rx) = mpsc::unbounded_channel();
        let listener = EventWriter { tx };
        let term = Term::new(config, &size, listener);

        Self {
            term: Arc::new(Mutex::new(term)),
            parser: Arc::new(Mutex::new(ansi::Processor::new())),
            scroll_accumulator: Arc::new(Mutex::new(0.0)),
            selection_start: None,
            output_rx: Arc::new(Mutex::new(Some(rx))),
        }
    }

    /// Take the output receiver (should be called once during session setup)
    pub fn take_output_receiver(&self) -> Option<mpsc::UnboundedReceiver<Vec<u8>>> {
        self.output_rx.lock().take()
    }

    /// Process input bytes (from SSH stream)
    pub fn process_input(&mut self, data: &[u8]) {
        let mut term = self.term.lock();
        let mut parser = self.parser.lock();

        // Feed the data to the parser, which updates the terminal state
        // Term implements Handler, so we pass it as the handler.
        parser.advance(&mut *term, data);
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
        F: FnMut(usize, usize, char, alacritty_terminal::vte::ansi::Color, bool),
    {
        let term = self.term.lock();
        let content = term.renderable_content();
        let grid = term.grid();
        let cols = grid.columns();
        let rows = grid.screen_lines();
        let display_offset = grid.display_offset();
        // TERM.selection is a field
        let selection = &term.selection; // Changed from term.selection()

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
                // Attempt to use to_range which is common in older alacritty versions
                let is_selected = selection
                    .as_ref()
                    .and_then(|s| s.to_range(&*term))
                    .map(|range| range.contains(item.point))
                    .unwrap_or(false);
                func(col as usize, line as usize, c, fg, is_selected);
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

    pub fn copy_selection(&self) -> Option<String> {
        let term = self.term.lock();
        term.selection_to_string()
    }

    pub fn on_mouse_double_click(&mut self, col: usize, line: usize) {
        use alacritty_terminal::index::Side;
        use alacritty_terminal::selection::{Selection, SelectionType};

        let mut term = self.term.lock();
        let point = self.viewport_to_point(&term, col, line);
        let side = Side::Left; // Default side
        term.selection = Some(Selection::new(SelectionType::Semantic, point, side));
        self.selection_start = None; // Reset start point to avoid conflict with drag
    }

    pub fn on_mouse_press(&mut self, col: usize, line: usize) {
        let mut term = self.term.lock();
        let point = self.viewport_to_point(&term, col, line);

        // Clear existing selection on press
        term.selection = None;
        self.selection_start = Some(point);
    }

    pub fn on_mouse_drag(&mut self, col: usize, line: usize) {
        use alacritty_terminal::index::Side;
        use alacritty_terminal::selection::{Selection, SelectionType};

        let mut term = self.term.lock();
        let point = self.viewport_to_point(&term, col, line);

        // If no selection exists but we have a start point, create it now (on drag)
        if term.selection.is_none() {
            if let Some(start) = self.selection_start {
                term.selection = Some(Selection::new(SelectionType::Simple, start, Side::Left));
            }
        }

        if let Some(selection) = term.selection.as_mut() {
            selection.update(point, Side::Right);
        }
    }

    pub fn on_mouse_release(&mut self) {
        self.selection_start = None;
    }

    fn viewport_to_point(
        &self,
        term: &Term<EventWriter>,
        col: usize,
        line: usize,
    ) -> alacritty_terminal::index::Point {
        let grid = term.grid();
        let display_offset = grid.display_offset();

        let grid_line = (line as i32) - (display_offset as i32);

        alacritty_terminal::index::Point::new(
            alacritty_terminal::index::Line(grid_line),
            alacritty_terminal::index::Column(col),
        )
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
