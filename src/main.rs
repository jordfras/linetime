use std::io::Read;
use std::time::{Duration, SystemTime};

mod read_char;
use crate::read_char::read_char;

#[derive(PartialEq)]
enum EscapeCursorMove {
    Home,
    ToLineAndColumn((u32, u32)),
    LinesUp(u32),
    LinesDown(u32),
    ColumnsRight(u32),
    ColumnsLeft(u32),
    UpOne,
    SavePosition,
    RestorePosition,
}

#[derive(PartialEq)]
enum EscapeErase {
    FromCursorToEndOfScreen,
    FromCursorToBeginningOfScreen,
    EntireScreen,
    SavedLine,
    FromCursorToEndOfLine,
    StartOfLineToCursor,
    EntireLine,
}

#[derive(PartialEq)]
enum Token {
    // A single character
    Char(char),
    CarriageReturn,
    LineFeed,
    // An ANSI escape sequence (starting with ESC) to move cursor
    EscapeMoveSequence(EscapeCursorMove),
    // An ANSI escape sequence (starting with ESC) to erase
    EscapeEraseSequence(EscapeErase),
    // End of file, i.e., end of input stream
    EndOfFile,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Char(c) => write!(f, "{c}"),
            Token::CarriageReturn => write!(f, "\u{240d}"),
            Token::LineFeed => {
                // Write \r to ensure starting on new line when handling output from Docker Windows
                // container in Linux
                write!(f, "\u{240a}\r\n")
            }
            Token::EscapeMoveSequence(_) => write!(f, "<MOVE>"),
            Token::EscapeEraseSequence(_) => write!(f, "<ERASE>"),
            Token::EndOfFile => write!(f, "\u{2404}"),
        }
    }
}

fn read_token(stream: &mut impl Read) -> Result<Token, std::io::Error> {
    if let Some(c) = read_char(stream)? {
        Ok(match c {
            '\r' => Token::CarriageReturn,
            '\n' => Token::LineFeed,
            // TODO: Escape sequencees
            _ => Token::Char(c),
        })
    } else {
        Ok(Token::EndOfFile)
    }
}

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
    loop {
        match read_token(&mut stdin) {
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
