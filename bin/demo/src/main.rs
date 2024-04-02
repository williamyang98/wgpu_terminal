use clap::Parser;
use terminal_process::*;
use wgpu_terminal::app::{AppBuilder, start_app, start_headless};
use std::sync::{Arc, Mutex};

#[derive(Clone,Copy,Debug,Default,clap::ValueEnum)]
enum Mode {
    #[cfg_attr(not(any(windows, unix)), default)]
    Raw,
    #[cfg(windows)]
    #[default]
    Conpty,
    #[cfg(unix)]
    #[default]
    Pty,
}

#[derive(Clone,Debug,Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Filepath of shell executable
    #[cfg_attr(windows, arg(default_value = "cmd.exe"))]
    #[cfg_attr(unix, arg(default_value = "/usr/bin/bash"))]
    filename: String,
    /// Executable arguments
    arguments: Vec<String>,
    /// Font size
    #[arg(long, default_value_t = 14.0)]
    font_size: f32,
    /// Font filename 
    #[arg(long, default_value = "./res/Iosevka-custom-regular.ttf")]
    font_filename: String,
    /// Type of process to launch
    #[arg(value_enum, long, default_value_t = Mode::default())]
    mode: Mode,
    /// Run without window by printing results to stdout
    #[arg(long, default_value_t = false)]
    headless: bool,
    /// Show console window
    #[cfg(windows)]
    #[cfg_attr(debug_assertions, arg(long = "hide-console", default_value_t = true))]
    #[cfg_attr(not(debug_assertions), arg(long = "show-console", default_value_t = false))]
    show_console: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.font_size <= 1.0 {
        return Err(anyhow::format_err!("Font size must be greater than 1.0, got {:.2}", args.font_size));
    }

    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Error)
        .env()
        .with_colors(true)
        .without_timestamps()
        .init()?;

    match args.mode { 
        Mode::Raw => start_raw_shell(&args),
        #[cfg(unix)]
        Mode::Pty => start_unix_pty(&args),
        #[cfg(windows)]
        Mode::Conpty => start_conpty(&args),
    }
}

#[cfg(unix)]
fn start_unix_pty(args: &Args) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(&args.filename);
    command.args(args.arguments.as_slice());
    let process = unix_pty::process::PtyProcess::spawn(command, None)?;
    let process = UnixPtyProcess::new(process);
    start_terminal(args.clone(), Arc::new(Mutex::new(Box::new(process))))?;
    Ok(())
}

#[cfg(windows)]
fn start_conpty(args: &Args) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(&args.filename);
    command.args(args.arguments.as_slice());
    let process = conpty::process::ConptyProcess::spawn(command, None)?;
    let process = ConptyProcess::new(process);
    show_console_window(args.show_console);
    start_terminal(args.clone(), Arc::new(Mutex::new(Box::new(process))))?;
    Ok(())
}

#[cfg(windows)]
fn show_console_window(is_show: bool) {
    use windows::Win32::{
        System::Console::GetConsoleWindow,
        UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOW},
    };
    let window = unsafe { GetConsoleWindow() };
    let command = if is_show { SW_SHOW } else { SW_HIDE };
    let _ = unsafe { ShowWindow(window, command) };
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
