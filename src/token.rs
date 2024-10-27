use crate::read_char::read_char;
use std::io::Read;

mod escape;

#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // Creates a string slice just containing an ANSI escape sequence
    macro_rules! stream {
        ($str:expr) => {
            std::io::Cursor::new($str.as_bytes())
        };
    }

    macro_rules! assert_next {
        ($tokenizer:ident, $token:expr) => {
            assert_eq!($token, $tokenizer.next().unwrap())
        };
    }

    #[test]
    fn text_is_tokenized_as_chars() {
        let mut stream = stream!("text");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, Token::Char('t'));
        assert_next!(tokenizer, Token::Char('e'));
        assert_next!(tokenizer, Token::Char('x'));
        assert_next!(tokenizer, Token::Char('t'));
        assert_next!(tokenizer, Token::EndOfFile);
    }

    #[test]
    fn eof_is_repeatedly_returned() {
        let mut stream = stream!("t");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, Token::Char('t'));
        assert_next!(tokenizer, Token::EndOfFile);
        assert_next!(tokenizer, Token::EndOfFile);
        assert_next!(tokenizer, Token::EndOfFile);
    }

    #[test]
    fn line_breaks_are_tokenized_with_cr_and_lf() {
        let mut stream = stream!("1\n2\r\n");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, Token::Char('1'));
        assert_next!(tokenizer, Token::LineFeed);
        assert_next!(tokenizer, Token::Char('2'));
        assert_next!(tokenizer, Token::CarriageReturn);
        assert_next!(tokenizer, Token::LineFeed);
        assert_next!(tokenizer, Token::EndOfFile);
    }

    #[test]
    fn other_special_characters_are_tokenize_as_char() {
        let mut stream = stream!("\t\0\\");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, Token::Char('\t'));
        assert_next!(tokenizer, Token::Char('\0'));
        assert_next!(tokenizer, Token::Char('\\'));
        assert_next!(tokenizer, Token::EndOfFile);
    }
}
