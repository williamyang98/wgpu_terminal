mod utf8_parser;
mod colour_table;
pub mod scrollback_buffer;
mod primitives;
mod terminal_parser;
mod viewport;
pub mod terminal_display;
pub mod terminal_renderer;
mod terminal;

pub use crate::terminal::{
    TerminalIOControl,
    TerminalUserEvent,
    Terminal,
    TerminalBuilder,
};
pub use crate::primitives::{
    Cell,
    StyleFlags,
};
