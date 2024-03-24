use cgmath::Vector2;
use crate::common::{
    BellVolume,
    CharacterSet,
    CursorStyle,
    EraseMode,
    GraphicStyle,
    Rgb8,
    ScreenMode,
    ScrollRegion,
    WindowAction,
};
use crate::encoder::{
    InputMode,
    KeyType,
    MouseCoordinateFormat,
};

#[derive(Clone,Debug,PartialEq)]
pub enum Command {
    // cursor positioning
    MoveCursorUp(u16),
    MoveCursorDown(u16),
    MoveCursorRight(u16),
    MoveCursorLeft(u16),
    MoveCursorReverseIndex,
    SaveCursorToMemory,
    RestoreCursorFromMemory,
    MoveCursorNextLine(u16),
    MoveCursorPreviousLine(u16),
    MoveCursorHorizontalAbsolute(u16),
    MoveCursorVerticalAbsolute(u16),
    MoveCursorPositionViewport(Vector2<u16>),
    // input mode
    SetKeypadMode(InputMode),
    // viewport positioning
    ScrollUp(u16),
    ScrollDown(u16),
    // text modification
    InsertSpaces(u16),
    DeleteCharacters(u16),
    ReplaceWithSpaces(u16),
    InsertLines(u16),
    DeleteLines(u16),
    EraseInDisplay(EraseMode),
    EraseInLine(EraseMode),
    // text formatting
    SetGraphicStyle(GraphicStyle),
    SetForegroundColourTable(u8),
    SetBackgroundColourTable(u8),
    SetForegroundColourRgb(Rgb8),
    SetBackgroundColourRgb(Rgb8),
    // query state
    QueryCursorPosition,
    QueryTerminalIdentity,
    QueryKeyModifierOption(KeyType),
    // tabs
    SetTabStopAtCurrentColumn,
    AdvanceCursorToTabStop(u16),
    ReverseCursorToTabStop(u16),
    ClearCurrentTabStop,
    ClearAllTabStops,
    // designate character set
    SetCharacterSet(CharacterSet),
    // scrolling margins
    SetScrollRegion(Option<ScrollRegion>),
    // operating system command 
    SetHyperlink(String),
    // common private modes
    SetCursorKeyInputMode(InputMode),
    SetConsoleWidth(u16),
    SetLightBackground,
    SetDarkBackground,
    SetCursorBlinking(bool),
    SetCursorVisible(bool),
    SaveScreen,
    RestoreScreen,
    SetReportMouseClick(bool),
    SetHighlightMouseTracking(bool),
    SetCellMouseTracking(bool),
    SetAllMouseTracking(bool),
    SetReportFocus(bool),
    SetMouseCoordinateFormat(MouseCoordinateFormat),
    SetAlternateBuffer(bool),
    SetBracketedPasteMode(bool),
    // screen mode
    SetLineWrapping(bool),
    SetScreenMode(ScreenMode),
    ResetScreenMode(ScreenMode),
    // window
    WindowAction(WindowAction),
    // ESC [ <n> <space>
    ShiftLeftByColumns(u16),
    ShiftRightByColumns(u16),
    SetCursorStyle(CursorStyle),
    SetWarningBellVolume(BellVolume),
    SetMarginBellVolume(BellVolume),
    // ESC [ <n> h/l
    SetKeyboardActionMode(bool),
    SetInsertMode,
    SetReplaceMode,
    SetAutomaticNewline,
    SetNormalLinefeed,
    // modifier keys
    SetKeyModifierOption(KeyType, Option<u16>),
    // soft reset
    SoftReset,
}

