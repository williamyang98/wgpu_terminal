pub mod parser;
pub mod command;
pub mod common;
pub mod encoder;

#[cfg(test)]
mod tests {
    use cgmath::Vector2;
    use crate::{
        command::Command,
        parser::{Parser,ParserHandler,ParserError},
        common::*,
        encoder::*,
    };
    use std::collections::HashMap;

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

    #[derive(Default,Debug,Clone)]
    struct Handler {
        errors: Vec<ParserError>,
        commands: Vec<Command>,
    }

    impl ParserHandler for Handler {
        fn on_command(&mut self, command: Command) {
            self.commands.push(command);
        }
        fn on_error(&mut self, error: ParserError, _parser: &Parser) {
            self.errors.push(error);
        }
    }

    fn test_valid_sequence(seq: &[u8], commands: &[Command]) {
        let mut parser = Parser::default(); 
        let mut handler = Handler::default();
        for (i,&b) in seq.iter().enumerate() {
            parser.feed_byte(b, &mut handler);
            if i < seq.len()-1 {
                if parser.is_terminated() {
                    panic!(
                        "[error] Parser ended early at index {}\n  \
                           Sequence: {:?} ({:?})\n  \
                           Expected: {:?}\n  \
                           Parser: {:?}\n  \
                           Handler: {:?}",
                        i, 
                        seq, std::str::from_utf8(seq).unwrap_or(""),
                        commands,
                        parser,
                        handler,
                    );
                }
            } else if handler.commands.as_slice() != commands {
                panic!(
                    "[error] Parser didnt get expected command at index {}\n  \
                       Sequence: {:?} ({:?})\n  \
                       Expected: {:?}\n  \
                       Parser: {:?}\n  \
                       Handler: {:?}",
                    i, 
                    seq, std::str::from_utf8(seq).unwrap_or(""),
                    commands,
                    parser,
                    handler,
                );
            }
        }
    }

    fn test_invalid_sequence(seq: &[u8], index: Option<usize>, errors: &[ParserError]) {
        let index = index.unwrap_or(seq.len().max(1)-1);
        let mut parser = Parser::default();
        let mut handler = Handler::default();
        for (i,&b) in seq.iter().enumerate() {
            parser.feed_byte(b, &mut handler);
            if i == index {
                if !parser.is_terminated() || handler.errors.as_slice() != errors {
                    panic!(
                        "[error] Parser didnt get expected error at index {}\n  \
                           Sequence: {:?} ({:?})\n  \
                           Expected: {:?}\n  \
                           Parser: {:?}\n  \
                           Handler: {:?}",
                        i, 
                        seq, std::str::from_utf8(seq).unwrap_or(""),
                        errors, 
                        parser,
                        handler,
                    );
                }
            } else if parser.is_terminated() { 
                panic!(
                    "[error] Parser ended early at index {}\n  \
                       Sequence: {:?} ({:?})\n  \
                       Expected: {:?}\n  \
                       Parser: {:?}\n  \
                       Handler: {:?}",
                    i, 
                    seq, std::str::from_utf8(seq).unwrap_or(""),
                    errors, 
                    parser,
                    handler,
                );
            }
        }
    }

    #[test]
    fn valid_single_move_cursor() {
        let default_v = 1;
        test_valid_sequence(b"A", &[Command::MoveCursorUp(default_v)]);
        test_valid_sequence(b"B", &[Command::MoveCursorDown(default_v)]);
        test_valid_sequence(b"C", &[Command::MoveCursorRight(default_v)]);
        test_valid_sequence(b"D", &[Command::MoveCursorLeft(default_v)]);
        test_valid_sequence(b"M", &[Command::MoveCursorUp(default_v)]);
        test_valid_sequence(b"[A", &[Command::MoveCursorUp(default_v)]);
        test_valid_sequence(b"[B", &[Command::MoveCursorDown(default_v)]);
        test_valid_sequence(b"[C", &[Command::MoveCursorRight(default_v)]);
        test_valid_sequence(b"[D", &[Command::MoveCursorLeft(default_v)]);
        test_valid_sequence(b"[E", &[Command::MoveCursorNextLine(default_v)]);
        test_valid_sequence(b"[F", &[Command::MoveCursorPreviousLine(default_v)]);
        test_valid_sequence(b"[G", &[Command::MoveCursorHorizontalAbsolute(default_v)]);
        test_valid_sequence(b"[d", &[Command::MoveCursorVerticalAbsolute(default_v)]);
    }
 
