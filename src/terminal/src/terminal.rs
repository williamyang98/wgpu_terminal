use crate::{
    terminal_parser::TerminalParser,
    terminal_display::TerminalDisplay,
    terminal_keyboard::TerminalKeyboard,
    terminal_window::TerminalWindow,
    terminal_process::TerminalProcess,
    terminal_core::TerminalCore,
};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::io::Read;
use cgmath::Vector2;

pub struct Terminal {
    process: Box<dyn TerminalProcess>,
    display: Arc<Mutex<TerminalDisplay>>,
    keyboard: Arc<Mutex<TerminalKeyboard>>,
    read_thread: Option<JoinHandle<()>>,
}

impl Terminal {
    pub fn new(mut process: Box<dyn TerminalProcess>, window: Box<dyn TerminalWindow + Send>) -> Self {
        let read_pipe = process.get_read_pipe();
        let write_pipe = process.get_write_pipe();

        let mut display = TerminalDisplay::default();
        display.set_is_newline_carriage_return(process.is_newline_carriage_return());
        let keyboard = TerminalKeyboard::new(write_pipe);

        let display = Arc::new(Mutex::new(display));
        let keyboard = Arc::new(Mutex::new(keyboard));
        let core = TerminalCore {
            display: display.clone(),
            keyboard: keyboard.clone(),
            window,
        };
        let read_thread = std::thread::spawn({
            move || {
                start_process_read_thread(core, read_pipe);
            }
        });
        Self {
            process,
            keyboard,
            display,
            read_thread: Some(read_thread),
        }
    }

    pub fn get_display(&self) -> MutexGuard<TerminalDisplay> {
        self.display.lock().unwrap()
    }

    pub fn get_keyboard(&self) -> MutexGuard<TerminalKeyboard> {
        self.keyboard.lock().unwrap()
    }

    pub fn set_size(&mut self, size: Vector2<usize>) {
        if let Err(err) = self.process.set_size(size) {
            log::error!("Couldn't resize process: {:?}", err);
        }
        let mut display = self.display.lock().unwrap();
        display.get_viewport_mut().set_size(size);
    }

    pub fn wait(&mut self) {
        if let Some(handle) = self.read_thread.take() {
            handle.join().unwrap();
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(err) = self.process.terminate() {
            log::error!("Failed to terminate terminal process: {:?}", err);
        }
        self.wait();
    }
}

fn start_process_read_thread(mut core: TerminalCore, mut read_pipe: Box<dyn Read + Send>) {
    const BLOCK_SIZE: usize = 8192;
    let mut buffer = vec![0u8; BLOCK_SIZE];
    let mut parser =  TerminalParser::default();
    loop {
        match read_pipe.read(buffer.as_mut_slice()) {
            Ok(0) => {
                log::info!("Closing child.stdout after reading 0 bytes");
                break;
            },
            Ok(total_read) => {
                let data = &buffer[0..total_read];
                parser.parse_bytes(data, &mut core);
            }, 
            Err(err) => {
                log::error!("Error while reading child.stdout: {:?}", err);
                break;
            },
        };
    }
}
