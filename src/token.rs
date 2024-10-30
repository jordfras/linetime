use crate::read_char::read_char;
use std::io::Read;

pub mod escape;

#[derive(Clone, Debug, PartialEq)]
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

impl Token {
    /// Create token from a char without taking escape sequences into account
    fn from_single_char(c: char) -> Self {
        match c {
            '\r' => Self::CarriageReturn,
            '\n' => Self::LineFeed,
            _ => Self::Char(c),
        }
    }
}

/// A tokenizer consuming a stream of characters.
///
/// The tokenizer can detect multi-character ANSI escape sequences and tokenize them as single
/// characters. It serially reads characters rather than consume complete lines to be able to
/// detect escaoe sequences before a newline (where stdout is usually flushed). Escape sequences
/// are used to overwrite the same line several times but this tool wants to detect this and
/// show all output,
pub struct SerialTokenizer<'a, R: Read> {
    stream: &'a mut R,
    /// A buffer to hold characters while detecting ANSI escape sequences
    escape_buf: String,
}

impl<'a, R: Read> SerialTokenizer<'a, R> {
    pub fn new(stream: &'a mut R) -> Self {
        Self {
            stream,
            escape_buf: String::with_capacity(32),
        }
    }

    /// Gets the next token from the stream
    pub fn next(&mut self) -> Result<Token, std::io::Error> {
        if self.escape_buf.is_empty() {
            if let Some(c) = read_char(self.stream)? {
                if c == escape::ESC {
                    self.escape_buf.push(c);
                    self.detect_and_get_escape()
                } else {
                    Ok(Token::from_single_char(c))
                }
            } else {
                Ok(Token::EndOfFile)
            }
        } else if self.escape_buf.chars().next().unwrap() == escape::ESC {
            self.detect_and_get_escape()
        } else {
            Ok(self.take_char_from_buffer())
        }
    }

    fn detect_and_get_escape(&mut self) -> Result<Token, std::io::Error> {
        assert!(!self.escape_buf.is_empty());
        while let Some(c) = read_char(self.stream)? {
            self.escape_buf.push(c);
            if c.is_control() {
                // Control character, e.g., newline. This can't be part of ANSI escape sequence and
                // we don't want to read further since it might block unnecessarily, e.g., if an
                // application outputs to stdout it is usually flushed at newlines and we don't
                // want to wait for a complete extra line.
                break;
            }
            if let Some(sequence) = escape::Sequence::from(self.escape_buf.as_str()) {
                self.escape_buf.clear();
                return Ok(Token::EscapeSequence(sequence));
            }
        }

        Ok(self.take_char_from_buffer())
    }

    fn take_char_from_buffer(&mut self) -> Token {
        assert!(!self.escape_buf.is_empty());
        let c = self.escape_buf.chars().next().unwrap();
        // TODO: Can we use some other type to avoid copying strings?
        self.escape_buf.remove(0);
        Token::from_single_char(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Command = escape::SequenceCommand;

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

    macro_rules! esc_token {
        ($command:expr, $text: expr) => {
            Token::EscapeSequence(escape::Sequence {
                command: $command,
                text: $text.to_string(),
            })
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
    fn line_breaks_are_tokenized_as_cr_and_lf() {
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
    fn sole_escape_sequence_is_tokenized_as_such() {
        let mut stream = stream!("\x1b[H");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, esc_token!(Command::CursorMoveHome, "\x1b[H"));
        assert_next!(tokenizer, Token::EndOfFile);
    }

    #[test]
    fn consecutive_escape_sequences_are_tokenized_as_such() {
        let mut stream = stream!("\x1b[H\x1bM");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, esc_token!(Command::CursorMoveHome, "\x1b[H"));
        assert_next!(tokenizer, esc_token!(Command::CursorMoveUpOne, "\x1bM"));
        assert_next!(tokenizer, Token::EndOfFile);
    }

    #[test]
    fn escape_sequence_in_text_is_tokenized_as_such() {
        let mut stream = stream!("1\x1b[17;42f2");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, Token::Char('1'));
        assert_next!(
            tokenizer,
            esc_token!(Command::CursorMoveToLineAndColumn((17, 42)), "\x1b[17;42f")
        );
        assert_next!(tokenizer, Token::Char('2'));
        assert_next!(tokenizer, Token::EndOfFile);
    }

    #[test]
    fn escape_that_is_not_sequence_is_tokenized_as_char() {
        let mut stream = stream!("\x1b[1");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, Token::Char(escape::ESC));
        assert_next!(tokenizer, Token::Char('['));
        assert_next!(tokenizer, Token::Char('1'));
        assert_next!(tokenizer, Token::EndOfFile);
    }

    #[test]
    fn escape_that_is_not_sequence_near_newline_is_tokenized_as_char() {
        let mut stream = stream!("\x1b[\n");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, Token::Char(escape::ESC));
        assert_next!(tokenizer, Token::Char('['));
        assert_next!(tokenizer, Token::LineFeed);
        assert_next!(tokenizer, Token::EndOfFile);
    }

    #[test]
    fn other_special_characters_are_tokenized_as_char() {
        let mut stream = stream!("\t\0\\");
        let mut tokenizer = SerialTokenizer::new(&mut stream);
        assert_next!(tokenizer, Token::Char('\t'));
        assert_next!(tokenizer, Token::Char('\0'));
        assert_next!(tokenizer, Token::Char('\\'));
        assert_next!(tokenizer, Token::EndOfFile);
    }
}
