use regex::Regex;
use std::sync::LazyLock;

/// Representation of various ANSI escape sequences, in particular sequences for moving and
/// erasing. Sequences are strings starting with the esc character to control console behavior.
#[derive(Clone, Debug, PartialEq)]
pub struct Sequence {
    pub command: SequenceCommand,
    pub text: String,
}

/// The command an escape sequences represents, see
/// <https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797> for reference
#[derive(Clone, Debug, PartialEq)]
pub enum SequenceCommand {
    CursorMove(CursorMove),
    CursorPosition(CursorPosition),
    Erase(Erase),
    Unhandled,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CursorMove {
    /// ESC[H
    Home,
    /// ESC[#;#H or ESC[#;#f
    ToLineAndColumn((u32, u32)),
    /// ESC[#A
    LinesUp(u32),
    /// ESC[#B
    LinesDown(u32),
    /// ESC[#C
    ColumnsRight(u32),
    /// ESC[#D
    ColumnsLeft(u32),
    /// ESC[#E
    BeginningLinesDown(u32),
    /// ESC[#F
    BeginningLinesUp(u32),
    /// ESC[#G
    ToColumn(u32),
    /// ESC M
    UpOne,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CursorPosition {
    /// ESC[6n
    Request,
    /// ESC 7 or ESC[s
    Save,
    /// ESC 8 or ESC[u
    Restore,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Erase {
    /// ESC[J or ESC[0J
    FromCursorToEndOfScreen,
    /// ESC[1J
    FromBeginningOfScreenToCursor,
    /// ESC[2J
    EntireScreen,
    /// ESC[3J
    SavedLines,
    /// ESC[K or ESC[0K
    FromCursorToEndOfLine,
    /// ESC[1K
    FromStartOfLineToCursor,
    /// ESC[2K
    EntireLine,
}

/// The escape character
pub const ESC: char = '\x1b';

/// Regex to catch escape sequences
static SEQUENCE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        format!(
            r"(?x)
              ^                         # Must match from start of string
              {ESC}                     # Escape character
              (?:        
                ([A-Za-z0-9])           # If sequence without '[', capture a single character
                |
                \[                      # If sequence with '[':
                ([0-9]+(?:;[0-9]+)*)?   # Capture numbers separated by ';'
                ([A-Za-z])              # And a single following character
              )
              $                         # Must match to end of string"
        )
        .as_str(),
    )
    .unwrap()
});

impl Sequence {
    /// Creates an escape sequence struct from a string
    pub fn from(buffer: &str) -> Option<Self> {
        SequenceCommand::from(buffer).map(|command| Self {
            command,
            text: buffer.to_string(),
        })
    }
}

impl SequenceCommand {
    fn from(buffer: &str) -> Option<Self> {
        let captures = (*SEQUENCE_REGEX).captures(buffer)?;
        assert_eq!(4, captures.len());

        Some(if let Some(cap1) = captures.get(1) {
            assert_eq!(None, captures.get(2));
            assert_eq!(1, cap1.len());
            Self::without_bracket(cap1.as_str().chars().nth(0).unwrap())
        } else {
            let numbers = if let Some(numbers) = captures.get(2) {
                numbers
                    .as_str()
                    .split(';')
                    .map(|s| s.parse::<u32>().unwrap())
                    .collect::<Vec<u32>>()
            } else {
                vec![]
            };
            let cap3 = captures.get(3).expect("Regex should find end character");
            assert_eq!(1, cap3.len());
            let c = cap3.as_str().chars().nth(0).unwrap();
            Self::with_bracket(&numbers, c)
        })
    }

    // Sequence like "ESC M" (without '[')
    fn without_bracket(c: char) -> Self {
        match c {
            'M' => Self::CursorMove(CursorMove::UpOne),
            '7' => Self::CursorPosition(CursorPosition::Save),
            '8' => Self::CursorPosition(CursorPosition::Restore),
            _ => Self::Unhandled,
        }
    }

