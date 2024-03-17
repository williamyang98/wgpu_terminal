use terminal::terminal_display::TerminalDisplay;
use terminal::terminal_parser::TerminalParserHandler;
use vt100::{
    command::Command as Vt100Command,
    misc::{EraseMode,Vector2},
    graphic_style::{Rgb8, GraphicStyle},
};
use cgmath::Vector2 as Vec2;
use std::num::NonZeroU16;
use test_log::test;

fn create_cursor(x: u16, y: u16) -> Vector2<NonZeroU16> {
    // (1,1) is the (top,left) origin for Vt100 terminals
    Vector2::new(
        NonZeroU16::new(x+1).unwrap(), 
        NonZeroU16::new(y+1).unwrap(),
    )
}

fn create_default_display() -> TerminalDisplay {
    let mut display = TerminalDisplay::default();
    let default_size: Vec2<usize> = Vec2::new(48,8);
    display.get_viewport_mut().set_size(default_size);
    display
}

fn check_display_matches(display: &TerminalDisplay, source: &str) {
    let viewport = display.get_viewport();
    let size = viewport.get_size();
    let lines: Vec<&str> = source.split("\n").collect();
    if lines.len() > size.y {
        panic!("Reference string has more lines than display: ({}>{})", lines.len(), size.y);
    }
    let total_lines = lines.len();
    let mut is_line_correct: Vec<bool> = vec![false; total_lines];
    let mut total_mismatch: usize = 0;
    for ((y, correct_line), is_correct) in (0..total_lines).zip(lines.iter()).zip(is_line_correct.iter_mut()) {
        let (row, status) = viewport.get_row(y);
        let given_line: String = row[..status.length].iter().map(|c| c.character).collect();
        *is_correct = given_line.as_str() == *correct_line;
        if !*is_correct {
            total_mismatch += 1;
        }
    }
    if total_mismatch > 0 {
        log::error!("{} lines did not match", total_mismatch);
        for ((y, correct_line), is_correct) in (0..total_lines).zip(lines.iter()).zip(is_line_correct.iter()) {
            let (row, status) = viewport.get_row(y);
            let given_line: String = row[..status.length].iter().map(|c| c.character).collect();
            log::error!("{:>2},{:>2} | {}", y, status.length, given_line);
            if !is_correct {
                log::error!("  ,{:>2} : {}", correct_line.len(), correct_line);
            }
        }
        panic!("Viewport did not match expected string");
    }
}

fn check_cursor_matches(display: &TerminalDisplay, pos: Vec2<usize>) {
    let cursor = display.get_viewport().get_cursor();
    if cursor != pos {
        panic!("Cursor position expected to be ({},{}) but got ({},{})", pos.x, pos.y, cursor.x, cursor.y);
    }
}

#[test]
fn write_to_display() {
    let mut display = create_default_display();
    let test_line: &str = "abcdefghijklmnopqrstuvwxyz0123456789";
    display.on_ascii_data(test_line.as_bytes());
    check_display_matches(&display, test_line);
    check_cursor_matches(&display, Vec2::new(test_line.len(),0));
}

#[test]
fn write_to_display_multline() {
    let mut display = create_default_display();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n";
    display.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&display, test_lines);
    check_cursor_matches(&display, Vec2::new(0,3));
}

#[test]
fn move_cursor() {
    let mut display = create_default_display();
    let size = display.get_viewport().get_size();
    for stride in 1..size.y {
        let pos = stride;
        let stride = NonZeroU16::new(stride as u16).unwrap();
        display.on_vt100(&Vt100Command::MoveCursorDown(stride));
        check_cursor_matches(&display, Vec2::new(0,pos));
        display.on_vt100(&Vt100Command::MoveCursorUp(stride));
        check_cursor_matches(&display, Vec2::new(0,0));
        display.on_vt100(&Vt100Command::MoveCursorRight(stride));
        check_cursor_matches(&display, Vec2::new(pos,0));
        display.on_vt100(&Vt100Command::MoveCursorLeft(stride));
        check_cursor_matches(&display, Vec2::new(0,0));
    }
}

#[test]
fn move_cursor_past_limits() {
    let mut display = create_default_display();
    let size = display.get_viewport().get_size();
    display.on_vt100(&Vt100Command::MoveCursorDown(NonZeroU16::new(2*size.y as u16).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,size.y-1));
    display.on_vt100(&Vt100Command::MoveCursorUp(NonZeroU16::new(2*size.y as u16).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,0));
    // cursor.x can overflow until change is committed
    display.on_vt100(&Vt100Command::MoveCursorRight(NonZeroU16::new(2*size.x as u16).unwrap()));
    check_cursor_matches(&display, Vec2::new(size.x,0));
    display.on_vt100(&Vt100Command::MoveCursorLeft(NonZeroU16::new(2*size.x as u16).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,0));
}

#[test]
fn replace_entire_line_with_spaces() {
    let mut display = create_default_display();
    let test_line = b"abcdefghijklmnopqrstuvwxyz0123456789";
    display.on_ascii_data(test_line);
    display.on_vt100(&Vt100Command::MoveCursorPositionViewport(create_cursor(0,0)));
    display.on_vt100(&Vt100Command::ReplaceWithSpaces(NonZeroU16::new(test_line.len() as u16).unwrap()));
    check_display_matches(&display, "                                    ");
    check_cursor_matches(&display, Vec2::new(0,0));
}

