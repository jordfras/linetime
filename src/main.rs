mod output;
mod token;

use crate::output::Printer;
use crate::token::{SerialTokenizer, Token};
use gumdrop::{Options, ParsingStyle};
use std::io::Read;
use std::process::{Command, Stdio};

#[derive(Debug, Options)]
struct ProgramOptions {
    #[options(short = "c", help = "show control characters as unicode symbols")]
    show_control: bool,

    #[options(short = "e", help = "show ANSI escape sequences")]
    show_escape: bool,

    #[options(help = "print help message")]
    help: bool,

    #[options(
        free,
        help = "command, with optional arguments, to execute and grab output from"
    )]
    command: Vec<String>,
}

fn show_help(program_name: &str) {
    println!("Usage: {program_name} [option ...] command [argument ...]");
    println!("       {program_name} [option ...] -- command [argument ...]");
    println!("       command [argument] > {program_name} [option ...]");
    println!("       command [argument] 2>&1 {program_name} [option ...]");
    println!();
    println!("Reads from stdin and prefixes each line with a timestamp.");
    println!("Unfolding is attempted for input trying to ovewrite the current line.");
    println!();
    println!("{}", ProgramOptions::usage());
}

fn loop_input<R: Read>(input: &mut R, output_options: output::Options) {
    let mut tokenizer = SerialTokenizer::new(input);
    let mut stdout = std::io::stdout().lock();
    let mut printer = Printer::new(&mut stdout, output_options);

    loop {
        match tokenizer.next() {
            Ok(token) => {
                if let Err(error) = printer.print(&token) {
                    eprintln!("Error writing to stdout: {error}");
                    std::process::exit(2);
                }
                if token == Token::EndOfFile {
                    break;
                }
            }
            Err(error) => {
                eprintln!("Error reading input: {error}");
                std::process::exit(3);
            }
        }
    }
}

fn loop_stdin(output_options: output::Options) {
    let mut stdin = std::io::stdin().lock();
    loop_input(&mut stdin, output_options);
}

fn loop_command_output(
    command_and_args: Vec<String>,
    output_options: output::Options,
) -> Result<(), std::io::Error> {
    let child_process = Command::new(command_and_args[0].as_str())
        .args(&command_and_args[1..])
        .stdout(Stdio::piped())
        .spawn()?;

    let mut child_out = child_process.stdout.expect("Output expected to be piped");
    loop_input(&mut child_out, output_options);
    Ok(())
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if let Ok(options) = ProgramOptions::parse_args(&args[1..], ParsingStyle::StopAtFirstFree) {
        if options.help_requested() {
            show_help(args[0].as_str());
            return;
        }

        let output_options = output::Options {
            show_control: options.show_control,
            show_escape: options.show_escape,
        };

        if options.command.is_empty() {
            loop_stdin(output_options);
        } else if let Err(error) = loop_command_output(options.command, output_options) {
            eprintln!("Failed to execute command: {error}");
            std::process::exit(1);
        }
    } else {
        show_help(args[0].as_str());
        eprintln!("\nProgram arguments could not be parsed");
        std::process::exit(1);
    }
}
