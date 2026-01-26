use iced::Task;

use crate::ui::{App, Message};
use crate::ui::terminal_widget;

impl App {
    pub(in crate::ui) fn focus_terminal_ime(&self) -> Task<Message> {
        iced::widget::operation::focus(self.ime_input_id.clone())
    }

    pub(in crate::ui) fn cell_width(&self) -> f32 {
        terminal_widget::cell_width(self.terminal_font_size)
    }

    pub(in crate::ui) fn cell_height(&self) -> f32 {
        terminal_widget::cell_height(self.terminal_font_size)
    }

    pub(in crate::ui) fn recalc_terminal_size(&self) -> Task<Message> {
        let width = self.window_width;
        let height = self.window_height;
        if width == 0 || height == 0 {
            return Task::none();
        }

        let reserved_width = 0.0;
        let h_padding = 24.0;
        let v_padding = 120.0;

        let term_w = (width as f32 - reserved_width - h_padding).max(0.0);
        let term_h = (height as f32 - v_padding).max(0.0);

        let cols = (term_w / self.cell_width()) as usize;
        let rows = (term_h / self.cell_height()) as usize;

        Task::done(Message::TerminalResize(cols, rows))
    }

    pub(in crate::ui) fn bracketed_paste_bytes(&self, text: &str) -> Vec<u8> {
        let mut data = Vec::with_capacity(text.len() + 12);
        data.extend_from_slice(b"\x1b[200~");
        data.extend_from_slice(text.as_bytes());
        data.extend_from_slice(b"\x1b[201~");
        data
    }

    pub(in crate::ui) fn maybe_wrap_bracketed_paste(&self, data: &[u8]) -> Vec<u8> {
        if data.contains(&b'\n') && !data.windows(6).any(|w| w == b"\x1b[200~") {
            let mut wrapped = Vec::with_capacity(data.len() + 12);
            wrapped.extend_from_slice(b"\x1b[200~");
            wrapped.extend_from_slice(data);
            wrapped.extend_from_slice(b"\x1b[201~");
            wrapped
        } else {
            data.to_vec()
        }
    }
}
