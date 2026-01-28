use alacritty_terminal::vte::ansi::CursorShape;
use iced::font::{Style as FontStyle, Weight as FontWeight};
use iced::mouse;
use iced::widget::canvas::{self, Cache, Canvas, Frame, Geometry, Text};
use iced::widget::text::LineHeight;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};
use unicode_width::UnicodeWidthChar;

use crate::terminal::TerminalEmulator;
use crate::ui::Message;
use crate::ui::terminal_colors::convert_color;

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
    hover_link: Option<String>,
}

impl Default for TerminalWidgetState {
    fn default() -> Self {
        Self {
            is_dragging: false,
            last_click_time: None,
            hover_link: None,
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
                        if let Some(link) = state.hover_link.clone() {
                            return Some(iced::widget::canvas::Action::publish(Message::OpenUrl(
                                link,
                            )));
                        }
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
                    } else if is_over {
                        if let Some(position) = cursor.position_in(bounds) {
                            let col = (position.x / cell_width(self.font_size)) as usize;
                            let line = (position.y / cell_height(self.font_size)) as usize;
                            state.hover_link = self.emulator.hyperlink_at(col, line);
                        }
                    } else {
                        state.hover_link = None;
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
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            if state.hover_link.is_some() {
                return mouse::Interaction::Pointer;
            }
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
        let fallback_font_family = crate::platform::terminal_fallback_family();
        let (cursor_col, cursor_row, cursor_shape, cursor_rgb) = self.emulator.cursor_render_info();
        let preedit_len = self.preedit.map(display_width).unwrap_or(0);
        let (_, _, screen_lines) = self.emulator.get_scroll_state();
        let visible_lines = screen_lines.min(self.line_caches.len());

        for line in 0..visible_lines {
            let cache = &self.line_caches[line];
            let geometry = cache.draw(renderer, bounds.size(), |frame| {
                // --- Batched Text Rendering (per line) ---
                let mut current_text = String::new();
                let mut current_fg = Color::BLACK;
                let mut current_weight = FontWeight::Normal;
                let mut current_style = FontStyle::Normal;
                let mut current_family = terminal_font_family;
                let mut start_pos = Point::ORIGIN;
                let mut last_col = -1;

                self.emulator
                    .render_line(line, |col, _line, cell, is_selected| {
                        use alacritty_terminal::term::cell::Flags;

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
                        if cell.flags.contains(Flags::DIM) {
                            fg_color = Color {
                                a: fg_color.a * 0.6,
                                ..fg_color
                            };
                        }

                        let weight = if cell.flags.contains(Flags::BOLD) {
                            FontWeight::Bold
                        } else {
                            FontWeight::Normal
                        };
                        let style = if cell.flags.contains(Flags::ITALIC) {
                            FontStyle::Italic
                        } else {
                            FontStyle::Normal
                        };
                        let family = if c.is_ascii() {
                            terminal_font_family
                        } else {
                            fallback_font_family
                        };

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
                        let should_draw_bg = is_selected || bg_color != Color::WHITE;
                        if should_draw_bg {
                            frame.fill_rectangle(
                                Point::new(x, y),
                                Size::new(width, cell_height),
                                if is_selected { selection_bg } else { bg_color },
                            );
                        }

                        // Skip drawing text for spacer half, but keep background above.
                        if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                            last_col = col as i32;
                            return;
                        }

                        let break_span = fg_color != current_fg
                            || weight != current_weight
                            || style != current_style
                            || family != current_family
                            || col as i32 != last_col + 1;
                        if break_span && !current_text.is_empty() {
                            frame.fill_text(Text {
                                content: current_text.clone(),
                                position: start_pos,
                                color: current_fg,
                                size: self.font_size.into(),
                                font: iced::Font {
                                    family: iced::font::Family::Name(current_family),
                                    weight: current_weight,
                                    style: current_style,
                                    ..iced::Font::DEFAULT
                                },
                                ..Text::default()
                            });
                            current_text.clear();
                        }

                        if !c.is_ascii() {
                            let glyph_cells = UnicodeWidthChar::width(c).unwrap_or(1) as f32;
                            let glyph_width = glyph_cells * cell_width;
                            frame.fill_text(Text {
                                content: c.to_string(),
                                position: Point::new(x, y),
                                color: fg_color,
                                size: self.font_size.into(),
                                font: iced::Font {
                                    family: iced::font::Family::Name(family),
                                    weight,
                                    style,
                                    ..iced::Font::DEFAULT
                                },
                                max_width: glyph_width,
                                align_x: iced::alignment::Horizontal::Left.into(),
                                line_height: LineHeight::Absolute(iced::Pixels(cell_height)),
                                ..Text::default()
                            });
                            last_col = col as i32;
                            return;
                        }

                        if current_text.is_empty() {
                            start_pos = Point::new(x, y);
                            current_fg = fg_color;
                            current_weight = weight;
                            current_style = style;
                            current_family = family;
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
                        if cell.flags.contains(Flags::UNDERLINE) {
                            frame.fill_rectangle(
                                Point::new(x, y + cell_height - 2.0),
                                Size::new(width, 1.0),
                                fg_color,
                            );
                        }
                        if cell.flags.contains(Flags::STRIKEOUT) {
                            frame.fill_rectangle(
                                Point::new(x, y + cell_height / 2.0),
                                Size::new(width, 1.0),
                                fg_color,
                            );
                        }

                        last_col = col as i32;
                    });

                if !current_text.is_empty() {
                    frame.fill_text(Text {
                        content: current_text,
                        position: start_pos,
                        color: current_fg,
                        size: self.font_size.into(),
                        font: iced::Font {
                            family: iced::font::Family::Name(current_family),
                            weight: current_weight,
                            style: current_style,
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
        let cursor_color = cursor_rgb
            .map(|rgb| Color::from_rgb8(rgb.r, rgb.g, rgb.b))
            .unwrap_or(Color::from_rgba8(0, 0, 0, 0.5));

        match cursor_shape {
            CursorShape::Hidden => {}
            CursorShape::Block => {
                overlay.fill_rectangle(
                    Point::new(cursor_x, cursor_y),
                    Size::new(cell_width, cell_height),
                    Color {
                        a: 0.4,
                        ..cursor_color
                    },
                );
            }
            CursorShape::Underline => {
                overlay.fill_rectangle(
                    Point::new(cursor_x, cursor_y + cell_height - 2.0),
                    Size::new(cell_width, 2.0),
                    cursor_color,
                );
            }
            CursorShape::Beam => {
                overlay.fill_rectangle(
                    Point::new(cursor_x, cursor_y),
                    Size::new(2.0, cell_height),
                    cursor_color,
                );
            }
            CursorShape::HollowBlock => {
                let line = 1.0;
                overlay.fill_rectangle(
                    Point::new(cursor_x, cursor_y),
                    Size::new(cell_width, line),
                    cursor_color,
                );
                overlay.fill_rectangle(
                    Point::new(cursor_x, cursor_y + cell_height - line),
                    Size::new(cell_width, line),
                    cursor_color,
                );
                overlay.fill_rectangle(
                    Point::new(cursor_x, cursor_y),
                    Size::new(line, cell_height),
                    cursor_color,
                );
                overlay.fill_rectangle(
                    Point::new(cursor_x + cell_width - line, cursor_y),
                    Size::new(line, cell_height),
                    cursor_color,
                );
            }
        }

        if let Some(preedit) = self.preedit {
            if !preedit.is_empty() {
                let text_width = display_width(preedit).max(1) as f32 * cell_width;
                let preedit_family = if preedit.chars().any(|c| !c.is_ascii()) {
                    fallback_font_family
                } else {
                    terminal_font_family
                };

                overlay.fill_text(Text {
                    content: preedit.to_string(),
                    position: Point::new(cursor_col as f32 * cell_width, cursor_y),
                    color: Color::from_rgb8(30, 64, 175),
                    size: self.font_size.into(),
                    font: iced::Font {
                        family: iced::font::Family::Name(preedit_family),
                        ..iced::Font::DEFAULT
                    },
                    max_width: bounds.width,
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

fn display_width(text: &str) -> usize {
    text.chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(1))
        .sum()
}
