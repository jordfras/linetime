mod output;
mod token;

use crate::output::Printer;
use crate::token::{SerialTokenizer, Token};

fn main() {
    let mut stdin = std::io::stdin().lock();
    let mut tokenizer = SerialTokenizer::new(&mut stdin);
    let mut stdout = std::io::stdout().lock();
    let mut printer = Printer::new(&mut stdout);

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
                std::process::exit(1);
            }
        }
    }
}
