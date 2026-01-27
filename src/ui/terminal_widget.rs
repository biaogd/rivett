use iced::mouse;
use iced::widget::canvas::{self, Cache, Canvas, Frame, Geometry, Text};
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};

use crate::terminal::TerminalEmulator;
use crate::ui::Message;

pub const BASE_CELL_WIDTH: f32 = 7.2;
pub const BASE_CELL_HEIGHT: f32 = 16.0;

pub fn cell_width(font_size: f32) -> f32 {
    BASE_CELL_WIDTH * (font_size / 12.0)
}

pub fn cell_height(font_size: f32) -> f32 {
    BASE_CELL_HEIGHT * (font_size / 12.0)
}

pub struct TerminalView<'a> {
    emulator: TerminalEmulator,
    chrome_cache: &'a Cache,
    line_caches: &'a [Cache],
    preedit: Option<&'a str>,
    font_size: f32,
}

impl<'a> TerminalView<'a> {
    pub fn new(
        emulator: TerminalEmulator,
        chrome_cache: &'a Cache,
        line_caches: &'a [Cache],
        preedit: Option<&'a str>,
        font_size: f32,
    ) -> Self {
        Self {
            emulator,
            chrome_cache,
            line_caches,
            preedit,
            font_size,
        }
    }

    pub fn view(self) -> Element<'a, Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

pub struct TerminalWidgetState {
    is_dragging: bool,
    last_click_time: Option<std::time::Instant>,
}

impl Default for TerminalWidgetState {
    fn default() -> Self {
        Self {
            is_dragging: false,
            last_click_time: None,
        }
    }
}

impl<'a> canvas::Program<Message> for TerminalView<'a> {
    type State = TerminalWidgetState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::event::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<iced::widget::canvas::Action<Message>> {
        if let iced::event::Event::Mouse(mouse_event) = event {
            // Need cell position
            // But if we release OUTSIDE bounds, we still need to stop drag.
            // So ButtonReleased should be handled regardless of bounds?
            // "If cursor is over bounds"
            // The Canvas `update` is only called if events are relevant?
            // Actually `update` is called for all events if the widget is active.

            // To be safe, calculate col/line if over bounds.

            let is_over = cursor.is_over(bounds);

            match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if is_over {
                        if let Some(position) = cursor.position_in(bounds) {
                            let col = (position.x / cell_width(self.font_size)) as usize;
                            let line = (position.y / cell_height(self.font_size)) as usize;

                            // let mut emulator = self.emulator.clone();

                            // Check for double click
                            let now = std::time::Instant::now();
                            if let Some(last_click) = state.last_click_time {
                                if now.duration_since(last_click).as_millis() < 500 {
                                    // Double click!
                                    // emulator.on_mouse_double_click(col, line);
                                    state.is_dragging = true;
                                    state.last_click_time = None; // Reset
                                    // self.cache.clear();
                                    return Some(iced::widget::canvas::Action::publish(
                                        Message::TerminalMouseDoubleClick(col, line),
                                    ));
                                }
                            }

                            // Single click
                            // emulator.on_mouse_press(col, line);
                            state.is_dragging = true;
                            state.last_click_time = Some(now);

                            // self.cache.clear();
                            return Some(iced::widget::canvas::Action::publish(
                                Message::TerminalMousePress(col, line),
                            ));
                        }
                    }
                }
                mouse::Event::CursorMoved { .. } => {
                    if state.is_dragging && is_over {
                        if let Some(position) = cursor.position_in(bounds) {
                            let col = (position.x / cell_width(self.font_size)) as usize;
                            let line = (position.y / cell_height(self.font_size)) as usize;

                            // let mut emulator = self.emulator.clone();
                            // emulator.on_mouse_drag(col, line);
                            // self.cache.clear();
                            return Some(iced::widget::canvas::Action::publish(
                                Message::TerminalMouseDrag(col, line),
                            ));
                        }
                    }
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    if state.is_dragging {
                        // let mut emulator = self.emulator.clone();
                        // emulator.on_mouse_release();
                        state.is_dragging = false;
                        // self.cache.clear();
                        return Some(iced::widget::canvas::Action::publish(
                            Message::TerminalMouseRelease,
                        ));
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut geometries = Vec::new();

        let chrome = self.chrome_cache.draw(renderer, bounds.size(), |frame| {
            // Fill background
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::WHITE);

            // Draw Scrollbar
            let (total_lines, display_offset, screen_lines) = self.emulator.get_scroll_state();
            if total_lines > screen_lines {
                let scrollbar_width = 10.0;
                let track_x = bounds.width - scrollbar_width;
                let track_height = bounds.height;
                let max_offset = total_lines.saturating_sub(screen_lines);
                let scroll_fraction = if max_offset > 0 {
                    1.0 - (display_offset as f32 / max_offset as f32)
                } else {
                    1.0
                };
                let thumb_fraction = (screen_lines as f32 / total_lines as f32).max(0.05);
                let thumb_height = track_height * thumb_fraction;
                let thumb_y = available_track(track_height, thumb_height) * scroll_fraction;

                frame.fill_rectangle(
                    Point::new(track_x, 0.0),
                    Size::new(scrollbar_width, track_height),
                    Color::from_rgba8(200, 200, 200, 0.2),
                );
                frame.fill_rectangle(
                    Point::new(track_x, thumb_y),
                    Size::new(scrollbar_width, thumb_height),
                    Color::from_rgba8(100, 100, 100, 0.5),
                );
            }

            // FPS Counter
            use std::sync::Mutex;
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::time::Instant;

            static FRAME_COUNT: AtomicUsize = AtomicUsize::new(0);
            static LAST_SECOND: std::sync::OnceLock<Mutex<Instant>> = std::sync::OnceLock::new();

            let last_second_mutex = LAST_SECOND.get_or_init(|| Mutex::new(Instant::now()));
            let mut last_second = last_second_mutex.lock().unwrap();

            FRAME_COUNT.fetch_add(1, Ordering::Relaxed);

            if last_second.elapsed().as_secs() >= 1 {
                let count = FRAME_COUNT.swap(0, Ordering::Relaxed);
                println!("FPS: {}", count);
                *last_second = Instant::now();
            }
        });
        geometries.push(chrome);

        let cell_width = cell_width(self.font_size);
        let cell_height = cell_height(self.font_size);
        let terminal_font_family = crate::platform::default_terminal_font_family();
        let (cursor_col, cursor_row) = self.emulator.cursor_position();
        let preedit_len = self.preedit.map(|s| s.chars().count()).unwrap_or(0);
        let (_, _, screen_lines) = self.emulator.get_scroll_state();
        let visible_lines = screen_lines.min(self.line_caches.len());

        for line in 0..visible_lines {
            let cache = &self.line_caches[line];
            let geometry = cache.draw(renderer, bounds.size(), |frame| {
                // --- Batched Text Rendering (per line) ---
                let mut current_text = String::new();
                let mut current_fg = Color::BLACK;
                let mut start_pos = Point::ORIGIN;
                let mut last_col = -1;

                self.emulator
                    .render_line(line, |col, _line, cell, is_selected| {
                        use alacritty_terminal::term::cell::Flags;

                        // Skip wide char spacers (the second half of a wide char)
                        if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                            return;
                        }

                        let c = cell.c;
                        let fg = cell.fg;
                        let bg = cell.bg;

                        let x = col as f32 * cell_width;
                        let y = line as f32 * cell_height;
                        let mut fg_color = convert_color(fg);
                        let mut bg_color = convert_color(bg);
                        if cell.flags.contains(Flags::INVERSE) {
                            std::mem::swap(&mut fg_color, &mut bg_color);
                        }

                        // Only render selection background for non-space characters
                        // For wide chars, we might want to render background for double width?
                        // But since we skip spacer, we should probably render double width here if it's a wide char?
                        // Alternatively, just render single width here, and since we skip the next col,
                        // we won't draw background there?
                        // Actually, if we skip render for spacer, we won't draw background for the second half.
                        // We should probably draw double width background for WIDE_CHAR.
                        let width = if cell.flags.contains(Flags::WIDE_CHAR) {
                            cell_width * 2.0
                        } else {
                            cell_width
                        };

                        let selection_bg = Color::from_rgba8(100, 100, 200, 0.5);
                        let should_draw_bg =
                            is_selected || bg_color != Color::WHITE;
                        if should_draw_bg {
                            frame.fill_rectangle(
                                Point::new(x, y),
                                Size::new(width, cell_height),
                                if is_selected { selection_bg } else { bg_color },
                            );
                        }

                        let break_span =
                            fg_color != current_fg || col as i32 != last_col + 1;
                        if break_span && !current_text.is_empty() {
                            frame.fill_text(Text {
                                content: current_text.clone(),
                                position: start_pos,
                                color: current_fg,
                                size: self.font_size.into(),
                                font: iced::Font {
                                    family: iced::font::Family::Name(terminal_font_family),
                                    ..iced::Font::DEFAULT
                                },
                                ..Text::default()
                            });
                            current_text.clear();
                        }

                        if current_text.is_empty() {
                            start_pos = Point::new(x, y);
                            current_fg = fg_color;
                        }

                        current_text.push(c);

                        // If wide char, we still just mark this col.
                        // The next col is spacer and will be skipped.
                        // So last_col will lag behind by 1 when we hit the Char after the spacer.
                        // e.g. Wide at 0. Spacer at 1. Next char at 2.
                        // Draw 0. last_col=0.
                        // Skip 1.
                        // Draw 2. break_span check: 2 != 0 + 1. True. Span breaks.
                        // This is actually FINE. It means wide chars might break batching, but that's safe.
                        last_col = col as i32;
                    });

                if !current_text.is_empty() {
                    frame.fill_text(Text {
                        content: current_text,
                        position: start_pos,
                        color: current_fg,
                        size: self.font_size.into(),
                        font: iced::Font {
                            family: iced::font::Family::Name(terminal_font_family),
                            ..iced::Font::DEFAULT
                        },
                        ..Text::default()
                    });
                }
            });

            geometries.push(geometry);
        }

        let mut overlay = Frame::new(renderer, bounds.size());
        let cursor_x = (cursor_col + preedit_len) as f32 * cell_width;
        let cursor_y = cursor_row as f32 * cell_height;

        overlay.fill_rectangle(
            Point::new(cursor_x, cursor_y),
            Size::new(cell_width, cell_height),
            Color::from_rgba8(0, 0, 0, 0.3),
        );

        if let Some(preedit) = self.preedit {
            if !preedit.is_empty() {
                let text_width = preedit.chars().count().max(1) as f32 * cell_width;

                overlay.fill_text(Text {
                    content: preedit.to_string(),
                    position: Point::new(cursor_col as f32 * cell_width, cursor_y),
                    color: Color::from_rgb8(30, 64, 175),
                    size: self.font_size.into(),
                    font: iced::Font {
                        family: iced::font::Family::Name(terminal_font_family),
                        ..iced::Font::DEFAULT
                    },
                    ..Text::default()
                });

                overlay.fill_rectangle(
                    Point::new(cursor_col as f32 * cell_width, cursor_y + cell_height - 2.0),
                    Size::new(text_width, 1.0),
                    Color::from_rgb8(30, 64, 175),
                );
            }
        }

        geometries.push(overlay.into_geometry());

        geometries
    }
}

// Helper to avoid lifetime issues in closure
fn available_track(track_h: f32, thumb_h: f32) -> f32 {
    track_h - thumb_h
}

fn convert_color(color: alacritty_terminal::vte::ansi::Color) -> Color {
    use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};

    match color {
        AnsiColor::Named(named) => match named {
            NamedColor::Black => Color::BLACK,
            NamedColor::Red => Color::from_rgb8(205, 49, 49),
            NamedColor::Green => Color::from_rgb8(13, 188, 121),
            NamedColor::Yellow => Color::from_rgb8(180, 160, 0), // Darker yellow for visibility
            NamedColor::Blue => Color::from_rgb8(36, 114, 200),
            NamedColor::Magenta => Color::from_rgb8(188, 63, 188),
            NamedColor::Cyan => Color::from_rgb8(0, 150, 200), // Darker cyan
            NamedColor::White => Color::from_rgb8(240, 240, 240),
            NamedColor::Foreground => Color::BLACK, // Default text is black
            NamedColor::Background => Color::WHITE,
            _ => Color::BLACK,
        },
        AnsiColor::Spec(rgb) => Color::from_rgb8(rgb.r, rgb.g, rgb.b),
        AnsiColor::Indexed(_idx) => {
            // Basic 256 color mapping logic or fallback
            Color::from_rgb8(50, 50, 50)
        }
    }
}
