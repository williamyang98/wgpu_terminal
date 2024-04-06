use crate::{
    scrollback_buffer::ScrollbackBuffer,
    viewport::Viewport, 
    primitives::Pen,
};
use cgmath::Vector2;
use vt100::common::CursorStyle;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CursorStatus {
    pub is_visible: bool,
    pub is_blinking: bool,
    pub style: CursorStyle,
}

impl Default for CursorStatus {
    fn default() -> Self {
        Self {
            is_visible: true,
            is_blinking: true,
            style: CursorStyle::Block,
        }
    }
}

#[derive(Clone,Debug)]
pub struct TerminalDisplay {
    primary_viewport: Viewport,
    alternate_viewport: Viewport,
    is_alternate_viewport: bool,
    size: Vector2<usize>,
    pub(crate) cursor_status: CursorStatus,
}

impl Default for TerminalDisplay {
    fn default() -> Self {
        let mut primary_viewport = Viewport::default();
        primary_viewport.scrollback_buffer = Some(ScrollbackBuffer::default());
        Self {
            size: Vector2::new(1,1),
            cursor_status: CursorStatus::default(),
            primary_viewport,
            alternate_viewport: Viewport::default(),
            is_alternate_viewport: false,
        }
    }
}

impl TerminalDisplay {
    pub(crate) fn set_is_newline_carriage_return(&mut self, is_newline_carriage_return: bool) {
        self.primary_viewport.is_newline_carriage_return = is_newline_carriage_return;
        self.alternate_viewport.is_newline_carriage_return = is_newline_carriage_return;
    }

    pub(crate) fn set_default_pen(&mut self, pen: Pen) {
        self.primary_viewport.default_pen = pen;
        self.alternate_viewport.default_pen = pen;
    }

    pub(crate) fn set_size(&mut self, size: Vector2<usize>) {
        self.size = size;
        let viewport = self.get_current_viewport_mut();
        viewport.set_size(size);
    }

    pub(crate) fn set_is_alternate(&mut self, is_alternate: bool) {
        self.is_alternate_viewport = is_alternate;
        let size = self.size;
        let viewport = self.get_current_viewport_mut();
        viewport.set_size(size);
    }

    pub(crate) fn get_current_viewport_mut(&mut self) -> &mut Viewport {
        if self.is_alternate_viewport {
            &mut self.alternate_viewport
        } else {
            &mut self.primary_viewport
        }
    }

    pub(crate) fn get_current_viewport(&self) -> &Viewport {
        if self.is_alternate_viewport {
            &self.alternate_viewport
        } else {
            &self.primary_viewport
        }
    }
}