    #[test]
    fn valid_multiple_move_cursor() {
        let values = generate_sample_values();
        for &v in &values {
            let n = v.clamp(1, MAX_VALUE);
            test_valid_sequence(format!("[{}A", v).as_bytes(), &[Command::MoveCursorUp(n)]);
            test_valid_sequence(format!("[{}B", v).as_bytes(), &[Command::MoveCursorDown(n)]);
            test_valid_sequence(format!("[{}C", v).as_bytes(), &[Command::MoveCursorRight(n)]);
            test_valid_sequence(format!("[{}D", v).as_bytes(), &[Command::MoveCursorLeft(n)]);
            test_valid_sequence(format!("[{}E", v).as_bytes(), &[Command::MoveCursorNextLine(n)]);
            test_valid_sequence(format!("[{}F", v).as_bytes(), &[Command::MoveCursorPreviousLine(n)]);
            test_valid_sequence(format!("[{}G", v).as_bytes(), &[Command::MoveCursorHorizontalAbsolute(n)]);
            test_valid_sequence(format!("[{}d", v).as_bytes(), &[Command::MoveCursorVerticalAbsolute(n)]);
        }
    }

    #[test]
    fn valid_move_xy_cursor() {
        let values = generate_sample_values();
        for &x in &values {
            for &y in &values {
                let x_actual = x.min(MAX_VALUE).max(1);
                let y_actual = y.min(MAX_VALUE).max(1);
                let command = Command::MoveCursorPositionViewport(Vector2::new(x_actual,y_actual));
                test_valid_sequence(format!("[{};{}H", y, x).as_bytes(), &[command.clone()]);
                test_valid_sequence(format!("[{};{}f", y, x).as_bytes(), &[command.clone()]);
            }
        }
        test_valid_sequence(b"[H", &[Command::MoveCursorPositionViewport(Vector2::new(1,1))]);
    }

    #[test]
    fn valid_set_input_mode() {
        test_valid_sequence(b"=", &[Command::SetKeypadMode(InputMode::Application)]);
        test_valid_sequence(b">", &[Command::SetKeypadMode(InputMode::Numeric)]);
    }