    // Sequence with '[', like "ESC[17;42f"
    fn with_bracket(numbers: &[u32], c: char) -> Self {
        match numbers.len() {
            0 => match c {
                'H' => Self::CursorMove(CursorMove::Home),
                'J' => Self::Erase(Erase::FromCursorToEndOfScreen),
                'K' => Self::Erase(Erase::FromCursorToEndOfLine),
                's' => Self::CursorPosition(CursorPosition::Save),
                'u' => Self::CursorPosition(CursorPosition::Restore),
                _ => Self::Unhandled,
            },
            1 => {
                let number = numbers[0];
                match c {
                    'A' => Self::CursorMove(CursorMove::LinesUp(number)),
                    'B' => Self::CursorMove(CursorMove::LinesDown(number)),
                    'C' => Self::CursorMove(CursorMove::ColumnsRight(number)),
                    'D' => Self::CursorMove(CursorMove::ColumnsLeft(number)),
                    'E' => Self::CursorMove(CursorMove::BeginningLinesUp(number)),
                    'F' => Self::CursorMove(CursorMove::BeginningLinesDown(number)),
                    'G' => Self::CursorMove(CursorMove::ToColumn(number)),
                    'J' => match number {
                        0 => Self::Erase(Erase::FromCursorToEndOfScreen),
                        1 => Self::Erase(Erase::FromBeginningOfScreenToCursor),
                        2 => Self::Erase(Erase::EntireScreen),
                        3 => Self::Erase(Erase::SavedLines),
                        _ => Self::Unhandled,
                    },
                    'K' => match number {
                        0 => Self::Erase(Erase::FromCursorToEndOfLine),
                        1 => Self::Erase(Erase::FromStartOfLineToCursor),
                        2 => Self::Erase(Erase::EntireLine),
                        _ => Self::Unhandled,
                    },
                    'n' => {
                        if number == 6 {
                            Self::CursorPosition(CursorPosition::Request)
                        } else {
                            Self::Unhandled
                        }
                    }
                    _ => Self::Unhandled,
                }
            }
            2 => match c {
                'f' | 'H' => {
                    Self::CursorMove(CursorMove::ToLineAndColumn((numbers[0], numbers[1])))
                }
                _ => Self::Unhandled,
            },
            _ => Self::Unhandled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Creates a string slice just containing an ANSI escape sequence
    macro_rules! esc {
        ($chars:expr) => {
            format!("{}{}", ESC, $chars).as_str()
        };
    }

    macro_rules! assert_esc {
        ($command:expr, $text:expr) => {
            assert_eq!(
                Sequence {
                    command: $command,
                    text: $text.to_string()
                },
                Sequence::from($text).unwrap()
            )
        };
    }

    macro_rules! assert_incomplete_esc {
        ($text:expr) => {
            assert_eq!(None, Sequence::from($text))
        };
    }

    #[test]
    fn match_escape_returns_none_for_incomplete_escape_sequences() {
        assert_incomplete_esc!(esc!(""));
        assert_incomplete_esc!(esc!("["));
        assert_incomplete_esc!(esc!("[1"));
        assert_incomplete_esc!(esc!("[12"));
        assert_incomplete_esc!(esc!("[12;1"));
        assert_incomplete_esc!(esc!("[12;13"));
    }

    #[test]
    fn match_escape_returns_correct_escape_sequences() {
        assert_esc!(SequenceCommand::CursorMove(CursorMove::UpOne), esc!("M"));
        assert_esc!(
            SequenceCommand::CursorPosition(CursorPosition::Save),
            esc!("7")
        );
        assert_esc!(
            SequenceCommand::CursorPosition(CursorPosition::Restore),
            esc!("8")
        );
        assert_esc!(SequenceCommand::CursorMove(CursorMove::Home), esc!("[H"));
        assert_esc!(
            SequenceCommand::CursorPosition(CursorPosition::Save),
            esc!("[s")
        );
        assert_esc!(
            SequenceCommand::CursorPosition(CursorPosition::Restore),
            esc!("[u")
        );

        assert_esc!(
            SequenceCommand::CursorMove(CursorMove::LinesUp(17)),
            esc!("[17A")
        );
        assert_esc!(
            SequenceCommand::CursorMove(CursorMove::LinesDown(18)),
            esc!("[18B")
        );
        assert_esc!(
            SequenceCommand::CursorMove(CursorMove::ColumnsRight(19)),
            esc!("[19C")
        );
        assert_esc!(
            SequenceCommand::CursorMove(CursorMove::ColumnsLeft(20)),
            esc!("[20D")
        );
        assert_esc!(
            SequenceCommand::CursorMove(CursorMove::BeginningLinesUp(21)),
            esc!("[21E")
        );
        assert_esc!(
            SequenceCommand::CursorMove(CursorMove::BeginningLinesDown(22)),
            esc!("[22F")
        );
        assert_esc!(
            SequenceCommand::CursorMove(CursorMove::ToColumn(23)),
            esc!("[23G")
        );
        assert_esc!(
            SequenceCommand::CursorPosition(CursorPosition::Request),
            esc!("[6n")
        );

        assert_esc!(
            SequenceCommand::Erase(Erase::FromCursorToEndOfScreen),
            esc!("[J")
        );
        assert_esc!(
            SequenceCommand::Erase(Erase::FromCursorToEndOfScreen),
            esc!("[0J")
        );
        assert_esc!(
            SequenceCommand::Erase(Erase::FromBeginningOfScreenToCursor),
            esc!("[1J")
        );
        assert_esc!(SequenceCommand::Erase(Erase::EntireScreen), esc!("[2J"));
        assert_esc!(SequenceCommand::Erase(Erase::SavedLines), esc!("[3J"));
        assert_esc!(
            SequenceCommand::Erase(Erase::FromCursorToEndOfLine),
            esc!("[K")
        );
        assert_esc!(
            SequenceCommand::Erase(Erase::FromCursorToEndOfLine),
            esc!("[0K")
        );
        assert_esc!(
            SequenceCommand::Erase(Erase::FromStartOfLineToCursor),
            esc!("[1K")
        );
        assert_esc!(SequenceCommand::Erase(Erase::EntireLine), esc!("[2K"));

        assert_esc!(
            SequenceCommand::CursorMove(CursorMove::ToLineAndColumn((17, 42))),
            esc!("[17;42H")
        );
        assert_esc!(
            SequenceCommand::CursorMove(CursorMove::ToLineAndColumn((17, 42))),
            esc!("[17;42f")
        );
    }

    #[test]
    fn match_escape_returns_unhandled_for_other_escape_sequences() {
        assert_esc!(SequenceCommand::Unhandled, esc!("9"));
        assert_esc!(SequenceCommand::Unhandled, esc!("Q"));
        assert_esc!(SequenceCommand::Unhandled, esc!("[Q"));
        assert_esc!(SequenceCommand::Unhandled, esc!("[17Q"));
        assert_esc!(SequenceCommand::Unhandled, esc!("[17;18Q"));
    }
}
