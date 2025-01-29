use crate::token::escape;
use crate::token::Token;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub mod buffered;
pub mod timestamp;

use self::timestamp::Timestamp;

#[derive(Clone)]
pub struct Options {
    /// Show delta time since previous line
    pub show_delta: bool,
    /// Prefix added to start of each line together with a timestamp
    pub prefix: String,
    /// Show control characters as unicode symbols
    pub show_control: bool,
    /// Show handled escape sequences as string with unciode symbol for the escape character
    pub show_escape: bool,
    /// Dump each token to stderr
    pub dump_tokens: bool,
    /// Flush output stream after each token
    pub flush_all: bool,
}

pub struct Printer<'a> {
    stream: &'a mut (dyn Write + Send),
    options: Options,

    timestamp: Arc<Mutex<Timestamp>>,
    previous_time: Option<Duration>,
    start_of_line: bool,
    break_tokens: VecDeque<Token>,
}

impl<'a> Printer<'a> {
    pub fn new(
        stream: &'a mut (dyn Write + Send),
        timestamp: Arc<Mutex<Timestamp>>,
        options: Options,
    ) -> Self {
        Self {
            stream,
            options,
            timestamp,
            previous_time: None,
            start_of_line: true,
            break_tokens: VecDeque::new(),
        }
    }

    pub fn print(&mut self, token: &Token) -> Result<(), std::io::Error> {
        if self.options.dump_tokens {
            eprintln!("{:?}", token);
        }

        if Self::causes_soft_break(token) {
            self.break_tokens.push_back(token.clone());
        } else if !self.break_tokens.is_empty() && *token != Token::LineFeed {
            // Soft break triggers newline when not followed by a linefeed, to unwrap lines
            // otherwise being overwritten in the terminal
            self.newline()?;
        }

        if self.start_of_line {
            self.timestamp()?;
        } else if *token == Token::EndOfFile {
            // Ensure EOF is always written on a new line with its own timestamp, even if an
            // explicit linefeed token was not received before
            self.print_str("\n")?;
            self.timestamp()?;
        }

        self.print_token(token)?;
        if *token == Token::LineFeed {
            self.newline()?;
        }
        if self.options.flush_all {
            self.stream.flush()?;
        }

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

    fn print_control(&mut self, s: &str) -> Result<(), std::io::Error> {
        if self.options.show_control {
            self.print_str(s)?;
        }
        Ok(())
    }

    fn print_escape(&mut self, s: &str) -> Result<(), std::io::Error> {
        if self.options.show_escape {
            self.print_str(s)?;
        }
        Ok(())
    }

    fn print_token(&mut self, token: &Token) -> Result<(), std::io::Error> {
        match token {
            Token::Char(c) => {
                let mut buffer: [u8; 4] = [0; 4];
                self.print_str(c.encode_utf8(&mut buffer))
            }
            Token::CarriageReturn => self.print_control("\u{240d}"),
            Token::LineFeed => self.print_control("\u{240a}"),
            Token::EscapeSequence(sequence) => {
                if sequence.command == escape::SequenceCommand::Unhandled {
                    self.print_str(sequence.text.as_str())
                } else {
                    self.print_escape("\u{241b}")?;
                    self.print_escape(&sequence.text[1..])
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

        self.print_str("\n")?;
        self.start_of_line = true;
        Ok(())
    }

    fn timestamp(&mut self) -> Result<(), std::io::Error> {
        let t = self
            .timestamp
            .lock()
            .map_err(|_| {
                std::io::Error::other("Could not lock timestamp mutex, other thread panicked!")
            })?
            .get();
        self.print_str(format(t).as_str())?;
        if self.options.show_delta {
            self.print_str(
                if let Some(previous) = self.previous_time {
                    let delta = t - previous;
                    format!(" ({})", format(delta))
                } else {
                    "            ".to_string()
                }
                .as_str(),
            )?;
        }
        if !self.options.prefix.is_empty() {
            self.print_str(format!(" {}", self.options.prefix).as_str())?;
        }
        self.print_str(": ")?;
        self.previous_time = Some(t);
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

    fn printer_showing_control_and_escape(stream: &'_ mut Vec<u8>) -> Printer<'_> {
        Printer::new(
            stream,
            Arc::new(Mutex::new(Timestamp::new())),
            Options {
                show_delta: false,
                prefix: String::new(),
                show_control: true,
                show_escape: true,
                dump_tokens: false,
                flush_all: false,
            },
        )
    }

    fn expect_get_timestamp(printer: &mut Printer, timestamp: Duration) {
        printer.timestamp.lock().unwrap().expect_get(timestamp);
    }

    fn assert_all_timestamps_used(printer: &Printer) {
        printer.timestamp.lock().unwrap().assert_all_used();
    }

    #[test]
    fn timestamp_is_added_at_beginning_of_lines() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A");
    }

    #[test]
    fn timestamp_is_requested_for_first_token_on_line() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::LineFeed).unwrap();

        // Timestamp is not request until first token on new line is received
        expect_get_timestamp(&mut printer, Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A\u{240a}\n", "00:04.000: B");
    }

    #[test]
    fn overwriting_line_with_cr_is_unfolded() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::CarriageReturn).unwrap();

        expect_get_timestamp(&mut printer, Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A\u{240d}\r\n", "00:04.000: B");
    }

    #[test]
    fn cr_lf_causes_only_one_newline_but_cr_is_forwarded() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::CarriageReturn).unwrap();
        printer.print(&Token::LineFeed).unwrap();

        expect_get_timestamp(&mut printer, Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A\u{240d}\u{240a}\r\n", "00:04.000: B");
    }

    #[test]
    fn multiples_new_lines_are_handled() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::CarriageReturn).unwrap();
        printer.print(&Token::LineFeed).unwrap();

        expect_get_timestamp(&mut printer, Duration::from_secs(4));
        printer.print(&Token::CarriageReturn).unwrap();
        printer.print(&Token::LineFeed).unwrap();

        expect_get_timestamp(&mut printer, Duration::from_secs(5));
        printer.print(&Token::LineFeed).unwrap();

        expect_get_timestamp(&mut printer, Duration::from_secs(6));
        printer.print(&Token::LineFeed).unwrap();

        assert_all_timestamps_used(&printer);
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
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::CarriageReturn).unwrap();
        printer
            .print(&esc_token!(
                escape::SequenceCommand::EraseFromCursorToEndOfLine,
                "\x1b[K"
            ))
            .unwrap();

