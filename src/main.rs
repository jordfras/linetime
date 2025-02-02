mod command;
mod error;
mod main_loop;
mod output;
mod token;

use crate::error::Result;
use crate::main_loop::MainLoop;
use crate::output::buffered::LineWriteDecorator;
use gumdrop::{Options, ParsingStyle};
use std::io::Write;
use std::sync::{Arc, Mutex};

#[derive(Debug, Options)]
struct ProgramOptions {
    #[options(short = "d", help = "show delta time from previous line to stream")]
    show_delta: bool,

    #[options(short = "c", help = "show control characters as unicode symbols")]
    show_control: bool,

    #[options(short = "e", help = "show ANSI escape sequences")]
    show_escape: bool,

    #[options(
        short = "u",
        help = "enable microseconds in timestamps and delta times"
    )]
    micros: bool,

    #[options(short = "l", help = "disable line buffering when executing command")]
    no_line_buffering: bool,

    #[options(help = "print help message")]
    help: bool,

    #[options(short = "t", help = "dump all tokens to stderr")]
    #[cfg(debug_assertions)]
    dump_tokens: bool,

    #[options(short = "f", help = "flush output after each token")]
    #[cfg(debug_assertions)]
    flush_all: bool,

    #[options(
        free,
        help = "command, with optional arguments, to execute and grab output from"
    )]
    command: Vec<String>,
}

impl From<&ProgramOptions> for output::Options {
    #[cfg(debug_assertions)]
    fn from(options: &ProgramOptions) -> Self {
        Self {
            show_delta: options.show_delta,
            microseconds: options.micros,
            prefix: String::new(),
            show_control: options.show_control,
            show_escape: options.show_escape,
            dump_tokens: options.dump_tokens,
            flush_all: options.flush_all,
        }
    }
    #[cfg(not(debug_assertions))]
    fn from(options: &ProgramOptions) -> Self {
        Self {
            show_delta: options.show_delta,
            microseconds: options.micros,
            prefix: String::new(),
            show_control: options.show_control,
            show_escape: options.show_escape,
            dump_tokens: false,
            flush_all: false,
        }
    }
}

fn show_help(program_name: &str) {
    println!("Usage: {program_name} [option ...] command [argument ...]");
    println!("       {program_name} [option ...] -- command [argument ...]");
    println!("       command [argument] | {program_name} [option ...]");
    println!("       command [argument] 2>&1 | {program_name} [option ...]");
    println!();
    println!("Reads from stdin or executes a command and grabs its output. Each line is");
    println!("prefixed with a timestamp. Unfolding is attempted when escape sequences ");
    println!("overwrite the current line. When the command is executed, output is buffered");
    println!("to ensure lines written to stdout and stderr are not interleaved.");
    println!();
    println!("{}", ProgramOptions::usage());
}

fn run_main_loop(options: ProgramOptions) -> Result<()> {
    if options.command.is_empty() {
        let mut stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        let mut main_loop = MainLoop::new((&options).into());
        main_loop.add_stream(&mut stdin, &mut stdout, "");
        main_loop.run()?;
    } else {
        // Mutex to ensure not writing lines to stdout and stderr at the same time
        let output_mutex = Arc::new(Mutex::new(()));
        let mut stdout = std::io::stdout();
        let mut stderr = std::io::stderr();
        let mut wrapped_stdout = LineWriteDecorator::new(&mut stdout, output_mutex.clone());
        let mut wrapped_stderr = LineWriteDecorator::new(&mut stderr, output_mutex);
        let maybe_wrapped_stdout: &mut (dyn Write + Send) = if options.no_line_buffering {
            &mut stdout
        } else {
            &mut wrapped_stdout
        };
        let maybe_wrapped_stderr: &mut (dyn Write + Send) = if options.no_line_buffering {
            &mut stderr
        } else {
            &mut wrapped_stderr
        };

        let mut command = command::Runner::new(&options.command);
        command.spawn()?;
        let mut command_stdout = command.stdout();
        let mut command_stderr = command.stderr();

        let mut main_loop = MainLoop::new((&options).into());
        main_loop.add_stream(&mut command_stdout, maybe_wrapped_stdout, "stdout");
        main_loop.add_stream(&mut command_stderr, maybe_wrapped_stderr, "stderr");

        main_loop.run()?;
        command.wait();
        command.exit_if_failed()?;
    };
    Ok(())
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if let Ok(options) = ProgramOptions::parse_args(&args[1..], ParsingStyle::StopAtFirstFree) {
        if options.help_requested() {
            show_help(args[0].as_str());
            return;
        }

        if let Err(error) = run_main_loop(options) {
            eprintln!("{error}");
            std::process::exit(1);
        }
    } else {
        show_help(args[0].as_str());
        eprintln!("\nProgram arguments could not be parsed");
        std::process::exit(1);
    }
}
