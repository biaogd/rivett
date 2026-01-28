use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::CursorShape;
use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::renderer::Renderer as _;
use iced::advanced::text;
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::font::{Style as FontStyle, Weight as FontWeight};
use iced::mouse;
use iced::{Background, Border, Color, Element, Length, Pixels, Point, Rectangle, Size};
use unicode_width::UnicodeWidthChar;

use crate::terminal::TerminalEmulator;
use crate::ui::Message;
use crate::ui::terminal_colors::convert_color;
use crate::ui::terminal_widget::{cell_height, cell_width};

pub struct TerminalGpuView<'a> {
    emulator: TerminalEmulator,
    preedit: Option<&'a str>,
    font_size: f32,
}

impl<'a> TerminalGpuView<'a> {
    pub fn new(emulator: TerminalEmulator, preedit: Option<&'a str>, font_size: f32) -> Self {
        Self {
            emulator,
            preedit,
            font_size,
        }
    }

    pub fn view(self) -> Element<'a, Message> {
        Element::new(self)
    }
}

#[derive(Default)]
struct TerminalGpuState {
    is_dragging: bool,
    last_click_time: Option<std::time::Instant>,
    hover_link: Option<String>,
}

impl Widget<Message, iced::Theme, iced::Renderer> for TerminalGpuView<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn size_hint(&self) -> Size<Length> {
        self.size()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TerminalGpuState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(TerminalGpuState::default())
    }

    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<TerminalGpuState>();
        let bounds = layout.bounds();
        if let iced::Event::Mouse(mouse_event) = event {
            let is_over = cursor.is_over(bounds);
            match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if is_over {
                        if let Some(link) = state.hover_link.clone() {
                            shell.publish(Message::OpenUrl(link));
                            return;
                        }
                        if let Some(position) = cursor.position_in(bounds) {
                            let col = (position.x / cell_width(self.font_size)) as usize;
                            let line = (position.y / cell_height(self.font_size)) as usize;
                            let now = std::time::Instant::now();
                            if let Some(last_click) = state.last_click_time {
                                if now.duration_since(last_click).as_millis() < 500 {
                                    state.is_dragging = true;
                                    state.last_click_time = None;
                                    shell.publish(Message::TerminalMouseDoubleClick(col, line));
                                    return;
                                }
                            }
                            state.is_dragging = true;
                            state.last_click_time = Some(now);
                            shell.publish(Message::TerminalMousePress(col, line));
                        }
                    }
                }
                mouse::Event::CursorMoved { .. } => {
                    if state.is_dragging && is_over {
                        if let Some(position) = cursor.position_in(bounds) {
                            let col = (position.x / cell_width(self.font_size)) as usize;
                            let line = (position.y / cell_height(self.font_size)) as usize;
                            shell.publish(Message::TerminalMouseDrag(col, line));
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
                        state.is_dragging = false;
                        shell.publish(Message::TerminalMouseRelease);
                    }
                }
                _ => {}
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        if cursor.is_over(bounds) {
            let state = tree.state.downcast_ref::<TerminalGpuState>();
            if state.hover_link.is_some() {
                return mouse::Interaction::Pointer;
            }
            return mouse::Interaction::Text;
        }
        mouse::Interaction::default()
    }

    fn draw(
        &self,
        _tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let cell_w = cell_width(self.font_size);
        let cell_h = cell_height(self.font_size);
        let terminal_font_family = crate::platform::default_terminal_font_family();
        let fallback_font_family = crate::platform::terminal_fallback_family();

        let clip_bounds = bounds.intersection(viewport).unwrap_or(bounds);

        fill_rect(renderer, bounds, Color::WHITE);

        let (total_lines, display_offset, screen_lines) = self.emulator.get_scroll_state();
        if total_lines > screen_lines {
            let scrollbar_width = 10.0;
            let track_x = bounds.x + bounds.width - scrollbar_width;
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

            fill_rect(
                renderer,
                Rectangle::new(
                    Point::new(track_x, bounds.y),
                    Size::new(scrollbar_width, track_height),
                ),
                Color::from_rgba8(200, 200, 200, 0.2),
            );
            fill_rect(
                renderer,
                Rectangle::new(
                    Point::new(track_x, bounds.y + thumb_y),
                    Size::new(scrollbar_width, thumb_height),
                ),
                Color::from_rgba8(100, 100, 100, 0.5),
            );
        }

        let (cursor_col, cursor_row, cursor_shape, cursor_rgb) =
            self.emulator.cursor_render_info();
        let preedit_len = self.preedit.map(display_width).unwrap_or(0);
        let visible_lines = screen_lines;

        for line in 0..visible_lines {
            let mut current_text = String::new();
            let mut current_fg = Color::BLACK;
            let mut current_weight = FontWeight::Normal;
            let mut current_style = FontStyle::Normal;
            let mut current_family = terminal_font_family;
            let mut start_x = 0.0;
            let mut last_col = -1;

            self.emulator
                .render_line(line, |col, _line, cell, is_selected| {
                    let c = cell.c;
                    let mut fg = convert_color(cell.fg);
                    let mut bg = convert_color(cell.bg);
                    if cell.flags.contains(Flags::INVERSE) {
                        std::mem::swap(&mut fg, &mut bg);
                    }
                    if cell.flags.contains(Flags::DIM) {
                        fg = Color {
                            a: fg.a * 0.6,
                            ..fg
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

                    let x = bounds.x + col as f32 * cell_w;
                    let y = bounds.y + line as f32 * cell_h;
                    let width = if cell.flags.contains(Flags::WIDE_CHAR) {
                        cell_w * 2.0
                    } else {
                        cell_w
                    };

                    let selection_bg = Color::from_rgba8(100, 100, 200, 0.5);
                    let should_draw_bg = is_selected || bg != Color::WHITE;
                    if should_draw_bg {
                        fill_rect(
                            renderer,
                            Rectangle::new(Point::new(x, y), Size::new(width, cell_h)),
                            if is_selected { selection_bg } else { bg },
                        );
                    }

                    if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                        last_col = col as i32;
                        return;
                    }

                    let break_span = fg != current_fg
                        || weight != current_weight
                        || style != current_style
                        || family != current_family
                        || col as i32 != last_col + 1;
                    if break_span && !current_text.is_empty() {
                        let text_bounds = Size::new(bounds.width, cell_h);
                        renderer.fill_text(
                            text::Text {
                                content: current_text.clone(),
                                bounds: text_bounds,
                                size: self.font_size.into(),
                                line_height: text::LineHeight::Absolute(Pixels(cell_h)),
                                font: iced::Font {
                                    family: iced::font::Family::Name(current_family),
                                    weight: current_weight,
                                    style: current_style,
                                    ..iced::Font::DEFAULT
                                },
                                align_x: text::Alignment::Left,
                                align_y: iced::alignment::Vertical::Top,
                                shaping: text::Shaping::Basic,
                                wrapping: text::Wrapping::None,
                            },
                            Point::new(start_x, y),
                            current_fg,
                            clip_bounds,
                        );
                        current_text.clear();
                    }

                    if !c.is_ascii() {
                        let glyph_cells = UnicodeWidthChar::width(c).unwrap_or(1) as f32;
                        let glyph_width = glyph_cells * cell_w;
                        renderer.fill_text(
                            text::Text {
                                content: c.to_string(),
                                bounds: Size::new(glyph_width, cell_h),
                                size: self.font_size.into(),
                                line_height: text::LineHeight::Absolute(Pixels(cell_h)),
                                font: iced::Font {
                                    family: iced::font::Family::Name(family),
                                    weight,
                                    style,
                                    ..iced::Font::DEFAULT
                                },
                                align_x: text::Alignment::Left,
                                align_y: iced::alignment::Vertical::Top,
                                shaping: text::Shaping::Basic,
                                wrapping: text::Wrapping::None,
                            },
                            Point::new(x, y),
                            fg,
                            clip_bounds,
                        );
                        last_col = col as i32;
                        return;
                    }

                    if current_text.is_empty() {
                        start_x = x;
                        current_fg = fg;
                        current_weight = weight;
                        current_style = style;
                        current_family = family;
                    }

                    current_text.push(c);

                    if cell.flags.contains(Flags::UNDERLINE) {
                        fill_rect(
                            renderer,
                            Rectangle::new(
                                Point::new(x, y + cell_h - 2.0),
                                Size::new(width, 1.0),
                            ),
                            fg,
                        );
                    }
                    if cell.flags.contains(Flags::STRIKEOUT) {
                        fill_rect(
                            renderer,
                            Rectangle::new(
                                Point::new(x, y + cell_h / 2.0),
                                Size::new(width, 1.0),
                            ),
                            fg,
                        );
                    }

                    last_col = col as i32;
                });

            if !current_text.is_empty() {
                let text_bounds = Size::new(bounds.width, cell_h);
                renderer.fill_text(
                    text::Text {
                        content: current_text,
                        bounds: text_bounds,
                        size: self.font_size.into(),
                        line_height: text::LineHeight::Absolute(Pixels(cell_h)),
                        font: iced::Font {
                            family: iced::font::Family::Name(current_family),
                            weight: current_weight,
                            style: current_style,
                            ..iced::Font::DEFAULT
                        },
                        align_x: text::Alignment::Left,
                        align_y: iced::alignment::Vertical::Top,
                        shaping: text::Shaping::Basic,
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(start_x, bounds.y + line as f32 * cell_h),
                    current_fg,
                    clip_bounds,
                );
            }
        }

        let cursor_x = bounds.x + (cursor_col + preedit_len) as f32 * cell_w;
        let cursor_y = bounds.y + cursor_row as f32 * cell_h;
        let cursor_color = cursor_rgb
            .map(|rgb| Color::from_rgb8(rgb.r, rgb.g, rgb.b))
            .unwrap_or(Color::from_rgba8(0, 0, 0, 0.5));

        match cursor_shape {
            CursorShape::Hidden => {}
            CursorShape::Block => fill_rect(
                renderer,
                Rectangle::new(Point::new(cursor_x, cursor_y), Size::new(cell_w, cell_h)),
                Color {
                    a: 0.4,
                    ..cursor_color
                },
            ),
            CursorShape::Underline => fill_rect(
                renderer,
                Rectangle::new(
                    Point::new(cursor_x, cursor_y + cell_h - 2.0),
                    Size::new(cell_w, 2.0),
                ),
                cursor_color,
            ),
            CursorShape::Beam => fill_rect(
                renderer,
                Rectangle::new(Point::new(cursor_x, cursor_y), Size::new(2.0, cell_h)),
                cursor_color,
            ),
            CursorShape::HollowBlock => {
                let line = 1.0;
                fill_rect(
                    renderer,
                    Rectangle::new(Point::new(cursor_x, cursor_y), Size::new(cell_w, line)),
                    cursor_color,
                );
                fill_rect(
                    renderer,
                    Rectangle::new(
                        Point::new(cursor_x, cursor_y + cell_h - line),
                        Size::new(cell_w, line),
                    ),
                    cursor_color,
                );
                fill_rect(
                    renderer,
                    Rectangle::new(Point::new(cursor_x, cursor_y), Size::new(line, cell_h)),
                    cursor_color,
                );
                fill_rect(
                    renderer,
                    Rectangle::new(
                        Point::new(cursor_x + cell_w - line, cursor_y),
                        Size::new(line, cell_h),
                    ),
                    cursor_color,
                );
            }
        }

        if let Some(preedit) = self.preedit {
            if !preedit.is_empty() {
                let text_width = display_width(preedit).max(1) as f32 * cell_w;
                let preedit_family = if preedit.chars().any(|c| !c.is_ascii()) {
                    fallback_font_family
                } else {
                    terminal_font_family
                };
                renderer.fill_text(
                    text::Text {
                        content: preedit.to_string(),
                        bounds: Size::new(bounds.width, cell_h),
                        size: self.font_size.into(),
                        line_height: text::LineHeight::Absolute(Pixels(cell_h)),
                        font: iced::Font {
                            family: iced::font::Family::Name(preedit_family),
                            ..iced::Font::DEFAULT
                        },
                        align_x: text::Alignment::Left,
                        align_y: iced::alignment::Vertical::Top,
                        shaping: text::Shaping::Basic,
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(bounds.x + cursor_col as f32 * cell_w, cursor_y),
                    Color::from_rgb8(30, 64, 175),
                    clip_bounds,
                );
                fill_rect(
                    renderer,
                    Rectangle::new(
                        Point::new(
                            bounds.x + cursor_col as f32 * cell_w,
                            cursor_y + cell_h - 2.0,
                        ),
                        Size::new(text_width, 1.0),
                    ),
                    Color::from_rgb8(30, 64, 175),
                );
            }
        }
    }
}

fn fill_rect(renderer: &mut iced::Renderer, bounds: Rectangle, color: Color) {
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: Border::default(),
            shadow: iced::Shadow::default(),
            snap: true,
        },
        Background::Color(color),
    );
}

fn available_track(track_h: f32, thumb_h: f32) -> f32 {
    track_h - thumb_h
}

fn display_width(text: &str) -> usize {
    text.chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(1))
        .sum()
}
