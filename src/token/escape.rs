use regex::Regex;
use std::sync::LazyLock;

/// Representation of various ANSI escape sequences, in particular sequences for moving and
/// erasing. Sequences are strings starting with the esc character to control console behavior.
#[derive(Debug, PartialEq)]
pub struct Sequence {
    pub command: SequenceCommand,
    pub text: String,
}

/// The command an escape sequences represents, see
/// https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797 for reference
#[derive(Debug, PartialEq)]
pub enum SequenceCommand {
    /// ESC[H
    CursorMoveHome,
    /// ESC[#;#H or ESC[#;#f
    CursorMoveToLineAndColumn((u32, u32)),
    /// ESC[#A
    CursorMoveLinesUp(u32),
    /// ESC[#B
    CursorMoveLinesDown(u32),
    /// ESC[#C
    CursorMoveColumnsRight(u32),
    /// ESC[#D
    CursorMoveColumnsLeft(u32),
    /// ESC[#E
    CursorMoveBeginningLinesDown(u32),
    /// ESC[#F
    CursorMoveBeginningLinesUp(u32),
    /// ESC[#G
    CursorMoveToColumn(u32),
    /// ESC[6n
    CursorRequestPosition,
    /// ESC M
    CursorMoveUpOne,
    /// ESC 7 or ESC[s
    CursorSavePosition,
    /// ESC 8 or ESC[u
    CursorRestorePosition,
    /// ESC[J or ESC[0J
    EraseFromCursorToEndOfScreen,
    /// ESC[1J
    EraseFromBeginningOfScreenToCursor,
    /// ESC[2J
    EraseEntireScreen,
    /// ESC[3J
    EraseSavedLines,
    /// ESC[K or ESC[0K
    EraseFromCursorToEndOfLine,
    /// ESC[1K
    EraseFromStartOfLineToCursor,
    /// ESC[2K
    EraseEntireLine,
    /// Escape sequences unrelated to moving or erasing
    Unhandled,
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

        if let Some(cap1) = captures.get(1) {
            assert_eq!(None, captures.get(2));
            assert_eq!(1, cap1.len());
            Self::without_bracket(cap1.as_str().chars().nth(0).unwrap())
        } else {
            let numbers = if let Some(numbers) = captures.get(2) {
                numbers
                    .as_str()
                    .split(";")
                    .map(|s| s.parse::<u32>().unwrap())
                    .collect::<Vec<u32>>()
            } else {
                vec![]
            };
            let cap3 = captures.get(3).expect("Regex should find end character");
            assert_eq!(1, cap3.len());
            let c = cap3.as_str().chars().nth(0).unwrap();
            Self::with_bracket(numbers, c)
        }
    }

    // Sequence like "ESC M" (without '[')
    fn without_bracket(c: char) -> Option<Self> {
        Some(match c {
            'M' => Self::CursorMoveUpOne,
            '7' => Self::CursorSavePosition,
            '8' => Self::CursorRestorePosition,
            _ => Self::Unhandled,
        })
    }

    // Sequence with '[', like "ESC[17;42f"
    fn with_bracket(numbers: Vec<u32>, c: char) -> Option<Self> {
        Some(match numbers.len() {
            0 => match c {
                'H' => Self::CursorMoveHome,
                'J' => Self::EraseFromCursorToEndOfScreen,
                'K' => Self::EraseFromCursorToEndOfLine,
                's' => Self::CursorSavePosition,
                'u' => Self::CursorRestorePosition,
                _ => Self::Unhandled,
            },
            1 => {
                let number = numbers[0];
                match c {
                    'A' => Self::CursorMoveLinesUp(number),
                    'B' => Self::CursorMoveLinesDown(number),
                    'C' => Self::CursorMoveColumnsRight(number),
                    'D' => Self::CursorMoveColumnsLeft(number),
                    'E' => Self::CursorMoveBeginningLinesUp(number),
                    'F' => Self::CursorMoveBeginningLinesDown(number),
                    'G' => Self::CursorMoveToColumn(number),
                    'J' => match number {
                        0 => Self::EraseFromCursorToEndOfScreen,
                        1 => Self::EraseFromBeginningOfScreenToCursor,
                        2 => Self::EraseEntireScreen,
                        3 => Self::EraseSavedLines,
                        _ => Self::Unhandled,
                    },
                    'K' => match number {
                        0 => Self::EraseFromCursorToEndOfLine,
                        1 => Self::EraseFromStartOfLineToCursor,
                        2 => Self::EraseEntireLine,
                        _ => Self::Unhandled,
                    },
                    'n' => {
                        if number == 6 {
                            Self::CursorRequestPosition
                        } else {
                            Self::Unhandled
                        }
                    }
                    _ => Self::Unhandled,
                }
            }
            2 => match c {
                'f' | 'H' => Self::CursorMoveToLineAndColumn((numbers[0], numbers[1])),
                _ => Self::Unhandled,
            },
            _ => Self::Unhandled,
        })
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
        assert_esc!(SequenceCommand::CursorMoveUpOne, esc!("M"));
        assert_esc!(SequenceCommand::CursorSavePosition, esc!("7"));
        assert_esc!(SequenceCommand::CursorRestorePosition, esc!("8"));
        assert_esc!(SequenceCommand::CursorMoveHome, esc!("[H"));
        assert_esc!(SequenceCommand::CursorSavePosition, esc!("[s"));
        assert_esc!(SequenceCommand::CursorRestorePosition, esc!("[u"));

        assert_esc!(SequenceCommand::CursorMoveLinesUp(17), esc!("[17A"));
        assert_esc!(SequenceCommand::CursorMoveLinesDown(18), esc!("[18B"));
        assert_esc!(SequenceCommand::CursorMoveColumnsRight(19), esc!("[19C"));
        assert_esc!(SequenceCommand::CursorMoveColumnsLeft(20), esc!("[20D"));
        assert_esc!(
            SequenceCommand::CursorMoveBeginningLinesUp(21),
            esc!("[21E")
        );
        assert_esc!(
            SequenceCommand::CursorMoveBeginningLinesDown(22),
            esc!("[22F")
        );
        assert_esc!(SequenceCommand::CursorMoveToColumn(23), esc!("[23G"));
        assert_esc!(SequenceCommand::CursorRequestPosition, esc!("[6n"));

        assert_esc!(SequenceCommand::EraseFromCursorToEndOfScreen, esc!("[J"));
        assert_esc!(SequenceCommand::EraseFromCursorToEndOfScreen, esc!("[0J"));
        assert_esc!(
            SequenceCommand::EraseFromBeginningOfScreenToCursor,
            esc!("[1J")
        );
        assert_esc!(SequenceCommand::EraseEntireScreen, esc!("[2J"));
        assert_esc!(SequenceCommand::EraseSavedLines, esc!("[3J"));
        assert_esc!(SequenceCommand::EraseFromCursorToEndOfLine, esc!("[K"));
        assert_esc!(SequenceCommand::EraseFromCursorToEndOfLine, esc!("[0K"));
        assert_esc!(SequenceCommand::EraseFromStartOfLineToCursor, esc!("[1K"));
        assert_esc!(SequenceCommand::EraseEntireLine, esc!("[2K"));

        assert_esc!(
            SequenceCommand::CursorMoveToLineAndColumn((17, 42)),
            esc!("[17;42H")
        );
        assert_esc!(
            SequenceCommand::CursorMoveToLineAndColumn((17, 42)),
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
