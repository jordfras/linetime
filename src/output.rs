use crate::token::escape;
use crate::token::Token;
use std::collections::VecDeque;
use std::io::Write;
use std::time::Duration;

mod timestamp;
use crate::output::timestamp::Timestamp;

pub struct Printer<'a, W: Write> {
    stream: &'a mut W,
    timestamp: Timestamp,
    start_of_stream: bool,
    start_of_line: bool,
    break_tokens: VecDeque<Token>,
}

impl<'a, W: Write> Printer<'a, W> {
    pub fn new(stream: &'a mut W) -> Self {
        Self {
            stream,
            timestamp: Timestamp::new(),
            start_of_stream: true,
            start_of_line: true,
            break_tokens: VecDeque::new(),
        }
    }

    pub fn print(&mut self, token: &Token) -> Result<(), std::io::Error> {
        if Self::causes_soft_break(token) {
            self.break_tokens.push_back(token.clone());
        } else if !self.break_tokens.is_empty() && *token != Token::LineFeed {
            self.newline()?;
        }

        if self.start_of_line {
            self.timestamp()?;
        }

        self.print_token(token)?;
        if *token == Token::LineFeed {
            self.newline()?;
        }

        self.start_of_stream = false;
        Ok(())
    }

    fn causes_soft_break(token: &Token) -> bool {
        match token {
            // Ensure new line to handle cases where CR is used to overwrite the same line over
            // and over again. We want to see all input.
            Token::CarriageReturn => true,
            Token::EscapeSequence(sequence) => {
                // Unhandled escape sequences are just forwarded, as-is
                sequence.command != escape::SequenceCommand::Unhandled
            }
            _ => false,
        }
    }

    fn print_str(&mut self, s: &str) -> Result<(), std::io::Error> {
        self.stream.write_all(s.as_ref())
    }

    fn print_token(&mut self, token: &Token) -> Result<(), std::io::Error> {
        match token {
            Token::Char(c) => {
                let mut buffer: [u8; 4] = [0; 4];
                self.print_str(c.encode_utf8(&mut buffer))
            }
            Token::CarriageReturn => self.print_str("\u{240d}"),
            Token::LineFeed => self.print_str("\u{240a}"),
            Token::EscapeSequence(sequence) => {
                if sequence.command == escape::SequenceCommand::Unhandled {
                    self.print_str(sequence.text.as_str())
                } else {
                    self.print_str("\u{241b}")?;
                    self.print_str(&sequence.text[1..])
                }
            }
            Token::EndOfFile => self.print_str("\u{2404}\n"),
        }
    }

    fn newline(&mut self) -> Result<(), std::io::Error> {
        while let Some(token) = self.break_tokens.pop_front() {
            if token == Token::CarriageReturn {
                self.print_str("\r")?;
            }
        }

        if !self.start_of_stream {
            self.print_str("\n")?;
        }

        self.start_of_line = true;
        Ok(())
    }

    fn timestamp(&mut self) -> Result<(), std::io::Error> {
        let t = self.timestamp.get();
        self.print_str(format(t).as_str())?;
        self.print_str(": ")?;
        self.start_of_line = false;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_printed {
        ($stream:ident, $text:expr) => {
            let text = format!("{}", $text);
            assert_eq!(text, std::str::from_utf8(&$stream[..]).unwrap());
        };

        ($stream:ident, $($text:expr),+) => {
            let mut text = String::new();
            $(
                text += $text;
            )+
            assert_printed!($stream, text.as_str());
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
    fn timestamp_is_added_at_beginning_of_lines() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(&mut stream);

        printer.timestamp.expect_get(Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();

        printer.timestamp.expect_empty();
        assert_printed!(stream, "00:03.000: A");
    }

    #[test]
    fn timestamp_is_requested_for_first_token_on_line() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(&mut stream);

        printer.timestamp.expect_get(Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::LineFeed).unwrap();

        // Timestamp is not request until first token on new line is received
        printer.timestamp.expect_get(Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        printer.timestamp.expect_empty();
        assert_printed!(stream, "00:03.000: A\u{240a}\n", "00:04.000: B");
    }

    #[test]
    fn overwriting_line_with_cr_is_unfolded() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(&mut stream);

        printer.timestamp.expect_get(Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::CarriageReturn).unwrap();

        printer.timestamp.expect_get(Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        printer.timestamp.expect_empty();
        assert_printed!(stream, "00:03.000: A\u{240d}\r\n", "00:04.000: B");
    }

    #[test]
    fn cr_lf_causes_only_one_newline_but_cr_is_forwarded() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(&mut stream);

        printer.timestamp.expect_get(Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::CarriageReturn).unwrap();
        printer.print(&Token::LineFeed).unwrap();

        printer.timestamp.expect_get(Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        printer.timestamp.expect_empty();
        assert_printed!(stream, "00:03.000: A\u{240d}\u{240a}\r\n", "00:04.000: B");
    }

    #[test]
    fn multiples_new_lines_are_handled() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(&mut stream);

        printer.timestamp.expect_get(Duration::from_secs(3));
        printer.print(&Token::CarriageReturn).unwrap();
        printer.print(&Token::LineFeed).unwrap();

        printer.timestamp.expect_get(Duration::from_secs(4));
        printer.print(&Token::CarriageReturn).unwrap();
        printer.print(&Token::LineFeed).unwrap();

        printer.timestamp.expect_get(Duration::from_secs(5));
        printer.print(&Token::LineFeed).unwrap();

        printer.timestamp.expect_get(Duration::from_secs(6));
        printer.print(&Token::LineFeed).unwrap();

        printer.timestamp.expect_empty();
        assert_printed!(
            stream,
            "00:03.000: \u{240d}\u{240a}\r\n",
            "00:04.000: \u{240d}\u{240a}\r\n",
            "00:05.000: \u{240a}\n",
            "00:06.000: \u{240a}\n"
        );
    }

    #[test]
    fn cr_escape_erase_to_end_of_line_is_unfolded() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(&mut stream);

        printer.timestamp.expect_get(Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::CarriageReturn).unwrap();
        printer
            .print(&esc_token!(
                escape::SequenceCommand::EraseFromCursorToEndOfLine,
                "\x1b[K"
            ))
            .unwrap();

        printer.timestamp.expect_get(Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        printer.timestamp.expect_empty();
        assert_printed!(stream, "00:03.000: A\u{240d}\u{241b}[K\r\n", "00:04.000: B");
    }

    #[test]
    fn escape_erase_entire_line_is_unfolded() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(&mut stream);

        printer.timestamp.expect_get(Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer
            .print(&esc_token!(
                escape::SequenceCommand::EraseEntireLine,
                "\x1b[2K"
            ))
            .unwrap();

        printer.timestamp.expect_get(Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        printer.timestamp.expect_empty();
        assert_printed!(stream, "00:03.000: A\u{241b}[2K\n", "00:04.000: B");
    }

    #[test]
    fn escape_coloring_is_unchanged() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(&mut stream);

        printer.timestamp.expect_get(Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer
            .print(&esc_token!(escape::SequenceCommand::Unhandled, "\x1b[31m"))
            .unwrap();
        printer.print(&Token::Char('B')).unwrap();

        printer.timestamp.expect_empty();
        assert_printed!(stream, "00:03.000: A\x1b[31mB");
    }
}
