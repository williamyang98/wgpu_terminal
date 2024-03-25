use clap::Parser;
use terminal_process::*;
use wgpu_terminal::app::{AppBuilder, start_app, start_headless};
use std::sync::{Arc, Mutex};

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
    #[arg(default_value = "cmd.exe")]
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
        .with_level(log::LevelFilter::Error)
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
    let process = ConptyProcess::new(process);
    start_terminal(args.clone(), Arc::new(Mutex::new(Box::new(process))))?;
    Ok(())
}

fn start_raw_shell(args: &Args) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(&args.filename);
    command.args(args.arguments.as_slice());
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::null());
    let process = command.spawn()?;
    let process = RawProcess::new(process);
    start_terminal(args.clone(), Arc::new(Mutex::new(Box::new(process))))?;
    Ok(())
}

fn start_terminal(args: Args, process: Arc<Mutex<Box<dyn TerminalProcess + Send>>>) -> anyhow::Result<()> {
    let builder = AppBuilder {
        font_filename: args.font_filename.to_owned(),
        font_size: args.font_size,
        process,
    };
    if args.headless {
        start_headless(builder)
    } else {
        start_app(builder)
    }
}
