use clap::Parser;
use terminal::{
    terminal::Terminal, 
    parser::Parser as TerminalParser,
};
use std::io::{Write,Read};
use std::ops::DerefMut;
use std::sync::{Arc,Mutex};
use cgmath::Vector2;

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
    /// Mode
    #[arg(value_enum, long, default_value_t = Mode::default())]
    mode: Mode,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
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
    let mut terminal = Terminal::default();
    start_reader_thread(args.clone(), &mut terminal, &mut pipe_output);
    process.exit(0)?;
    drop(process);
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
    let mut terminal = Terminal::default();
    start_reader_thread(args.clone(), &mut terminal, &mut pipe_output);
    process.kill()?;
    drop(process);
    Ok(())
}

fn start_reader_thread(_args: Args, mut terminal: &mut Terminal, pipe_output: &mut impl Read) {
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
                parser.parse_bytes(data, terminal.deref_mut());
            }, 
            Err(err) => {
                log::error!("Error while reading child.stdout: {:?}", err);
                break;
            },
        };
    }
}
