use terminal::terminal_display::TerminalDisplay;
use terminal::terminal_core::TerminalCore;
use terminal::terminal_window::TerminalWindow;
use terminal::terminal_keyboard::TerminalKeyboard;

use std::sync::{Arc,Mutex,MutexGuard};

use std::io::Write;
use vt100::{
    common::{WindowAction},
};
use cgmath::Vector2;


#[derive(Clone,Debug,Default)]
pub struct TestWindow {
    pub actions: Vec<WindowAction>,
}

#[derive(Clone,Debug,Default)]
pub struct TestProcess {
    pub buffer: Vec<u8>,
}

struct ProcessWriter {
    process: Arc<Mutex<TestProcess>>,
}

impl Write for ProcessWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let mut process = self.process.lock().unwrap();
        process.buffer.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct WindowWriter {
    window: Arc<Mutex<TestWindow>>,
}

impl TerminalWindow for WindowWriter {
    fn on_window_action(&self, action: WindowAction) {
        let mut window = self.window.lock().unwrap();
        window.actions.push(action);
    }
}

pub struct TestTerminal {
    display: Arc<Mutex<TerminalDisplay>>,
    keyboard: Arc<Mutex<TerminalKeyboard>>,
    window: Arc<Mutex<TestWindow>>,
    process: Arc<Mutex<TestProcess>>,
}

impl Default for TestTerminal {
    fn default() -> Self {
        let window = TestWindow::default();
        let mut display = TerminalDisplay::default();
        let process = Arc::new(Mutex::new(TestProcess::default()));
        let keyboard = TerminalKeyboard::new(Box::new(ProcessWriter { process: process.clone() }));
        // treat \n as \r\n in test suite
        display.set_is_newline_carriage_return(true);
        Self { 
            display: Arc::new(Mutex::new(display)),
            keyboard: Arc::new(Mutex::new(keyboard)),
            window: Arc::new(Mutex::new(window)),
            process,
        }
    }
}

impl TestTerminal {
    pub fn create_core(&self) -> TerminalCore {
        TerminalCore {
            display: self.display.clone(),
            keyboard: self.keyboard.clone(),
            window: Box::new(WindowWriter { window: self.window.clone() }),
        }
    }

    pub fn set_size(&mut self, size: Vector2<usize>) {
        let mut display = self.display.lock().unwrap();
        display.get_viewport_mut().set_size(size);
    }

    pub fn get_display(&self) -> MutexGuard<TerminalDisplay> {
        self.display.lock().unwrap()
    }

    pub fn get_keyboard(&self) -> MutexGuard<TerminalKeyboard> {
        self.keyboard.lock().unwrap()
    }

    pub fn get_window(&self) -> MutexGuard<TestWindow> {
        self.window.lock().unwrap()
    }

    pub fn get_process(&self) -> MutexGuard<TestProcess> {
        self.process.lock().unwrap()
    }
}

