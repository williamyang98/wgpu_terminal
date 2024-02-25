use clap::Parser;
use terminal::{
    terminal::Terminal, 
    parser::Parser as TerminalParser,
};
use std::io::{Write,Read};
use std::ops::DerefMut;
use std::sync::{Arc,Mutex};
use cgmath::Vector2;
use wgpu_terminal::{
    terminal_window::TerminalWindow,
    terminal_target::{TerminalTarget,ConptyTarget},
};

#[derive(Clone,Copy,Debug,Default,clap::ValueEnum)]
enum Mode {
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
        Mode::Conpty => start_conpty(&args),
        Mode::Raw => start_raw_shell(&args),
    }
}

fn start_conpty(args: &Args) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(&args.filename);
    command.args(args.arguments.as_slice());
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut process = conpty::Process::spawn(command)?;
    let mut pipe_input = process.input()?;
    let mut pipe_output = process.output()?;
    let mut conpty_target = ConptyTarget {
        process: &mut process,
        pipe_input: &mut pipe_input,
    };
    let terminal = Arc::new(Mutex::new(Terminal::new(Vector2::new(128,128))));
    let pipe_output_thread = std::thread::spawn({
        let terminal = terminal.clone();
        let args = args.clone();
        move || {
            start_reader_thread(args, terminal, &mut pipe_output);
        }
    });
    start_render_thread(args.clone(), terminal.clone(), &mut conpty_target)?;
    process.exit(0)?;
    drop(process);
    let _ = pipe_output_thread.join();
    Ok(())
}

fn start_raw_shell(args: &Args) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(&args.filename);
    command.args(args.arguments.as_slice());
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::null());
    let mut process = command.spawn()?;
    let mut pipe_input = process.stdin.take().ok_or("Failed to get pipe.stdin").map_err(anyhow::Error::msg)?;
    let mut pipe_output = process.stdout.take().ok_or("Failed to get pipe.stdout").map_err(anyhow::Error::msg)?;
    let terminal = Arc::new(Mutex::new(Terminal::new(Vector2::new(128,128))));
    let pipe_output_thread = std::thread::spawn({
        let terminal = terminal.clone();
        let args = args.clone();
        move || {
            start_reader_thread(args, terminal, &mut pipe_output);
        }
    });
    start_render_thread(args.clone(), terminal.clone(), &mut pipe_input)?;
    process.kill()?;
    drop(process);
    let _ = pipe_output_thread.join();
    Ok(())
}

fn start_reader_thread(_args: Args, terminal: Arc<Mutex<Terminal>>, pipe_output: &mut impl Read) {
    const BLOCK_SIZE: usize = 8192;
    let mut parser = TerminalParser::default();
    let mut buffer = vec![0u8; BLOCK_SIZE];
    loop {
        match pipe_output.read(buffer.as_mut_slice()) {
            Ok(0) => {
                log::info!("Closing child.stdout after reading 0 bytes");
                break;
            },
            Ok(total_read) => {
                let data = &buffer[0..total_read];
                match terminal.lock() {
                    Ok(ref mut terminal) => parser.parse_bytes(data, terminal.deref_mut()),
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

fn start_render_thread(
    args: Args,
    terminal: Arc<Mutex<Terminal>>, 
    terminal_target: &mut impl TerminalTarget,
) -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = winit::window::WindowBuilder::new().build(&event_loop)?;
    let mut window_size = window.inner_size();
    window_size.width = window_size.width.max(1);
    window_size.height = window_size.height.max(1);
    let mut terminal_window = pollster::block_on(TerminalWindow::new(
        &window,
        terminal,
        terminal_target,
        args.font_filename.to_owned(), args.font_size,
    ))?;
    event_loop.run(move |event, target| {
        terminal_window.on_winit_event(event, target);
    })?;
    Ok(())
}
