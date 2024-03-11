use clap::Parser;
use terminal::{
    terminal::Terminal, 
    parser::Parser as TerminalParser,
};
use std::io::{Write,Read};
use std::ops::DerefMut;

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

    let mut terminal = Terminal::default();
    match args.mode { 
        Mode::Conpty => start_conpty(&args, &mut terminal),
        Mode::Raw => start_raw_shell(&args, &mut terminal),
    }?;
    let viewport = terminal.get_viewport();
    let size = viewport.get_size();
    let mut stdout = std::io::stdout();
    let mut tmp_buf = [0u8; 4];
    for y in 0..size.y {
        let (row, status) = viewport.get_row(y);
        for cell in &row[0..status.length] {
            let data = cell.character.encode_utf8(&mut tmp_buf);
            let _ = stdout.write(data.as_bytes());
        }
        if status.is_linebreak {
            let _ = stdout.write(b"\n");
        }
    }
    Ok(())
}

fn start_conpty(args: &Args, terminal: &mut Terminal) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(&args.filename);
    command.args(args.arguments.as_slice());
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut process = conpty::Process::spawn(command)?;
    let mut pipe_output = process.output()?;
    start_reader_thread(args.clone(), terminal, &mut pipe_output);
    let exit_code = process.wait(None)?;
    println!("conpty process exited with: {}", exit_code);
    Ok(())
}

fn start_raw_shell(args: &Args, terminal: &mut Terminal) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(&args.filename);
    command.args(args.arguments.as_slice());
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::null());
    let mut process = command.spawn()?;
    let mut pipe_output = process.stdout.take().ok_or("Failed to get pipe.stdout").map_err(anyhow::Error::msg)?;
    start_reader_thread(args.clone(), terminal, &mut pipe_output);
    let exit_code = process.wait()?;
    println!("raw process exited with: {:?}", exit_code);
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
