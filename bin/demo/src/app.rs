use terminal::{
    Terminal, 
    TerminalBuilder,
    TerminalIOControl,
    TerminalUserEvent,
};
use terminal::terminal_renderer::TerminalRenderer;
use terminal_process::TerminalProcess;
use vt100::common::WindowAction;
use crate::app_events::AppEvent;
use crate::app_window::AppWindow;
use cgmath::Vector2;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

pub struct AppBuilder {
    pub font_filename: String,
    pub font_size: f32,
    pub process: Arc<Mutex<Box<dyn TerminalProcess + Send>>>,
}

fn create_default_terminal_builder(process: Arc<Mutex<Box<dyn TerminalProcess + Send>>>) -> anyhow::Result<TerminalBuilder> {
    let process_read = {
        let mut read_pipe = process.lock().unwrap().get_read_pipe()?;
        move |data: &mut [u8]| {
            match read_pipe.read(data) {
                Ok(total) => total,
                Err(err) => {
                    log::info!("Terminal process read pipe failed: {:?}", err);
                    0
                }
            }
        }
    };
    let process_write = {
        let mut write_pipe = process.lock().unwrap().get_write_pipe()?;
        move |data: &[u8]| {
            if let Err(err) = write_pipe.write_all(data) {
                log::info!("Terminal process write pipe failed: {:?}", err);
            }
        }
    };
    let process_ioctl = {
        let process = process.clone();
        move |ev: TerminalIOControl| {
            let mut process = process.lock().unwrap();
            process.on_ioctl(ev).unwrap();
        }
    };
    let window_action = |_action: WindowAction| {};
    let is_newline_carriage_return = process.lock().unwrap().is_newline_carriage_return();
    Ok(TerminalBuilder {
        process_read: Box::new(process_read),
        process_write: Box::new(process_write),
        process_ioctl: Box::new(process_ioctl),
        window_action: Box::new(window_action),
        is_newline_carriage_return,
    })
}

pub fn start_app(builder: AppBuilder) -> anyhow::Result<()> {
    let process = builder.process;
    let mut terminal_builder = create_default_terminal_builder(process.clone())?;
    let event_loop = winit::event_loop::EventLoopBuilder::<AppEvent>::with_user_event().build()?;
    use std::sync::atomic::{AtomicBool, Ordering};
    let is_refresh_trigger = Arc::new(AtomicBool::new(false));
    let window_action = {
        let event_loop_proxy = event_loop.create_proxy();
        let is_refresh_trigger = is_refresh_trigger.clone();
        move |action: WindowAction| {
            if action == WindowAction::Refresh && is_refresh_trigger.fetch_or(true, Ordering::SeqCst) {
                return;
            }
            let _ = event_loop_proxy.send_event(AppEvent::WindowAction(action));
        }
    };
    terminal_builder.window_action = Box::new(window_action);
    let terminal = Terminal::new(terminal_builder);
    let window = winit::window::WindowBuilder::new().build(&event_loop)?;
    let mut window_size = window.inner_size();
    window_size.width = window_size.width.max(1);
    window_size.height = window_size.height.max(1);
    let mut terminal_window = pollster::block_on(AppWindow::new(
        &window,
        terminal,
        builder.font_filename, builder.font_size,
    ))?;
    event_loop.run({
        let is_refresh_trigger = is_refresh_trigger.clone();
        use winit::event::{Event, WindowEvent};
        move |event, target| {
            if let Event::WindowEvent { ref event, .. } = event { 
                if event == &WindowEvent::RedrawRequested {
                    is_refresh_trigger.store(false, Ordering::SeqCst);
                }
            }
            terminal_window.on_winit_event(event, target);
        }
    })?;
    match process.lock().unwrap().terminate() {
        Ok(()) => log::info!("Process terminated successfully"), 
        Err(err) => log::error!("Process failed to be terminated: {:?}", err),
    }
    Ok(())
}

pub fn start_headless(builder: AppBuilder) -> anyhow::Result<()> {
    let process = builder.process;
    let terminal_builder = create_default_terminal_builder(process.clone())?;
    let mut terminal = Terminal::new(terminal_builder);
    terminal.join_parser_thread();
    match process.lock().unwrap().terminate() {
        Ok(()) => log::info!("Process terminated successfully"),
        Err(err) => log::error!("Process failed to be terminated: {:?}", err),
    }

    let mut terminal_renderer = TerminalRenderer::default();
    let display = terminal.get_display();
    terminal_renderer.render_display(&display);
    let size = terminal_renderer.get_size();
    let cells = terminal_renderer.get_cells();
    let mut tmp_buf = [0u8; 4];
    let mut stdout = std::io::stdout();
    for y in 0..size.y {
        let index = y*size.x;
        let row = &cells[index..(index+size.x)];
        for cell in row {
            let data = cell.character.encode_utf8(&mut tmp_buf);
            let _ = stdout.write(data.as_bytes());
        }
        let _ = stdout.write(b"\n");
    }
    Ok(())
}
