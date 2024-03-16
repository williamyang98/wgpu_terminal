use crate::{
    terminal_parser::TerminalParser,
    terminal_display::TerminalDisplay,
    terminal_keyboard::TerminalKeyboard,
    terminal_process::TerminalProcess,
    terminal_renderer::TerminalRenderer,
};
use std::sync::{Arc,Mutex};
use std::thread::JoinHandle;
use std::io::{Read};
use std::ops::DerefMut;
use cgmath::Vector2;

pub struct Terminal {
    process: Box<dyn TerminalProcess>,
    display: Arc<Mutex<TerminalDisplay>>,
    renderer: TerminalRenderer,
    keyboard: TerminalKeyboard,
    read_thread: Option<JoinHandle<()>>,
}

impl Terminal {
    pub fn new(mut process: Box<dyn TerminalProcess>) -> Self {
        let display = Arc::new(Mutex::new(TerminalDisplay::default()));
        let read_thread = std::thread::spawn({
            let display = display.clone();
            let read_pipe = process.get_read_pipe();
            move || {
                start_process_read_thread(display, read_pipe);
            }
        });
        let keyboard = {
            let write_pipe = process.get_write_pipe();
            TerminalKeyboard::new(write_pipe)
        };
        Self {
            process,
            display,
            renderer: TerminalRenderer::default(),
            keyboard,
            read_thread: Some(read_thread),
        }
    }

    pub fn set_size(&mut self, size: Vector2<usize>) {
        if let Err(err) = self.process.set_size(size) {
            log::error!("Couldn't resize process: {:?}", err);
        }
        self.renderer.set_size(size);
        let mut display = self.display.lock().unwrap();
        display.get_viewport_mut().set_size(size);
    }

    pub fn get_keyboard_mut(&mut self) -> &mut TerminalKeyboard {
        &mut self.keyboard
    }

    pub fn get_renderer(&self) -> &TerminalRenderer {
        &self.renderer
    }

    pub fn get_renderer_mut(&mut self) -> &mut TerminalRenderer {
        &mut self.renderer
    }

    pub fn try_render(&mut self) -> bool {
        match self.display.try_lock() {
            Ok(ref display) => {
                let viewport = display.get_viewport();
                self.renderer.render_viewport(viewport);
                true
            },
            Err(_err) => {
                false
            },
        }
    }

    pub fn wait(&mut self) {
        if let Some(thread) = self.read_thread.take() {
            if let Err(err) = thread.join() {
                log::error!("Failed to join read thread: {:?}", err);
            }
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.process.terminate();
        self.wait();
    }
}

fn start_process_read_thread(display: Arc<Mutex<TerminalDisplay>>, mut read_pipe: Box<dyn Read + Send>) {
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
                match display.lock() {
                    Ok(ref mut display) => parser.parse_bytes(data, display.deref_mut()),
                    Err(err) => {
                        log::error!("Error while acquiring terminal: {:?}", err);
                        break;
                    },
                }
            }, 
            Err(err) => {
                log::error!("Error while reading child.stdout: {:?}", err);
                break;
            },
        };
    }
}
