mod error;
mod output;
mod token;

use crate::error::{ErrorWithContext, ResultExt};
use crate::output::timestamp::Timestamp;
use crate::output::Printer;
use crate::token::{SerialTokenizer, Token};
use gumdrop::{Options, ParsingStyle};
use std::io::Read;
use std::process::{Command, Stdio};

pub type Result<T> = std::result::Result<T, error::ErrorWithContext>;

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

fn loop_input<R: Read>(input: &mut R, output_options: output::Options) -> Result<()> {
    let mut tokenizer = SerialTokenizer::new(input);
    let mut stdout = std::io::stdout().lock();
    let mut printer = Printer::new(&mut stdout, Timestamp::new(), output_options);

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
    loop_input(&mut stdin, output_options)
}

fn loop_command_output(
    command_and_args: Vec<String>,
    output_options: output::Options,
) -> Result<()> {
    let mut child_process = Command::new(command_and_args[0].as_str())
        .args(&command_and_args[1..])
        .stdout(Stdio::piped())
        .spawn()
        .error_context("Failed to execute command")?;

    let mut child_out = child_process
        .stdout
        .take()
        .expect("Output expected to be piped");
    loop_input(&mut child_out, output_options)?;

    let status = child_process.wait().expect("Command expected to run");
    if !status.success() {
        if let Some(code) = status.code() {
            println!("Command exited with {code}");
            std::process::exit(code);
        } else {
            println!("Command terminated by signal");
        }
    }
    Ok(())
}

#[cfg(debug_assertions)]
fn output_options(options: &ProgramOptions) -> output::Options {
    output::Options {
        prefix: String::new(),
        show_control: options.show_control,
        show_escape: options.show_escape,
        dump_tokens: options.dump_tokens,
        flush_all: options.flush_all,
    }
}
#[cfg(not(debug_assertions))]
fn output_options(options: &ProgramOptions) -> output::Options {
    output::Options {
        prefix: String::new(),
        show_control: options.show_control,
        show_escape: options.show_escape,
        dump_tokens: false,
        flush_all: false,
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if let Ok(options) = ProgramOptions::parse_args(&args[1..], ParsingStyle::StopAtFirstFree) {
        if options.help_requested() {
            show_help(args[0].as_str());
            return;
        }

        let output_options = output_options(&options);

        if let Err(error) = if options.command.is_empty() {
            loop_stdin(output_options)
        } else {
            loop_command_output(options.command, output_options)
        } {
            eprintln!("{error}");
            std::process::exit(1);
        }
    } else {
        show_help(args[0].as_str());
        eprintln!("\nProgram arguments could not be parsed");
        std::process::exit(1);
    }
}
