mod command;
mod error;
mod output;
mod token;

use crate::error::{ErrorWithContext, Result, ResultExt};
use crate::output::buffered::LineWriteDecorator;
use crate::output::timestamp::Timestamp;
use crate::output::Printer;
use crate::token::{SerialTokenizer, Token};
use gumdrop::{Options, ParsingStyle};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Debug, Options)]
struct ProgramOptions {
    #[options(short = "c", help = "show control characters as unicode symbols")]
    show_control: bool,

    #[options(short = "e", help = "show ANSI escape sequences")]
    show_escape: bool,

    #[options(help = "print help message")]
    help: bool,

    #[options(short = "d", help = "dump all tokens to stderr")]
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
    println!("Reads from stdin and prefixes each line with a timestamp.");
    println!("Unfolding is attempted for input trying to ovewrite the current line.");
    println!();
    println!("{}", ProgramOptions::usage());
}

fn loop_input(
    input_stream: &mut dyn Read,
    output_stream: &mut dyn Write,
    timestamp: Timestamp,
    output_options: output::Options,
) -> Result<()> {
    let mut tokenizer = SerialTokenizer::new(input_stream);
    let mut printer = Printer::new(output_stream, timestamp, output_options);

    loop {
        match tokenizer.next() {
            Ok(token) => {
                printer
                    .print(&token)
                    .error_context("Error writing to stdout")?;
                if token == Token::EndOfFile {
                    break;
                }
            }
            Err(error) => {
                return Err(ErrorWithContext::wrap("Error reading input", error));
            }
        }
    }
    Ok(())
}

fn loop_stdin(output_options: output::Options) -> Result<()> {
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    loop_input(&mut stdin, &mut stdout, Timestamp::new(), output_options)
}

#[derive(PartialEq)]
enum OutputStream {
    StdOut,
    StdErr,
}

fn loop_in_thread<R: Read + Send + 'static>(
    mut child_out: R,
    output_type: OutputStream,
    timestamp: Timestamp,
    output_mutex: Arc<Mutex<()>>,
    mut output_options: output::Options,
) -> JoinHandle<Result<()>> {
    output_options.prefix = if output_type == OutputStream::StdOut {
        "stdout".to_string()
    } else {
        "stderr".to_string()
    };
    thread::spawn(move || {
        let output_stream: &mut dyn Write = if output_type == OutputStream::StdOut {
            &mut std::io::stdout()
        } else {
            &mut std::io::stderr()
        };
        loop_input(
            &mut child_out,
            &mut LineWriteDecorator::new(output_stream, output_mutex),
            timestamp,
            output_options,
        )
    })
}

fn loop_command_output(
    command_and_args: &Vec<String>,
    output_options: output::Options,
) -> Result<()> {
    // Create one instance that can be cloned to ensure starting with same reference time
    let timestamp = Timestamp::new();
    let mut command = command::Runner::new(command_and_args);
    command.spawn()?;

    // Mutex to ensure not writing lines to stdout and stderr at the same time
    let output_mutex = Arc::new(Mutex::new(()));
    let thread_out = loop_in_thread(
        command.stdout(),
        OutputStream::StdOut,
        timestamp.clone(),
        output_mutex.clone(),
        output_options.clone(),
    );

    let thread_err = loop_in_thread(
        command.stderr(),
        OutputStream::StdErr,
        timestamp,
        output_mutex,
        output_options,
    );

    command.wait();

    thread_out
        .join()
        .expect("Thread reading stdout unexpectedly panicked")?;
    thread_err
        .join()
        .expect("Thread reading stderr unexpectedly panicked")?;

    command.exit_if_failed()
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if let Ok(options) = ProgramOptions::parse_args(&args[1..], ParsingStyle::StopAtFirstFree) {
        if options.help_requested() {
            show_help(args[0].as_str());
            return;
        }

        let result = if options.command.is_empty() {
            loop_stdin((&options).into())
        } else {
            loop_command_output(&options.command, (&options).into())
        };

        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
    } else {
        show_help(args[0].as_str());
        eprintln!("\nProgram arguments could not be parsed");
        std::process::exit(1);
    }
}
