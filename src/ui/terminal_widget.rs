use iced::mouse;
use iced::widget::canvas::{self, Cache, Canvas, Geometry, Text};
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};

use crate::terminal::TerminalEmulator;
use crate::ui::Message;

pub const CELL_WIDTH: f32 = 7.5;
pub const CELL_HEIGHT: f32 = 16.0;

pub struct TerminalView<'a> {
    emulator: TerminalEmulator,
    cache: &'a Cache,
}

impl<'a> TerminalView<'a> {
    pub fn new(emulator: TerminalEmulator, cache: &'a Cache) -> Self {
        Self { emulator, cache }
    }

    pub fn view(self) -> Element<'a, Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message> for TerminalView<'a> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &iced::event::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<iced::widget::canvas::Action<Message>> {
        // We rely on the global subscription in App for scroll events
        // to avoid duplicate handling and state conflicts.
        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            // Fill background
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::WHITE);

            let cell_width = CELL_WIDTH;
            let cell_height = CELL_HEIGHT;

            // Draw Cursor
            let (cursor_col, cursor_row) = self.emulator.cursor_position();
            let cursor_x = cursor_col as f32 * cell_width;
            let cursor_y = cursor_row as f32 * cell_height;

            frame.fill_rectangle(
                Point::new(cursor_x, cursor_y),
                Size::new(cell_width, cell_height),
                Color::from_rgba8(0, 0, 0, 0.3), // Semi-transparent cursor
            );

            self.emulator.render_grid(|col, line, c, fg| {
                let x = col as f32 * cell_width;
                let y = line as f32 * cell_height;

                let color = convert_color(fg);

                frame.fill_text(Text {
                    content: c.to_string(),
                    position: Point::new(x, y),
                    color,
                    size: 12.0.into(),
                    font: iced::Font::MONOSPACE,
                    ..Text::default()
                });
            });

            // Draw Scrollbar
            let (total_lines, display_offset, screen_lines) = self.emulator.get_scroll_state();
            if total_lines > screen_lines {
                let scrollbar_width = 10.0;
                let track_x = bounds.width - scrollbar_width;
                let track_height = bounds.height;

                // Viewport position in history:
                // display_offset = 0 means bottom (end of history)
                // display_offset = total_lines - screen_lines means top
                //
                // We want:
                // Top of scrollbar = top of viewport

                // Let's visualize:
                // total_lines = 1000, screen_lines = 24.
                // display_offset = 0 (bottom). Viewport is [976..1000]. Scrollbar should be at bottom.
                // display_offset = 976 (top). Viewport is [0..24]. Scrollbar should be at top.

                let max_offset = total_lines.saturating_sub(screen_lines);
                let scroll_fraction = if max_offset > 0 {
                    1.0 - (display_offset as f32 / max_offset as f32)
                } else {
                    1.0
                };

                // Thumb height proportional to view size
                let thumb_fraction = (screen_lines as f32 / total_lines as f32).max(0.05); // Min 5% height
                let thumb_height = track_height * thumb_fraction;

                // Track ranges from 0 to (track_height - thumb_height)
                let available_track = track_height - thumb_height;
                let thumb_y = available_track * scroll_fraction;

                // Draw Track Background (optional, maybe just transparent)
                frame.fill_rectangle(
                    Point::new(track_x, 0.0),
                    Size::new(scrollbar_width, track_height),
                    Color::from_rgba8(200, 200, 200, 0.2),
                );

                // Draw Thumb
                frame.fill_rectangle(
                    Point::new(track_x, thumb_y),
                    Size::new(scrollbar_width, thumb_height),
                    Color::from_rgba8(100, 100, 100, 0.5),
                );
            }
        });

        vec![geometry]
    }
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