        expect_get_timestamp(&mut printer, Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A\u{240d}\u{241b}[K\r\n", "00:04.000: B");
    }

    #[test]
    fn escape_erase_entire_line_is_unfolded() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer
            .print(&esc_token!(
                escape::SequenceCommand::EraseEntireLine,
                "\x1b[2K"
            ))
            .unwrap();

        expect_get_timestamp(&mut printer, Duration::from_secs(4));
        printer.print(&Token::Char('B')).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A\u{241b}[2K\n", "00:04.000: B");
    }

    #[test]
    fn escape_coloring_is_unchanged() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer
            .print(&esc_token!(escape::SequenceCommand::Unhandled, "\x1b[31m"))
            .unwrap();
        printer.print(&Token::Char('B')).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A\x1b[31mB");
    }

    #[test]
    fn end_of_file_with_newline_before() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::LineFeed).unwrap();
        expect_get_timestamp(&mut printer, Duration::from_secs(4));
        printer.print(&Token::EndOfFile).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A\u{240a}\n", "00:04.000: \u{2404}\n");
    }

    #[test]
    fn end_of_file_with_empty_line_before() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::LineFeed).unwrap();
        expect_get_timestamp(&mut printer, Duration::from_secs(4));
        printer.print(&Token::EndOfFile).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: \u{240a}\n", "00:04.000: \u{2404}\n");
    }

    #[test]
    fn end_of_file_without_newline_before() {
        let mut stream = Vec::<u8>::new();
        let mut printer = printer_showing_control_and_escape(&mut stream);

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        expect_get_timestamp(&mut printer, Duration::from_secs(4));
        printer.print(&Token::EndOfFile).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A\n", "00:04.000: \u{2404}\n");
    }

    #[test]
    fn disabling_showing_control_characters_hides_symbol_for_linefeed() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(
            &mut stream,
            Arc::new(Mutex::new(Timestamp::new())),
            Options {
                show_delta: false,
                prefix: String::new(),
                show_control: false,
                show_escape: true,
                dump_tokens: false,
                flush_all: false,
            },
        );

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::LineFeed).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A\n");
    }

    #[test]
    fn disabling_showing_escape_sequence_hides_handled_sequence() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(
            &mut stream,
            Arc::new(Mutex::new(Timestamp::new())),
            Options {
                show_delta: false,
                prefix: String::new(),
                show_control: true,
                show_escape: false,
                dump_tokens: false,
                flush_all: false,
            },
        );

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();
        printer
            .print(&esc_token!(
                escape::SequenceCommand::EraseEntireLine,
                "\x1b[2K"
            ))
            .unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000: A");
    }

    #[test]
    fn prefix_should_be_added_with_timestamp() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(
            &mut stream,
            Arc::new(Mutex::new(Timestamp::new())),
            Options {
                show_delta: false,
                prefix: "prefix".to_string(),
                show_control: false,
                show_escape: false,
                dump_tokens: false,
                flush_all: false,
            },
        );

        expect_get_timestamp(&mut printer, Duration::from_secs(3));
        printer.print(&Token::Char('A')).unwrap();

        assert_all_timestamps_used(&printer);
        assert_printed!(stream, "00:03.000 prefix: A");
    }

    #[test]
    fn delta_should_be_added_with_timestamp() {
        let mut stream = Vec::<u8>::new();
        let mut printer = Printer::new(
            &mut stream,
            Arc::new(Mutex::new(Timestamp::new())),
            Options {
                show_delta: true,
                prefix: "prefix".to_string(),
                show_control: false,
                show_escape: false,
                dump_tokens: false,
                flush_all: false,
            },
        );

        expect_get_timestamp(&mut printer, Duration::from_millis(3000));
        expect_get_timestamp(&mut printer, Duration::from_millis(3100));
        printer.print(&Token::Char('A')).unwrap();
        printer.print(&Token::LineFeed).unwrap();
        printer.print(&Token::Char('B')).unwrap();

        //assert_all_timestamps_used(&printer);
        assert_printed!(
            stream,
            "00:03.000             prefix: A\n",
            "00:03.100 (00:00.100) prefix: B"
        );
    }
}
