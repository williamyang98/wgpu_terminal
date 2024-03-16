pub mod parser;
pub mod command;
pub mod graphic_style;
pub mod misc;
pub mod screen_mode;
pub mod key_input;

#[cfg(test)]
mod tests {
    use crate::{
        command::Command,
        parser::{Parser,ParserError},
        misc::{Vector2,EraseMode,ScrollRegion,CharacterSet,InputMode},
        screen_mode::ScreenMode,
        graphic_style::{GraphicStyle,Rgb8},
    };
    use std::num::NonZeroU16;

    const MAX_VALUE: u16 = 32767;

    fn generate_sample_values() -> Vec<u16> {
        let mut values = Vec::<u16>::with_capacity(512);
        values.extend(0..256u16);
        values.push(1024);
        values.push(2048);
        values.push(4096);
        values.push(8192);
        values.push(16384);
        values.extend(MAX_VALUE..MAX_VALUE+256);
        values
    }

    fn test_valid_sequence<'a>(seq: &[u8], command: Command) {
        let mut parser = Parser::default(); 
        for (i,&b) in seq.iter().enumerate() {
            let res = parser.feed_byte(b);
            if i < seq.len()-1 {
                if res != Err(ParserError::Pending) {
                    panic!(
                        "[error] Parser ended early at index {}\n  \
                           Sequence: {:?} ({:?})\n  \
                           Expected: {:?}\n  \
                           Got: {:?}",
                        i, seq, std::str::from_utf8(seq), ParserError::Pending, res,
                    );
                }
            } else {
                if res != Ok(command) {
                    panic!(
                        "[error] Parser didnt get valid command at index {}\n  \
                           Sequence: {:?} ({:?})\n  \
                           Expected: {:?}\n  \
                           Got: {:?}",
                        i, seq, std::str::from_utf8(seq), command, res,
                    );
                }
            }
        }
    }

    fn test_invalid_sequence(seq: &[u8], index: Option<usize>, error: ParserError) {
        let index = index.unwrap_or(seq.len().max(1)-1);
        let mut parser = Parser::default();
        for (i,&b) in seq.iter().enumerate() {
            let res = parser.feed_byte(b);
            if i == index {
                if res != Err(error) {
                    panic!(
                        "[error] Parser didnt get expected error at index {}\n  \
                           Sequence: {:?} ({:?})\n  \
                           Expected: {:?}\n  \
                           Got: {:?}",
                        i, seq, std::str::from_utf8(seq), error, res,
                    );
                }
            } else {
                if res != Err(ParserError::Pending) { 
                    panic!(
                        "[error] Parser ended early at index {}\n  \
                           Sequence: {:?} ({:?})\n  \
                           Expected: {:?}\n  \
                           Got: {:?}",
                        i, seq, std::str::from_utf8(seq), ParserError::Pending, res,
                    );
                }
            }
        }
    }

    #[test]
    fn valid_single_move_cursor() {
        let n_default = NonZeroU16::new(1).unwrap();
        test_valid_sequence(b"A", Command::MoveCursorUp(n_default));
        test_valid_sequence(b"B", Command::MoveCursorDown(n_default));
        test_valid_sequence(b"C", Command::MoveCursorRight(n_default));
        test_valid_sequence(b"D", Command::MoveCursorLeft(n_default));
        test_valid_sequence(b"M", Command::MoveCursorUp(n_default));
        test_valid_sequence(b"[A", Command::MoveCursorUp(n_default));
        test_valid_sequence(b"[B", Command::MoveCursorDown(n_default));
        test_valid_sequence(b"[C", Command::MoveCursorRight(n_default));
        test_valid_sequence(b"[D", Command::MoveCursorLeft(n_default));
        test_valid_sequence(b"[E", Command::MoveCursorNextLine(n_default));
        test_valid_sequence(b"[F", Command::MoveCursorPreviousLine(n_default));
        test_valid_sequence(b"[G", Command::MoveCursorHorizontalAbsolute(n_default));
        test_valid_sequence(b"[d", Command::MoveCursorVerticalAbsolute(n_default));
    }
 
    #[test]
    fn valid_multiple_move_cursor() {
        let values = generate_sample_values();
        for &v in &values {
            let n = NonZeroU16::new(v.clamp(1, MAX_VALUE)).unwrap();
            test_valid_sequence(format!("[{}A", v).as_bytes(), Command::MoveCursorUp(n));
            test_valid_sequence(format!("[{}B", v).as_bytes(), Command::MoveCursorDown(n));
            test_valid_sequence(format!("[{}C", v).as_bytes(), Command::MoveCursorRight(n));
            test_valid_sequence(format!("[{}D", v).as_bytes(), Command::MoveCursorLeft(n));
            test_valid_sequence(format!("[{}E", v).as_bytes(), Command::MoveCursorNextLine(n));
            test_valid_sequence(format!("[{}F", v).as_bytes(), Command::MoveCursorPreviousLine(n));
            test_valid_sequence(format!("[{}G", v).as_bytes(), Command::MoveCursorHorizontalAbsolute(n));
            test_valid_sequence(format!("[{}d", v).as_bytes(), Command::MoveCursorVerticalAbsolute(n));
        }
    }

    #[test]
    fn valid_move_xy_cursor() {
        let values = generate_sample_values();
        for &x in &values {
            for &y in &values {
                let x_actual = x.min(MAX_VALUE).max(1);
                let y_actual = y.min(MAX_VALUE).max(1);
                let x_actual = NonZeroU16::new(x_actual).unwrap();
                let y_actual = NonZeroU16::new(y_actual).unwrap();
                let command = Command::MoveCursorPositionViewport(Vector2::new(x_actual,y_actual));
                test_valid_sequence(format!("[{};{}H", y, x).as_bytes(), command);
                test_valid_sequence(format!("[{};{}f", y, x).as_bytes(), command);
            }
        }
        let default_v = NonZeroU16::new(1).unwrap();
        let default_v = Vector2::new(default_v, default_v);
        test_valid_sequence(b"[H", Command::MoveCursorPositionViewport(default_v));
    }

    #[test]
    fn valid_private_modes_nonstandard() {
        test_valid_sequence(b"=", Command::SetKeypadMode(InputMode::Application));
        test_valid_sequence(b">", Command::SetKeypadMode(InputMode::Numeric));
        test_valid_sequence(b"[?1h", Command::SetCursorKeysMode(InputMode::Application));
        test_valid_sequence(b"[?1l", Command::SetCursorKeysMode(InputMode::Numeric));
        test_valid_sequence(b"[?3h", Command::SetConsoleWidth(NonZeroU16::new(132).unwrap()));
        test_valid_sequence(b"[?3l", Command::SetConsoleWidth(NonZeroU16::new(80).unwrap()));
        test_valid_sequence(b"[?12h", Command::SetCursorBlinking(true));
        test_valid_sequence(b"[?12l", Command::SetCursorBlinking(false));
        test_valid_sequence(b"[?25h", Command::SetCursorVisible(true));
        test_valid_sequence(b"[?25l", Command::SetCursorVisible(false));
        test_valid_sequence(b"[?47h", Command::SaveScreen);
        test_valid_sequence(b"[?47l", Command::RestoreScreen);
        test_valid_sequence(b"[?1049h", Command::SetAlternateBuffer(true));
        test_valid_sequence(b"[?1049l", Command::SetAlternateBuffer(false));
    }

    #[test]
    fn valid_soft_reset() {
        test_valid_sequence(b"[!p", Command::SoftReset);
    }

    #[test]
    fn valid_set_screen_title() {
        let window_titles = ["", "hello world", "ĐđĒēĔĕĖėĘęĚěĜĝĞğ"];
        for window_title in &window_titles {
            let command = Command::SetWindowTitle(window_title.as_bytes());
            test_valid_sequence(format!("]0;{}\x07", window_title).as_bytes(), command);
            test_valid_sequence(format!("]2;{}\x07", window_title).as_bytes(), command);
        }
    }

    #[test]
    fn valid_set_colour_from_table() {
        for v in 0..=260 {
            let actual_value = v.min(255) as u8;
            test_valid_sequence(format!("[38;5;{}m",v).as_bytes(), Command::SetForegroundColourTable(actual_value)); 
            test_valid_sequence(format!("[48;5;{}m",v).as_bytes(), Command::SetBackgroundColourTable(actual_value)); 
        }
    }

    #[test]
    fn valid_set_colour_rgb() {
        for r in 0..=260 {
            for g in (0..=260).step_by(20) {
                for b in (0..=260).step_by(20) {
                    const MAX_VALUE: u16 = 255;
                    let colour = Rgb8 { 
                        r: r.min(MAX_VALUE) as u8,
                        g: g.min(MAX_VALUE) as u8,
                        b: b.min(MAX_VALUE) as u8,
                    };
                    test_valid_sequence(format!("[38;2;{};{};{}m",r,g,b).as_bytes(), Command::SetForegroundColourRgb(colour)); 
                    test_valid_sequence(format!("[48;2;{};{};{}m",r,g,b).as_bytes(), Command::SetBackgroundColourRgb(colour)); 
                }
            }
        }
    }

    #[test]
    fn valid_set_default_graphic_style() {
        let default_style = GraphicStyle::try_from_u16(0).unwrap();
        test_valid_sequence(b"[m", Command::SetGraphicStyles(&[default_style]));
    }

    #[test]
    fn valid_set_single_graphic_style() {
        for v in 0..255 {
            if let Some(style) = GraphicStyle::try_from_u16(v) {
                test_valid_sequence(format!("[{}m",v).as_bytes(), Command::SetGraphicStyles(&[style]));
            }
        }
    }

    #[test]
    fn valid_set_multiple_graphic_styles() {
        let codes  = 0..255u16;
        let mut numbers_string = String::new();
        let mut valid_styles: Vec<GraphicStyle> = vec![];
        for code in codes {
            if let Some(style) = GraphicStyle::try_from_u16(code) {
                if numbers_string.len() > 0 {
                    numbers_string.push(';');
                }
                numbers_string.extend(format!("{}", code).chars());
                valid_styles.push(style);
            }
        }
        test_valid_sequence(format!("[{}m", numbers_string).as_bytes(), Command::SetGraphicStyles(valid_styles.as_slice()));
    }

    #[test]
    fn valid_set_multiple_graphic_styles_with_some_invalid() {
        let codes  = 0..255u16;
        let mut numbers_string = String::new();
        let mut valid_styles: Vec<GraphicStyle> = vec![];
        for code in codes {
            if numbers_string.len() > 0 {
                numbers_string.push(';');
            }
            numbers_string.extend(format!("{}", code).chars());
            if let Some(style) = GraphicStyle::try_from_u16(code) {
                valid_styles.push(style);
            }
        }
        test_valid_sequence(format!("[{}m", numbers_string).as_bytes(), Command::SetGraphicStyles(valid_styles.as_slice()));
    }

    #[test]
    fn valid_save_cursor_location() {
        test_valid_sequence(b"7", Command::SaveCursorToMemory);
        test_valid_sequence(b"8", Command::RestoreCursorFromMemory);
        test_valid_sequence(b"[s", Command::SaveCursorToMemory);
        test_valid_sequence(b"[u", Command::RestoreCursorFromMemory);
    }

    #[test]
    fn valid_scroll_viewport() {
        let values = generate_sample_values();
        for &v in &values {
            let y =  NonZeroU16::new(v.clamp(1, MAX_VALUE)).unwrap();
            test_valid_sequence(format!("[{}S", v).as_bytes(), Command::ScrollUp(y));
            test_valid_sequence(format!("[{}T", v).as_bytes(), Command::ScrollDown(y));
        }
        let default_y = NonZeroU16::new(1).unwrap();
        test_valid_sequence(b"[S", Command::ScrollUp(default_y));
        test_valid_sequence(b"[T", Command::ScrollDown(default_y));
    }

    #[test]
    fn valid_text_modification() {
        let values = generate_sample_values();
        for &v in &values {
            let x = NonZeroU16::new(v.clamp(1, MAX_VALUE)).unwrap();
            test_valid_sequence(format!("[{}@", v).as_bytes(), Command::InsertSpaces(x));
            test_valid_sequence(format!("[{}P", v).as_bytes(), Command::DeleteCharacters(x));
            test_valid_sequence(format!("[{}X", v).as_bytes(), Command::ReplaceWithSpaces(x));
            test_valid_sequence(format!("[{}L", v).as_bytes(), Command::InsertLines(x));
            test_valid_sequence(format!("[{}M", v).as_bytes(), Command::DeleteLines(x));
        }
        let default_v = NonZeroU16::new(1).unwrap();
        test_valid_sequence(b"[@", Command::InsertSpaces(default_v));
        test_valid_sequence(b"[P", Command::DeleteCharacters(default_v));
        test_valid_sequence(b"[X", Command::ReplaceWithSpaces(default_v));
        test_valid_sequence(b"[L", Command::InsertLines(default_v));
        test_valid_sequence(b"[M", Command::DeleteLines(default_v));
    }

    #[test]
    fn valid_text_erase_mode() {
        for v in 0..=3 {
            if let Some(mode) = EraseMode::try_from_u16(v) {
                test_valid_sequence(format!("[{}J", v).as_bytes(), Command::EraseInDisplay(mode));
                test_valid_sequence(format!("[{}K", v).as_bytes(), Command::EraseInLine(mode));
            }
        }
        let mode = EraseMode::try_from_u16(0).unwrap();
        test_valid_sequence(b"[J", Command::EraseInDisplay(mode));
        test_valid_sequence(b"[K", Command::EraseInLine(mode));
    }

    #[test]
    fn valid_query_state() {
        test_valid_sequence(b"[6n", Command::QueryCursorPosition);
        test_valid_sequence(b"[0c", Command::QueryTerminalIdentity);
    }

    #[test]
    fn valid_tab_commands() {
        test_valid_sequence(b"H", Command::SetTabStopAtCurrentColumn);
        test_valid_sequence(b"[0g", Command::ClearCurrentTabStop);
        test_valid_sequence(b"[3g", Command::ClearAllTabStops);
        let values = generate_sample_values();
        for &v in &values {
            let n = NonZeroU16::new(v.clamp(1,MAX_VALUE)).unwrap();
            test_valid_sequence(format!("[{}I", v).as_bytes(), Command::AdvanceCursorToTabStop(n));
            test_valid_sequence(format!("[{}Z", v).as_bytes(), Command::ReverseCursorToTabStop(n));
        }
        let default_v = NonZeroU16::new(1).unwrap();
        test_valid_sequence(b"[I", Command::AdvanceCursorToTabStop(default_v));
        test_valid_sequence(b"[Z", Command::ReverseCursorToTabStop(default_v));
    }

    #[test]
    fn valid_designate_character_set() {
        test_valid_sequence(b"(0", Command::SetCharacterSet(CharacterSet::LineDrawing));
        test_valid_sequence(b"(B", Command::SetCharacterSet(CharacterSet::Ascii));
    }

    #[test]
    fn valid_scrolling_margins() {
        let values = generate_sample_values();
        for &top in &values {
            for &bottom in &values {
                let region = ScrollRegion::new(top.min(MAX_VALUE), bottom.min(MAX_VALUE));
                test_valid_sequence(format!("[{};{}r",top,bottom).as_bytes(), Command::SetScrollRegion(Some(region)));
            }
        }
        test_valid_sequence(b"[r", Command::SetScrollRegion(None));
    }

    #[test]
    fn valid_screen_mode() {
        const MODE_IDS: [u16;14] = [0,1,2,3,4,5,6,13,14,15,16,17,18,19];
        for &id in &MODE_IDS {
            let mode = ScreenMode::try_from_u16(id).unwrap(); 
            test_valid_sequence(format!("[={}h", id).as_bytes(), Command::SetScreenMode(mode));
            test_valid_sequence(format!("[={}l", id).as_bytes(), Command::ResetScreenMode(mode));
        }
    }

    #[test]
    fn valid_toggle_line_wrap() {
        test_valid_sequence(b"[=7h", Command::SetLineWrapping(true));
        test_valid_sequence(b"[=7l", Command::SetLineWrapping(false));
    }

    #[test]
    fn valid_set_hyperlink() {
        // 0x08 = BELL
        let tag = "id=9180-2";
        let link = "https://google.com.au/vt100.html";
        test_valid_sequence(format!("]8;{};{}\x07", tag, link).as_bytes(), Command::SetHyperlink { tag: tag.as_bytes(), link: link.as_bytes() });
        test_valid_sequence(b"]8;;\x07", Command::SetHyperlink { tag: &[], link: &[] });
    }

    #[test]
    fn invalid_set_multiple_graphic_styles() {
        let codes  = 0..255u16;
        let mut numbers_string = String::new();
        for code in codes {
            if GraphicStyle::try_from_u16(code).is_none() {
                if numbers_string.len() > 0 {
                    numbers_string.push(';');
                }
                numbers_string.extend(format!("{}", code).chars());
            }
        }
        test_invalid_sequence(format!("[{}m", numbers_string).as_bytes(), None, ParserError::NoValidGraphicStyles);
    }

    #[test]
    fn invalid_set_single_graphic_style() {
        for v in 0..255 {
            if GraphicStyle::try_from_u16(v).is_none() {
                test_invalid_sequence(format!("[{}m", v).as_bytes(), None, ParserError::NoValidGraphicStyles);
            }
        }
    }

    #[test]
    fn invalid_private_modes_nonstandard() {
        let invalid_codes = [0, 50, 1000, 2000, 4000];
        for n in invalid_codes {
            for c in &['h','l','c','d'] {
                let seq = format!("[?{}{}", n, c);
                test_invalid_sequence(seq.as_bytes(), None, ParserError::Unhandled);
            }
        }
    }

    #[test]
    fn invalid_text_erase_mode() {
        for v in 0..=25 {
            if EraseMode::try_from_u16(v).is_none() {
                test_invalid_sequence(format!("[{}J", v).as_bytes(), None, ParserError::InvalidEraseMode(v));
                test_invalid_sequence(format!("[{}K", v).as_bytes(), None, ParserError::InvalidEraseMode(v));
            }
        }
    }
}
