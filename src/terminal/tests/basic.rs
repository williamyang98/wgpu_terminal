



use terminal::terminal_parser::TerminalParserHandler;


use vt100::{
    command::Command as Vt100Command,
};
use cgmath::Vector2;
use test_log::test;

mod test_terminal;
use test_terminal::TestTerminal;

fn create_default_terminal() -> TestTerminal {
    let default_size: Vector2<usize> = Vector2::new(48,8);
    let mut terminal = TestTerminal::default();
    terminal.set_size(default_size);
    terminal
}

fn check_display_matches(terminal: &TestTerminal, source: &str) {
    let display = terminal.get_display();
    let viewport = display.get_viewport();
    let size = viewport.get_size();
    let lines: Vec<&str> = source.split('\n').collect();
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
        log::error!("cursor={:?}", viewport.get_cursor());
        log::error!("size={:?}", viewport.get_size());
        panic!("Viewport did not match expected string");
    }
}

fn check_cursor_matches(terminal: &TestTerminal, pos: Vector2<usize>) {
    let display = terminal.get_display();
    let viewport = display.get_viewport();
    let cursor = viewport.get_cursor();
    if cursor != pos {
        panic!("Cursor position expected to be ({},{}) but got ({},{})", pos.x, pos.y, cursor.x, cursor.y);
    }
}

#[test]
fn write_to_display() {
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let test_line: &str = "abcdefghijklmnopqrstuvwxyz0123456789";
    core.on_ascii_data(test_line.as_bytes());
    check_display_matches(&terminal, test_line);
    check_cursor_matches(&terminal, Vector2::new(test_line.len(),0));
}

#[test]
fn write_to_display_multline() {
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n";
    core.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&terminal, test_lines);
    check_cursor_matches(&terminal, Vector2::new(0,3));
}

#[test]
fn move_cursor() {
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let size = terminal.get_display().get_viewport().get_size();
    for stride in 1..size.y {
        let pos = stride;
        let stride = stride as u16;
        core.on_vt100(Vt100Command::MoveCursorDown(stride));
        check_cursor_matches(&terminal, Vector2::new(0,pos));
        core.on_vt100(Vt100Command::MoveCursorUp(stride));
        check_cursor_matches(&terminal, Vector2::new(0,0));
        core.on_vt100(Vt100Command::MoveCursorRight(stride));
        check_cursor_matches(&terminal, Vector2::new(pos,0));
        core.on_vt100(Vt100Command::MoveCursorLeft(stride));
        check_cursor_matches(&terminal, Vector2::new(0,0));
    }
}

#[test]
fn move_cursor_past_limits() {
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let size = terminal.get_display().get_viewport().get_size();
    core.on_vt100(Vt100Command::MoveCursorDown(2*size.y as u16));
    check_cursor_matches(&terminal, Vector2::new(0,size.y-1));
    core.on_vt100(Vt100Command::MoveCursorUp(2*size.y as u16));
    check_cursor_matches(&terminal, Vector2::new(0,0));
    // cursor.x can overflow until change is committed
    core.on_vt100(Vt100Command::MoveCursorRight(2*size.x as u16));
    check_cursor_matches(&terminal, Vector2::new(size.x,0));
    core.on_vt100(Vt100Command::MoveCursorLeft(2*size.x as u16));
    check_cursor_matches(&terminal, Vector2::new(0,0));
}

#[test]
fn replace_entire_line_with_spaces() {
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let test_line = b"abcdefghijklmnopqrstuvwxyz0123456789";
    core.on_ascii_data(test_line);
    core.on_vt100(Vt100Command::MoveCursorPositionViewport(Vector2::new(1,1)));
    core.on_vt100(Vt100Command::ReplaceWithSpaces(test_line.len() as u16));
    check_display_matches(&terminal, "                                    ");
    check_cursor_matches(&terminal, Vector2::new(0,0));
}

#[test]
fn replace_partial_line_with_spaces() {
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let test_line = b"abcdefghijklmnopqrstuvwxyz0123456789";
    core.on_ascii_data(test_line);
    core.on_vt100(Vt100Command::MoveCursorPositionViewport(Vector2::new(1,1)));
    let erase_length = 16;
    core.on_vt100(Vt100Command::ReplaceWithSpaces(erase_length));
    check_display_matches(&terminal, "                qrstuvwxyz0123456789");
    check_cursor_matches(&terminal, Vector2::new(0,0));
}

