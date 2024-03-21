use terminal::terminal::Terminal;
use terminal::terminal_process::TerminalProcess;
use terminal::terminal_window::TerminalWindow;
use terminal::terminal_renderer::TerminalRenderer;
use vt100::common::WindowAction;
use crate::app_events::AppEvent;
use crate::app_window::AppWindow;
use cgmath::Vector2;
use std::io::Write;

struct TerminalWindowEvents {
    event_loop_proxy: winit::event_loop::EventLoopProxy<AppEvent>,
}

impl TerminalWindow for TerminalWindowEvents {
    fn on_window_action(&self, action: WindowAction) {
        if let Err(err) = self.event_loop_proxy.send_event(AppEvent::WindowAction(action)) {
            log::error!("Failed to sent window action to app event loop: {:?}", err);
        }
    }
}

pub struct AppBuilder {
    pub font_filename: String,
    pub font_size: f32,
    pub process: Box<dyn TerminalProcess>,
}

pub fn start_app(builder: AppBuilder) -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoopBuilder::<AppEvent>::with_user_event().build()?;
    let terminal_window_events = TerminalWindowEvents { 
        event_loop_proxy: event_loop.create_proxy(),
    };
    let terminal = Terminal::new(builder.process, Box::new(terminal_window_events));

    let window = winit::window::WindowBuilder::new().build(&event_loop)?;
    let mut window_size = window.inner_size();
    window_size.width = window_size.width.max(1);
    window_size.height = window_size.height.max(1);
    let mut terminal_window = pollster::block_on(AppWindow::new(
        &window,
        terminal,
        builder.font_filename, builder.font_size,
    ))?;
    event_loop.run(move |event, target| {
        terminal_window.on_winit_event(event, target);
    })?;
    Ok(())
}

pub fn start_headless(builder: AppBuilder) -> anyhow::Result<()> {
    struct DummyWindow {}
    impl TerminalWindow for DummyWindow {
        fn on_window_action(&self, _action: WindowAction) {}
    }
    let mut terminal = Terminal::new(builder.process, Box::new(DummyWindow {}));

    terminal.set_size(Vector2::new(100,32));
    terminal.wait();
    let mut terminal_renderer = TerminalRenderer::default();
    let display = terminal.get_display();
    terminal_renderer.render_viewport(display.get_viewport());
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