#[test]
fn replace_partial_line_with_spaces() {
    let mut display = create_default_display();
    let test_line = b"abcdefghijklmnopqrstuvwxyz0123456789";
    display.on_ascii_data(test_line);
    display.on_vt100(&Vt100Command::MoveCursorPositionViewport(create_cursor(0,0)));
    let erase_length: usize = 16;
    display.on_vt100(&Vt100Command::ReplaceWithSpaces(NonZeroU16::new(erase_length as u16).unwrap()));
    check_display_matches(&display, "                qrstuvwxyz0123456789");
    check_cursor_matches(&display, Vec2::new(0,0));
}

#[test]
fn insert_lines() {
    let mut display = create_default_display();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n";
    display.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&display, test_lines);

    check_cursor_matches(&display, Vec2::new(0,4));
    display.on_vt100(&Vt100Command::MoveCursorUp(NonZeroU16::new(3).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,1));
    display.on_vt100(&Vt100Command::InsertLines(NonZeroU16::new(2).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,1));
    let test_lines: &str = "\
        line 0: abc\n\
        \n\
        \n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n";
    check_display_matches(&display, test_lines);

    display.on_vt100(&Vt100Command::MoveCursorDown(NonZeroU16::new(3).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,4));
    display.on_vt100(&Vt100Command::InsertLines(NonZeroU16::new(2).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,4));
    let test_lines: &str = "\
        line 0: abc\n\
        \n\
        \n\
        line 1: 123\n\
        \n\
        \n\
        line 2: 345\n\
        line 3: def";
    check_display_matches(&display, test_lines);
}

#[test]
fn delete_lines() {
    let mut display = create_default_display();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n\
        line 4: 789\n\
        line 5: ghi\n\
        line 6: @#$\n";
    display.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&display, test_lines);

    check_cursor_matches(&display, Vec2::new(0,7));
    display.on_vt100(&Vt100Command::MoveCursorUp(NonZeroU16::new(3).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,4));
    display.on_vt100(&Vt100Command::DeleteLines(NonZeroU16::new(1).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,4));
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n\
        line 5: ghi\n\
        line 6: @#$\n\n";
    check_display_matches(&display, test_lines);

    display.on_vt100(&Vt100Command::MoveCursorUp(NonZeroU16::new(3).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,1));
    display.on_vt100(&Vt100Command::DeleteLines(NonZeroU16::new(4).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,1));
    let test_lines: &str = "\
        line 0: abc\n\
        line 6: @#$\n\n\n\n\n";
    check_display_matches(&display, test_lines);

    display.on_vt100(&Vt100Command::MoveCursorUp(NonZeroU16::new(1).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,0));
    display.on_vt100(&Vt100Command::DeleteLines(NonZeroU16::new(1).unwrap()));
    check_cursor_matches(&display, Vec2::new(0,0));
    let test_lines: &str = "\
        line 6: @#$\n\n\n\n\n\n";
    check_display_matches(&display, test_lines);
}


#[test]
fn write_with_scrollback() {
    let mut display = create_default_display();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n\
        line 4: 789\n\
        line 5: ghi\n\
        line 6: @#$\n";
    display.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&display, test_lines);
    check_cursor_matches(&display, Vec2::new(0,7));

    let new_line = "line 7: Get out of my way";
    display.on_ascii_data(new_line.as_bytes());
    check_cursor_matches(&display, Vec2::new(new_line.len(),7));

    display.on_ascii_data(b"\n");
    let test_lines: &str = "\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n\
        line 4: 789\n\
        line 5: ghi\n\
        line 6: @#$\n\
        line 7: Get out of my way";
    check_display_matches(&display, test_lines);
    check_cursor_matches(&display, Vec2::new(0,7));

    let scrollback_buffer = display.get_viewport().get_scrollback_buffer();
    let lines = scrollback_buffer.get_lines();
    // scrollback buffer has a pending line segment for consuming new ejected lines
    assert!(lines.len() == 2);
    let line = scrollback_buffer.get_row(&lines[0]);
    let line_string: String = line.iter().map(|c| c.character).collect();
    assert!(line_string.as_str() == "line 0: abc");
}

#[test]
fn save_restore_cursor_from_memory() {
    let mut display = create_default_display();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345";
    let expected_save_cursor: Vec2<usize> = Vec2::new(11,2);
    display.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&display, test_lines);
    check_cursor_matches(&display, expected_save_cursor);

    display.on_vt100(&Vt100Command::SaveCursorToMemory);
    let test_lines: &str = "\
        def-continue\n\
        line 3: ghi\n\
        line 4: 456\n";
    display.on_ascii_data(test_lines.as_bytes());
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345def-continue\n\
        line 3: ghi\n\
        line 4: 456\n";
    check_display_matches(&display, test_lines);
    check_cursor_matches(&display, Vec2::new(0,5));

    display.on_vt100(&Vt100Command::RestoreCursorFromMemory);
    check_display_matches(&display, test_lines);
    check_cursor_matches(&display, expected_save_cursor);

    display.on_ascii_data(b"456@");
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345456@continue\n\
        line 3: ghi\n\
        line 4: 456\n";
    check_display_matches(&display, test_lines);
    check_cursor_matches(&display, Vec2::new(15,2));
}