    fn get_private_mode_nonstandard_commands() -> HashMap<(u16,bool), Vec<Command>> {
        HashMap::from([
            ((   1, true) , vec![Command::SetCursorKeyInputMode(InputMode::Application)]),
            ((   1, false), vec![Command::SetCursorKeyInputMode(InputMode::Numeric)]),
            ((   3, true) , vec![Command::SetConsoleWidth(132)]),
            ((   3, false), vec![Command::SetConsoleWidth(80)]),
            ((   5, true) , vec![Command::SetLightBackground]),
            ((   5, false), vec![Command::SetDarkBackground]),
            ((   9, true) , vec![Command::SetMouseTrackingMode(MouseTrackingMode::X10), Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10)]),
            ((   9, false), vec![Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)]),
            ((  12, true) , vec![Command::SetCursorBlinking(true)]),
            ((  12, false), vec![Command::SetCursorBlinking(false)]),
            ((  25, true) , vec![Command::SetCursorVisible(true)]),
            ((  25, false), vec![Command::SetCursorVisible(false)]),
            ((  47, true) , vec![Command::SetAlternateBuffer(true)]),
            ((  47, false), vec![Command::SetAlternateBuffer(false)]),
            ((1000, true) , vec![Command::SetMouseTrackingMode(MouseTrackingMode::Normal)]),
            ((1000, false), vec![Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)]),
            ((1001, true) , vec![Command::SetMouseTrackingMode(MouseTrackingMode::Highlight)]),
            ((1001, false), vec![Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)]),
            ((1002, true) , vec![Command::SetMouseTrackingMode(MouseTrackingMode::Motion)]),
            ((1002, false), vec![Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)]),
            ((1003, true) , vec![Command::SetMouseTrackingMode(MouseTrackingMode::Any)]),
            ((1003, false), vec![Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)]),
            ((1004, true) , vec![Command::SetReportFocus(true)]),
            ((1004, false), vec![Command::SetReportFocus(false)]),
            ((1005, true),  vec![Command::SetMouseCoordinateFormat(MouseCoordinateFormat::Utf8)]),
            ((1005, false), vec![Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10)]),
            ((1006, true),  vec![Command::SetMouseCoordinateFormat(MouseCoordinateFormat::Sgr)]),
            ((1006, false), vec![Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10)]),
            ((1015, true),  vec![Command::SetMouseCoordinateFormat(MouseCoordinateFormat::Urxvt)]),
            ((1015, false), vec![Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10)]),
            ((1016, true),  vec![Command::SetMouseCoordinateFormat(MouseCoordinateFormat::SgrPixel)]),
            ((1016, false), vec![Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10)]),
            ((1047, true) , vec![Command::SetAlternateBuffer(true)]),
            ((1047, false), vec![Command::SetAlternateBuffer(false)]),
            ((1048, true) , vec![Command::SaveCursorToMemory]),
            ((1048, false), vec![Command::RestoreCursorFromMemory]),
            ((1049, true) , vec![Command::SaveCursorToMemory, Command::SetAlternateBuffer(true)]),
            ((1049, false), vec![Command::SetAlternateBuffer(false), Command::RestoreCursorFromMemory]),
            ((2004, true),  vec![Command::SetBracketedPasteMode(true)]),
            ((2004, false), vec![Command::SetBracketedPasteMode(false)]),
        ])
    }

    #[test]
    fn valid_private_modes_nonstandard() {
        let codes = get_private_mode_nonstandard_commands();
        for ((mode, is_enable), commands) in &codes {
            let tag = if *is_enable { "h" } else { "l" };
            test_valid_sequence(format!("[?{}{}", mode, tag).as_bytes(), commands.as_slice());
        }
    }

    #[test]
    fn valid_multiple_private_modes_nonstandard_enable() {
        let codes = get_private_mode_nonstandard_commands();
        let mut modes = Vec::new();
        let mut commands = Vec::new();
        for ((mode, _), sub_commands) in codes.iter().filter(|((_, is_enable), _)| *is_enable) {
            modes.push(mode);
            commands.extend_from_slice(sub_commands.as_slice());
        }
        let numbers_string: Vec<String> = modes.iter().map(|v| format!("{}", v)).collect();
        let numbers_string = numbers_string.join(";");
        test_valid_sequence(format!("[?{}h", numbers_string).as_bytes(), commands.as_slice());
    }

    #[test]
    fn valid_multiple_private_modes_nonstandard_disable() {
        let codes = get_private_mode_nonstandard_commands();
        let mut modes = Vec::new();
        let mut commands = Vec::new();
        for ((mode, _), sub_commands) in codes.iter().filter(|((_, is_enable), _)| !*is_enable) {
            modes.push(mode);
            commands.extend_from_slice(sub_commands.as_slice());
        }
        let numbers_string: Vec<String> = modes.iter().map(|v| format!("{}", v)).collect();
        let numbers_string = numbers_string.join(";");
        test_valid_sequence(format!("[?{}l", numbers_string).as_bytes(), commands.as_slice());
    }

    #[test]
    fn valid_set_key_modifier_option() {
        for n in 0..5u16 {
            if let Some(key_type) = KeyType::try_from_u16(n) {
                for value in 0..5u16 {
                    let command = Command::SetKeyModifierOption(key_type, Some(value));
                    test_valid_sequence(format!("[>{};{}m", n, value).as_bytes(), &[command]);
                }
            }
        }
        for n in 0..5u16 {
            if let Some(key_type) = KeyType::try_from_u16(n) {
                let command = Command::SetKeyModifierOption(key_type, None);
                test_valid_sequence(format!("[>{}m", n).as_bytes(), &[command]);
            }
        }
    }

    #[test]
    fn valid_soft_reset() {
        test_valid_sequence(b"[!p", &[Command::SoftReset]);
    }

    #[test]
    fn valid_set_screen_title() {
        let window_titles = ["", "hello world", "ĐđĒēĔĕĖėĘęĚěĜĝĞğ"];
        for window_title in &window_titles {
            let command = Command::WindowAction(WindowAction::SetWindowTitle(window_title.to_string()));
            test_valid_sequence(format!("]0;{}\x07", window_title).as_bytes(), &[command.clone()]);
            test_valid_sequence(format!("]2;{}\x07", window_title).as_bytes(), &[command.clone()]);
        }
    }

    #[test]
    fn valid_set_colour_from_table() {
        for v in 0..=260 {
            let actual_value = v.min(255) as u8;
            test_valid_sequence(format!("[38;5;{}m",v).as_bytes(), &[Command::SetForegroundColourTable(actual_value)]); 
            test_valid_sequence(format!("[48;5;{}m",v).as_bytes(), &[Command::SetBackgroundColourTable(actual_value)]); 
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
                    let command = Command::SetForegroundColourRgb(colour);
                    test_valid_sequence(format!("[38;2;{};{};{}m",r,g,b).as_bytes(), &[command]); 
                    let command = Command::SetBackgroundColourRgb(colour);
                    test_valid_sequence(format!("[48;2;{};{};{}m",r,g,b).as_bytes(), &[command]); 
                }
            }
        }
    }

    #[test]
    fn valid_set_default_graphic_style() {
        let default_style = GraphicStyle::try_from_u16(0).unwrap();
        test_valid_sequence(b"[m", &[Command::SetGraphicStyle(default_style)]);
    }

    #[test]
    fn valid_set_single_graphic_style() {
        for v in 0..255 {
            if let Some(style) = GraphicStyle::try_from_u16(v) {
                test_valid_sequence(format!("[{}m",v).as_bytes(), &[Command::SetGraphicStyle(style)]);
            }
        }
    }

    #[test]
    fn valid_set_multiple_graphic_styles() {
        let codes  = 0..255u16;
        let mut valid_codes = Vec::new();
        let mut valid_styles: Vec<GraphicStyle> = vec![];
        for code in codes {
            if let Some(style) = GraphicStyle::try_from_u16(code) {
                valid_codes.push(code);
                valid_styles.push(style);
            }
        }
        let numbers_string: Vec<String> = valid_codes.iter().map(|v| format!("{}", v)).collect();
        let numbers_string = numbers_string.join(";");
        let commands: Vec<Command> = valid_styles.iter().map(|style| Command::SetGraphicStyle(*style)).collect();
        test_valid_sequence(format!("[{}m", numbers_string).as_bytes(), commands.as_slice());
    }

    #[test]
    fn valid_set_multiple_graphic_styles_with_some_invalid() {
        let codes: Vec<u16> = (0..255u16).collect();
        let mut invalid_codes = Vec::new();
        let mut valid_styles: Vec<GraphicStyle> = vec![];
        for code in codes.iter() {
            if let Some(style) = GraphicStyle::try_from_u16(*code) {
                valid_styles.push(style);
            } else {
                invalid_codes.push(*code);
            }
        }
        let numbers_string: Vec<String> = codes.iter().map(|v| format!("{}", v)).collect();
        let numbers_string = numbers_string.join(";");
        let commands: Vec<Command> = valid_styles.iter().map(|style| Command::SetGraphicStyle(*style)).collect();
        let errors: Vec<ParserError> = invalid_codes.iter().map(|code| ParserError::InvalidGraphicStyle(*code)).collect();
        test_valid_sequence(format!("[{}m", numbers_string).as_bytes(), commands.as_slice());
        test_invalid_sequence(format!("[{}m", numbers_string).as_bytes(), None, errors.as_slice());
    }

    #[test]
    fn valid_save_cursor_location() {
        test_valid_sequence(b"7", &[Command::SaveCursorToMemory]);
        test_valid_sequence(b"8", &[Command::RestoreCursorFromMemory]);
        test_valid_sequence(b"[s", &[Command::SaveCursorToMemory]);
        test_valid_sequence(b"[u", &[Command::RestoreCursorFromMemory]);
    }

    #[test]
    fn valid_scroll_viewport() {
        let values = generate_sample_values();
        for &v in &values {
            let y =  v.clamp(1, MAX_VALUE);
            test_valid_sequence(format!("[{}S", v).as_bytes(), &[Command::ScrollUp(y)]);
            test_valid_sequence(format!("[{}T", v).as_bytes(), &[Command::ScrollDown(y)]);
        }
        let default_v = 1;
        test_valid_sequence(b"[S", &[Command::ScrollUp(default_v)]);
        test_valid_sequence(b"[T", &[Command::ScrollDown(default_v)]);
    }

    #[test]
    fn valid_text_modification() {
        let values = generate_sample_values();
        for &v in &values {
            let x = v.clamp(1, MAX_VALUE);
            test_valid_sequence(format!("[{}@", v).as_bytes(), &[Command::InsertSpaces(x)]);
            test_valid_sequence(format!("[{}P", v).as_bytes(), &[Command::DeleteCharacters(x)]);
            test_valid_sequence(format!("[{}X", v).as_bytes(), &[Command::ReplaceWithSpaces(x)]);
            test_valid_sequence(format!("[{}L", v).as_bytes(), &[Command::InsertLines(x)]);
            test_valid_sequence(format!("[{}M", v).as_bytes(), &[Command::DeleteLines(x)]);
        }
        let default_v = 1;
        test_valid_sequence(b"[@", &[Command::InsertSpaces(default_v)]);
        test_valid_sequence(b"[P", &[Command::DeleteCharacters(default_v)]);
        test_valid_sequence(b"[X", &[Command::ReplaceWithSpaces(default_v)]);
        test_valid_sequence(b"[L", &[Command::InsertLines(default_v)]);
        test_valid_sequence(b"[M", &[Command::DeleteLines(default_v)]);
    }

    #[test]
    fn valid_text_erase_mode() {
        for v in 0..=3 {
            if let Some(mode) = EraseMode::try_from_u16(v) {
                test_valid_sequence(format!("[{}J", v).as_bytes(), &[Command::EraseInDisplay(mode)]);
                test_valid_sequence(format!("[{}K", v).as_bytes(), &[Command::EraseInLine(mode)]);
            }
        }
        let mode = EraseMode::try_from_u16(0).unwrap();
        test_valid_sequence(b"[J", &[Command::EraseInDisplay(mode)]);
        test_valid_sequence(b"[K", &[Command::EraseInLine(mode)]);
    }

    #[test]
    fn valid_query_state() {
        test_valid_sequence(b"[6n", &[Command::QueryCursorPosition]);
        test_valid_sequence(b"[c",  &[Command::QueryTerminalIdentity]);
        test_valid_sequence(b"[0c", &[Command::QueryTerminalIdentity]);
    }

    #[test]
    fn valid_query_key_modifier_option() {
        for n in 0..5u16 {
            if let Some(key_type) = KeyType::try_from_u16(n) {
                test_valid_sequence(format!("[?{}m",n).as_bytes(), &[Command::QueryKeyModifierOption(key_type)]);
            }
        }
    }

    #[test]
    fn valid_tab_commands() {
        test_valid_sequence(b"H", &[Command::SetTabStopAtCurrentColumn]);
        test_valid_sequence(b"[0g", &[Command::ClearCurrentTabStop]);
        test_valid_sequence(b"[3g", &[Command::ClearAllTabStops]);
        let values = generate_sample_values();
        for &v in &values {
            let n = v.clamp(1,MAX_VALUE);
            test_valid_sequence(format!("[{}I", v).as_bytes(), &[Command::AdvanceCursorToTabStop(n)]);
            test_valid_sequence(format!("[{}Z", v).as_bytes(), &[Command::ReverseCursorToTabStop(n)]);
        }
        let default_v = 1;
        test_valid_sequence(b"[I", &[Command::AdvanceCursorToTabStop(default_v)]);
        test_valid_sequence(b"[Z", &[Command::ReverseCursorToTabStop(default_v)]);
    }

    #[test]
    fn valid_designate_character_set() {
        test_valid_sequence(b"(0", &[Command::SetCharacterSet(CharacterSet::LineDrawing)]);
        test_valid_sequence(b"(B", &[Command::SetCharacterSet(CharacterSet::Ascii)]);
    }

    #[test]
    fn valid_scrolling_margins() {
        let values = generate_sample_values();
        for &top in &values {
            for &bottom in &values {
                let region = ScrollRegion::new(top.min(MAX_VALUE), bottom.min(MAX_VALUE));
                test_valid_sequence(format!("[{};{}r",top,bottom).as_bytes(), &[Command::SetScrollRegion(Some(region))]);
            }
        }
        test_valid_sequence(b"[r", &[Command::SetScrollRegion(None)]);
    }

    #[test]
    fn valid_screen_mode() {
        const MODE_IDS: [u16;14] = [0,1,2,3,4,5,6,13,14,15,16,17,18,19];
        for &id in &MODE_IDS {
            let mode = ScreenMode::try_from_u16(id).unwrap(); 
            test_valid_sequence(format!("[={}h", id).as_bytes(), &[Command::SetScreenMode(mode)]);
            test_valid_sequence(format!("[={}l", id).as_bytes(), &[Command::ResetScreenMode(mode)]);
        }
    }

    #[test]
    fn valid_toggle_line_wrap() {
        test_valid_sequence(b"[=7h", &[Command::SetLineWrapping(true)]);
        test_valid_sequence(b"[=7l", &[Command::SetLineWrapping(false)]);
    }

    #[test]
    fn valid_set_hyperlink() {
        // 0x08 = BELL
        let link = "id=9180-2;https://google.com.au/vt100.html";
        let command = Command::SetHyperlink(link.to_owned());
        test_valid_sequence(format!("]8;{}\x07", link).as_bytes(),  &[command]);
        test_valid_sequence(b"]8;;\x07", &[Command::SetHyperlink("".to_string())]);
    }

    #[test]
    fn valid_test_window_action() {
        test_valid_sequence(b"[1t", &[Command::WindowAction(WindowAction::SetMinimised(false))]);
        test_valid_sequence(b"[2t", &[Command::WindowAction(WindowAction::SetMinimised(true))]);
        test_valid_sequence(b"[5t", &[Command::WindowAction(WindowAction::SendToFront)]);
        test_valid_sequence(b"[6t", &[Command::WindowAction(WindowAction::SendToBack)]);
        test_valid_sequence(b"[7t", &[Command::WindowAction(WindowAction::Refresh)]);
        test_valid_sequence(b"[9;0t", &[Command::WindowAction(WindowAction::RestoreMaximised)]);
        test_valid_sequence(b"[9;1t", &[Command::WindowAction(WindowAction::Maximise(Vector2::new(true,true)))]);
        test_valid_sequence(b"[9;2t", &[Command::WindowAction(WindowAction::Maximise(Vector2::new(false,true)))]);
        test_valid_sequence(b"[9;3t", &[Command::WindowAction(WindowAction::Maximise(Vector2::new(true,false)))]);
        test_valid_sequence(b"[10;0t", &[Command::WindowAction(WindowAction::SetFullscreen(false))]);
        test_valid_sequence(b"[10;1t", &[Command::WindowAction(WindowAction::SetFullscreen(true))]);
        test_valid_sequence(b"[10;2t", &[Command::WindowAction(WindowAction::ToggleFullscreen)]);
        test_valid_sequence(b"[11t", &[Command::WindowAction(WindowAction::GetWindowState)]);
        test_valid_sequence(b"[13t", &[Command::WindowAction(WindowAction::GetWindowPosition)]);
        test_valid_sequence(b"[13;2t", &[Command::WindowAction(WindowAction::GetTextAreaPosition)]);
        test_valid_sequence(b"[14t", &[Command::WindowAction(WindowAction::GetTextAreaSize)]);
        test_valid_sequence(b"[14;2t", &[Command::WindowAction(WindowAction::GetWindowSize)]);
        test_valid_sequence(b"[15t", &[Command::WindowAction(WindowAction::GetScreenSize)]);
        test_valid_sequence(b"[16t", &[Command::WindowAction(WindowAction::GetCellSize)]);
        test_valid_sequence(b"[18t", &[Command::WindowAction(WindowAction::GetTextAreaGridSize)]);
        test_valid_sequence(b"[19t", &[Command::WindowAction(WindowAction::GetScreenGridSize)]);
        test_valid_sequence(b"[20t", &[Command::WindowAction(WindowAction::GetWindowIconLabel)]);
        test_valid_sequence(b"[21t", &[Command::WindowAction(WindowAction::GetWindowTitle)]);
        test_valid_sequence(b"[22;0t", &[Command::WindowAction(WindowAction::SaveIconTitle(None)), Command::WindowAction(WindowAction::SaveWindowTitle(None))]);
        test_valid_sequence(b"[22;1t", &[Command::WindowAction(WindowAction::SaveIconTitle(None))]);
        test_valid_sequence(b"[22;2t", &[Command::WindowAction(WindowAction::SaveWindowTitle(None))]);
        test_valid_sequence(b"[23;0t", &[Command::WindowAction(WindowAction::RestoreIconTitle(None)), Command::WindowAction(WindowAction::RestoreWindowTitle(None))]);
        test_valid_sequence(b"[23;1t", &[Command::WindowAction(WindowAction::RestoreIconTitle(None))]);
        test_valid_sequence(b"[23;2t", &[Command::WindowAction(WindowAction::RestoreWindowTitle(None))]);
        for n in 24..256 {
            let command = Command::WindowAction(WindowAction::ResizeWindowHeight(n));
            test_valid_sequence(format!("[{}t", n).as_bytes(), &[command]);
        }

        let values = generate_sample_values();
        for &x in values.iter() {
            for &y in values.iter() {
                let pos: Vector2<u16> = Vector2::new(x.min(MAX_VALUE),y.min(MAX_VALUE));
                test_valid_sequence(format!("[3;{};{}t",x,y).as_bytes(), &[Command::WindowAction(WindowAction::Move(pos))]);
                test_valid_sequence(format!("[4;{};{}t",x,y).as_bytes(), &[Command::WindowAction(WindowAction::Resize(pos))]);
                test_valid_sequence(format!("[8;{};{}t",x,y).as_bytes(), &[Command::WindowAction(WindowAction::ResizeTextArea(pos))]);
            }
        }
    }

    #[test]
    fn valid_cursor_style() {
        test_valid_sequence(b"[0 q", &[Command::SetCursorBlinking(true), Command::SetCursorStyle(CursorStyle::Block)]);
        test_valid_sequence(b"[1 q", &[Command::SetCursorBlinking(true), Command::SetCursorStyle(CursorStyle::Block)]);
        test_valid_sequence(b"[2 q", &[Command::SetCursorBlinking(false), Command::SetCursorStyle(CursorStyle::Block)]);
        test_valid_sequence(b"[3 q", &[Command::SetCursorBlinking(true), Command::SetCursorStyle(CursorStyle::Underline)]);
        test_valid_sequence(b"[4 q", &[Command::SetCursorBlinking(false), Command::SetCursorStyle(CursorStyle::Underline)]);
        test_valid_sequence(b"[5 q", &[Command::SetCursorBlinking(true), Command::SetCursorStyle(CursorStyle::Bar)]);
        test_valid_sequence(b"[6 q", &[Command::SetCursorBlinking(false), Command::SetCursorStyle(CursorStyle::Bar)]);
    }

    #[test]
    fn valid_column_shift() {
        let values = generate_sample_values();
        for &v in values.iter() {
            let n = v.clamp(1, MAX_VALUE);
            test_valid_sequence(format!("[{} @", v).as_bytes(), &[Command::ShiftLeftByColumns(n)]);
            test_valid_sequence(format!("[{} A", v).as_bytes(), &[Command::ShiftRightByColumns(n)]);
        }
    }

    #[test]
    fn valid_set_bell_volume() {
        test_valid_sequence(b"[0 t", &[Command::SetWarningBellVolume(BellVolume::Off)]);
        test_valid_sequence(b"[1 t", &[Command::SetWarningBellVolume(BellVolume::Off)]);
        test_valid_sequence(b"[2 t", &[Command::SetWarningBellVolume(BellVolume::Low)]);
        test_valid_sequence(b"[3 t", &[Command::SetWarningBellVolume(BellVolume::Low)]);
        test_valid_sequence(b"[4 t", &[Command::SetWarningBellVolume(BellVolume::Low)]);
        test_valid_sequence(b"[5 t", &[Command::SetWarningBellVolume(BellVolume::High)]);
        test_valid_sequence(b"[6 t", &[Command::SetWarningBellVolume(BellVolume::High)]);
        test_valid_sequence(b"[7 t", &[Command::SetWarningBellVolume(BellVolume::High)]);
        test_valid_sequence(b"[8 t", &[Command::SetWarningBellVolume(BellVolume::High)]);

        test_valid_sequence(b"[0 u", &[Command::SetMarginBellVolume(BellVolume::High)]);
        test_valid_sequence(b"[1 u", &[Command::SetMarginBellVolume(BellVolume::Off)]);
        test_valid_sequence(b"[2 u", &[Command::SetMarginBellVolume(BellVolume::Low)]);
        test_valid_sequence(b"[3 u", &[Command::SetMarginBellVolume(BellVolume::Low)]);
        test_valid_sequence(b"[4 u", &[Command::SetMarginBellVolume(BellVolume::Low)]);
        test_valid_sequence(b"[5 u", &[Command::SetMarginBellVolume(BellVolume::High)]);
        test_valid_sequence(b"[6 u", &[Command::SetMarginBellVolume(BellVolume::High)]);
        test_valid_sequence(b"[7 u", &[Command::SetMarginBellVolume(BellVolume::High)]);
        test_valid_sequence(b"[8 u", &[Command::SetMarginBellVolume(BellVolume::High)]);
    }

    #[test]
    fn invalid_set_multiple_graphic_styles() {
        let codes  = 0..255u16;
        let mut numbers_string = String::new();
        let mut errors = Vec::new();
        for code in codes {
            if GraphicStyle::try_from_u16(code).is_none() {
                if !numbers_string.is_empty() {
                    numbers_string.push(';');
                }
                numbers_string.extend(format!("{}", code).chars());
                errors.push(ParserError::InvalidGraphicStyle(code));
            }
        }
        test_invalid_sequence(format!("[{}m", numbers_string).as_bytes(), None, errors.as_slice());
    }

    #[test]
    fn invalid_set_single_graphic_style() {
        for v in 0..255 {
            if GraphicStyle::try_from_u16(v).is_none() {
                test_invalid_sequence(format!("[{}m", v).as_bytes(), None, &[ParserError::InvalidGraphicStyle(v)]);
            }
        }
    }

    #[test]
    fn invalid_text_erase_mode() {
        for v in 0..=25 {
            if EraseMode::try_from_u16(v).is_none() {
                test_invalid_sequence(format!("[{}J", v).as_bytes(), None, &[ParserError::InvalidEraseMode(v)]);
                test_invalid_sequence(format!("[{}K", v).as_bytes(), None, &[ParserError::InvalidEraseMode(v)]);
            }
        }
    }
}
