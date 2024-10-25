use crate::read_char::read_char;
use std::io::Read;

mod escape;

#[derive(PartialEq)]
pub enum Token {
    // A single character
    Char(char),
    CarriageReturn,
    LineFeed,
    // An ANSI escape sequence (starting with ESC)
    EscapeSequence(escape::Sequence),
    // End of file, i.e., end of input stream
    EndOfFile,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Char(c) => write!(f, "{c}"),
            Self::CarriageReturn => write!(f, "\u{240d}"),
            Self::LineFeed => {
                // Write \r to ensure starting on new line when handling output from Docker Windows
                // container in Linux
                write!(f, "\u{240a}\r\n")
            }
            Token::EscapeSequence(_) => write!(f, "<ESC>"),
            Token::EndOfFile => write!(f, "\u{2404}"),
        }
    }
}

pub struct SerialTokenizer<'a, R: Read> {
    stream: &'a mut R,
    /// A buffer for handling
    _escape_buf: String,
}

impl<'a, R: Read> SerialTokenizer<'a, R> {
    pub fn new(stream: &'a mut R) -> Self {
        Self {
            stream,
            _escape_buf: String::new(),
        }
    }

    pub fn next(&mut self) -> Result<Token, std::io::Error> {
        if let Some(c) = read_char(self.stream)? {
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
}
