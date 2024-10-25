use std::time::{Duration, SystemTime};

mod read_char;
mod token;

use crate::token::{SerialTokenizer, Token};

fn format(duration: Duration) -> String {
    format!(
        "{:0>2}:{:0>2}.{:0>3}",
        duration.as_secs() / 60,
        duration.as_secs() % 60,
        duration.subsec_millis()
    )
}

fn main() {
    let start_time = SystemTime::now();
    let mut stdin = std::io::stdin().lock();

    let mut start_of_line = true;
    let mut tokenizer = SerialTokenizer::new(&mut stdin);
    loop {
        match tokenizer.next() {
            Ok(Token::EndOfFile) => {
                println!("{}", Token::EndOfFile);
                break;
            }
            Ok(token) => {
                if start_of_line {
                    print!(
                        "{}: ",
                        format(SystemTime::now().duration_since(start_time).unwrap())
                    );
                }
                print!("{}", token);
                start_of_line = token == Token::LineFeed;
            }
            Err(error) => {
                eprintln!("Error reading from stdin: {error}");
                std::process::exit(1);
            }
        }
    }
}
