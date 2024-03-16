use clap::Parser;
use cgmath::Vector2;
use terminal::terminal::Terminal;
use terminal_process::*;
use std::io::Write;
use wgpu_terminal::terminal_window::TerminalWindow;

#[derive(Clone,Copy,Debug,Default,clap::ValueEnum)]
enum Mode {
    #[cfg(unix)]
    #[default]
    Pty,
    #[cfg(windows)]
    #[default]
    Conpty,
    Raw,
}

#[derive(Clone,Debug,Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Filepath of shell executable
    filename: String,
    /// Filepath arguments
    arguments: Vec<String>,
    /// Font size
    #[arg(long, default_value_t = 14.0)]
    font_size: f32,
    /// Font filename 
    #[arg(long, default_value = "./res/Iosevka-custom-regular.ttf")]
    font_filename: String,
    /// Mode
    #[arg(value_enum, long, default_value_t = Mode::default())]
    mode: Mode,
    /// Headless
    #[arg(long, default_value_t = false)]
    headless: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.font_size <= 1.0 {
        return Err(anyhow::format_err!("Font size must be greater than 1.0, got {:.2}", args.font_size));
    }

    simple_logger::SimpleLogger::new()
        .env()
        .with_colors(true)
        .without_timestamps()
        .init()?;

    match args.mode { 
        Mode::Raw => start_raw_shell(&args),
        #[cfg(unix)]
        Mode::Pty => start_pty(&args),
        #[cfg(windows)]
        Mode::Conpty => start_conpty(&args),
    }
}

#[cfg(unix)]
fn start_pty(args: &Args) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn start_conpty(args: &Args) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(&args.filename);
    command.args(args.arguments.as_slice());
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let process = conpty::Process::spawn(command)?;
    let process = Box::new(ConptyProcess::new(process));
    let terminal = Terminal::new(process);
    start_terminal(args.clone(), terminal)?;
    Ok(())
}

fn start_raw_shell(args: &Args) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(&args.filename);
    command.args(args.arguments.as_slice());
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::null());
    let process = command.spawn()?;
    let process = Box::new(RawProcess::new(process));
    let terminal = Terminal::new(process);
    start_terminal(args.clone(), terminal)?;
    Ok(())
}

fn start_terminal(args: Args, terminal: Terminal) -> anyhow::Result<()> {
    if args.headless {
        start_headless(args, terminal)
    } else {
        start_render_thread(args, terminal)
    }
}

fn start_render_thread(args: Args, terminal: Terminal) -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = winit::window::WindowBuilder::new().build(&event_loop)?;
    let mut window_size = window.inner_size();
    window_size.width = window_size.width.max(1);
    window_size.height = window_size.height.max(1);
    let mut terminal_window = pollster::block_on(TerminalWindow::new(
        &window,
        terminal,
        args.font_filename.to_owned(), args.font_size,
    ))?;
    event_loop.run(move |event, target| {
        terminal_window.on_winit_event(event, target);
    })?;
    Ok(())
}

fn start_headless(_args: Args, mut terminal: Terminal) -> anyhow::Result<()> {
    terminal.set_size(Vector2::new(100,32));
    terminal.wait();
    if !terminal.try_render() {
        log::error!("Failed to render terminal after closing");
    }
    let renderer = terminal.get_renderer();
    let size = renderer.get_size();
    let cells = renderer.get_cells();
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
