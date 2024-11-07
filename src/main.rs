mod output;
mod token;

use crate::output::Printer;
use crate::token::{SerialTokenizer, Token};
use gumdrop::{Options, ParsingStyle};

#[derive(Debug, Options)]
#[options(help = "Reads from stdin and prefixes each line with a timestamp.\n\
            Unfolding is attempted for input trying to ovewrite the current line.")]
struct ProgramOptions {
    #[options(short = "c", help = "show control characters as unicode symbols")]
    show_control: bool,

    #[options(short = "e", help = "show ANSI escape sequences")]
    show_escape: bool,

    #[options(help = "print help message")]
    help: bool,
}

fn loop_input(output_options: output::Options) {
    let mut stdin = std::io::stdin().lock();
    let mut tokenizer = SerialTokenizer::new(&mut stdin);
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
                eprintln!("Error reading from stdin: {error}");
                std::process::exit(3);
            }
        }
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if let Ok(options) = ProgramOptions::parse_args(&args[1..], ParsingStyle::StopAtFirstFree) {
        if options.help_requested() {
            println!("Usage: {} [option ...] command [argument ...]", args[0]);
            println!("{}", ProgramOptions::usage());
            return;
        }
        loop_input(output::Options {
            show_control: options.show_control,
            show_escape: options.show_escape,
        });
    } else {
        eprintln!("Program arguments could not be parsed");
        std::process::exit(1);
    }
}
