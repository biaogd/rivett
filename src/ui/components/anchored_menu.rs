use iced::advanced::layout;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget;
use iced::advanced::widget::tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::{Element, Length, Point, Rectangle, Size, Vector, mouse};

pub fn anchored_menu<'a, Message, Theme, Renderer>(
    target: impl Into<Element<'a, Message, Theme, Renderer>>,
    menu: impl Into<Element<'a, Message, Theme, Renderer>>,
    open: bool,
    gap: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a + 'static,
    Renderer: renderer::Renderer + 'a,
    Message: 'a,
{
    Element::new(AnchoredMenu {
        target: target.into(),
        menu: menu.into(),
        open,
        gap,
    })
}

struct AnchoredMenu<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    target: Element<'a, Message, Theme, Renderer>,
    menu: Element<'a, Message, Theme, Renderer>,
    open: bool,
    gap: f32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for AnchoredMenu<'_, Message, Theme, Renderer>
where
    Theme: 'static,
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        self.target.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.target.as_widget().size_hint()
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.target), tree::Tree::new(&self.menu)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(&[&self.target, &self.menu]);
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.target
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.target.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.target.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.target.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        _offset: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let target_bounds = layout.bounds();
        if !self.open {
            return self.target.as_widget_mut().overlay(
                &mut tree.children[0],
                layout,
                renderer,
                viewport,
                _offset,
            );
        }

        let menu_overlay = overlay::Element::new(Box::new(MenuOverlay {
            menu: &mut self.menu,
            tree: &mut tree.children[1],
            target_bounds,
            gap: self.gap,
        }));
        Some(menu_overlay)
    }
}

struct MenuOverlay<'a, 'b, Message, Theme, Renderer> {
    menu: &'b mut Element<'a, Message, Theme, Renderer>,
    tree: &'b mut tree::Tree,
    target_bounds: Rectangle,
    gap: f32,
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for MenuOverlay<'_, '_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let limits =
            layout::Limits::new(Size::ZERO, bounds).width(Length::Fixed(self.target_bounds.width));
        let menu_layout = self
            .menu
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);
        let node = menu_layout.move_to(Point::new(
            self.target_bounds.x,
            self.target_bounds.y + self.target_bounds.height + self.gap,
        ));

        node
    }

    fn update(
        &mut self,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        self.menu.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &Rectangle::with_size(layout.bounds().size()),
        );
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.menu.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let viewport = layout.bounds();
        self.menu
            .as_widget()
            .mouse_interaction(self.tree, layout, cursor, &viewport, renderer)
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.menu
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }
}
