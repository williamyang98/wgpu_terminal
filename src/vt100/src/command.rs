use std::num::NonZeroU16;

use crate::{
    misc::{Vector2,EraseMode,ScrollRegion},
    graphic_style::{GraphicStyle,Rgb8},
    screen_mode::{ScreenMode},
    key_input::KeyInput,
};

#[derive(Clone,Copy,Debug,PartialEq)]
pub enum Command<'a> {
    // cursor positioning
    MoveCursorUp(NonZeroU16),
    MoveCursorDown(NonZeroU16),
    MoveCursorRight(NonZeroU16),
    MoveCursorLeft(NonZeroU16),
    MoveCursorReverseIndex,
    SaveCursorToMemory,
    RestoreCursorFromMemory,
    MoveCursorNextLine(NonZeroU16),
    MoveCursorPreviousLine(NonZeroU16),
    MoveCursorHorizontalAbsolute(NonZeroU16),
    MoveCursorVerticalAbsolute(NonZeroU16),
    MoveCursorPositionViewport(Vector2<NonZeroU16>),
    // cursor visibility
    EnableCursorBlinking,
    DisableCursorBlinking,
    ShowCursor,
    HideCursor,
    // viewport positioning
    ScrollUp(NonZeroU16),
    ScrollDown(NonZeroU16),
    // text modification
    InsertSpaces(NonZeroU16),
    DeleteCharacters(NonZeroU16),
    ReplaceWithSpaces(NonZeroU16),
    InsertLines(NonZeroU16),
    DeleteLines(NonZeroU16),
    EraseInDisplay(EraseMode),
    EraseInLine(EraseMode),
    // text formatting
    SetGraphicStyles(&'a [GraphicStyle]),
    SetForegroundColourTable(u8),
    SetBackgroundColourTable(u8),
    SetForegroundColourRgb(Rgb8),
    SetBackgroundColourRgb(Rgb8),
    // mode changes
    EnableKeypadApplicationMode,
    EnableKeypadNumericMode,
    EnableCursorKeysApplicationMode,
    EnableCursorKeysNumericMode,
    // query state
    QueryCursorPosition,
    QueryTerminalIdentity,
    // tabs
    SetTabStopAtCurrentColumn,
    AdvanceCursorToTabStop(NonZeroU16),
    ReverseCursorToTabStop(NonZeroU16),
    ClearCurrentTabStop,
    ClearAllTabStops,
    // designate character set
    EnableLineDrawingMode,
    EnableAsciiMode,
    // scrolling margins
    SetScrollRegion(Option<ScrollRegion>),
    // operating system command 
    SetWindowTitle(&'a [u8]),
    SetHyperlink { tag: &'a [u8], link: &'a [u8] },
    // alternate screen buffer
    EnableAlternateBuffer,
    DisableAlternateBuffer,
    SaveScreen,
    RestoreScreen,
    SetScreenMode(ScreenMode),
    ResetScreenMode(ScreenMode),
    EnableLineWrapping,
    DisableLineWrapping,
    // window width
    SetConsoleWidth(NonZeroU16),
    // soft reset
    SoftReset,
    // input sequences
    KeyInput(KeyInput),
}