#[test]
fn insert_lines() {
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n";
    core.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&terminal, test_lines);

    check_cursor_matches(&terminal, Vector2::new(0,4));
    core.on_vt100(Vt100Command::MoveCursorUp(3));
    check_cursor_matches(&terminal, Vector2::new(0,1));
    core.on_vt100(Vt100Command::InsertLines(2));
    check_cursor_matches(&terminal, Vector2::new(0,1));
    let test_lines: &str = "\
        line 0: abc\n\
        \n\
        \n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n";
    check_display_matches(&terminal, test_lines);

    core.on_vt100(Vt100Command::MoveCursorDown(3));
    check_cursor_matches(&terminal, Vector2::new(0,4));
    core.on_vt100(Vt100Command::InsertLines(2));
    check_cursor_matches(&terminal, Vector2::new(0,4));
    let test_lines: &str = "\
        line 0: abc\n\
        \n\
        \n\
        line 1: 123\n\
        \n\
        \n\
        line 2: 345\n\
        line 3: def";
    check_display_matches(&terminal, test_lines);
}

#[test]
fn delete_lines() {
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n\
        line 4: 789\n\
        line 5: ghi\n\
        line 6: @#$\n";
    core.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&terminal, test_lines);

    check_cursor_matches(&terminal, Vector2::new(0,7));
    core.on_vt100(Vt100Command::MoveCursorUp(3));
    check_cursor_matches(&terminal, Vector2::new(0,4));
    core.on_vt100(Vt100Command::DeleteLines(1));
    check_cursor_matches(&terminal, Vector2::new(0,4));
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n\
        line 5: ghi\n\
        line 6: @#$\n\n";
    check_display_matches(&terminal, test_lines);

    core.on_vt100(Vt100Command::MoveCursorUp(3));
    check_cursor_matches(&terminal, Vector2::new(0,1));
    core.on_vt100(Vt100Command::DeleteLines(4));
    check_cursor_matches(&terminal, Vector2::new(0,1));
    let test_lines: &str = "\
        line 0: abc\n\
        line 6: @#$\n\n\n\n\n";
    check_display_matches(&terminal, test_lines);

    core.on_vt100(Vt100Command::MoveCursorUp(1));
    check_cursor_matches(&terminal, Vector2::new(0,0));
    core.on_vt100(Vt100Command::DeleteLines(1));
    check_cursor_matches(&terminal, Vector2::new(0,0));
    let test_lines: &str = "\
        line 6: @#$\n\n\n\n\n\n";
    check_display_matches(&terminal, test_lines);
}


#[test]
fn write_with_scrollback() {
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n\
        line 4: 789\n\
        line 5: ghi\n\
        line 6: @#$\n";
    core.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&terminal, test_lines);
    check_cursor_matches(&terminal, Vector2::new(0,7));

    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n\
        line 4: 789\n\
        line 5: ghi\n\
        line 6: @#$\n\
        line 7: Get out of my way";
    let new_line = "line 7: Get out of my way";
    core.on_ascii_data(new_line.as_bytes());
    check_display_matches(&terminal, test_lines);
    check_cursor_matches(&terminal, Vector2::new(new_line.len(),7));

    let test_lines: &str = "\
        line 1: 123\n\
        line 2: 345\n\
        line 3: def\n\
        line 4: 789\n\
        line 5: ghi\n\
        line 6: @#$\n\
        line 7: Get out of my way\n";
    core.on_ascii_data(b"\n");
    check_display_matches(&terminal, test_lines);
    check_cursor_matches(&terminal, Vector2::new(0,7));

    let display = terminal.get_display();
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
    let terminal = create_default_terminal();
    let mut core = terminal.create_core();
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345";
    let expected_save_cursor: Vector2<usize> = Vector2::new(11,2);
    core.on_ascii_data(test_lines.as_bytes());
    check_display_matches(&terminal, test_lines);
    check_cursor_matches(&terminal, expected_save_cursor);

    core.on_vt100(Vt100Command::SaveCursorToMemory);
    let test_lines: &str = "\
        def-continue\n\
        line 3: ghi\n\
        line 4: 456\n";
    core.on_ascii_data(test_lines.as_bytes());
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345def-continue\n\
        line 3: ghi\n\
        line 4: 456\n";
    check_display_matches(&terminal, test_lines);
    check_cursor_matches(&terminal, Vector2::new(0,5));

    core.on_vt100(Vt100Command::RestoreCursorFromMemory);
    check_display_matches(&terminal, test_lines);
    check_cursor_matches(&terminal, expected_save_cursor);

    core.on_ascii_data(b"456@");
    let test_lines: &str = "\
        line 0: abc\n\
        line 1: 123\n\
        line 2: 345456@continue\n\
        line 3: ghi\n\
        line 4: 456\n";
    check_display_matches(&terminal, test_lines);
    check_cursor_matches(&terminal, Vector2::new(15,2));
}
