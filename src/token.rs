use std::io::Read;
use crate::read_char::read_char;


#[derive(PartialEq)]
pub enum EscapeCursorMove {
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
pub enum EscapeErase {
    FromCursorToEndOfScreen,
    FromCursorToBeginningOfScreen,
    EntireScreen,
    SavedLine,
    FromCursorToEndOfLine,
    StartOfLineToCursor,
    EntireLine,
}

#[derive(PartialEq)]
pub enum Token {
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
            Self::Char(c) => write!(f, "{c}"),
            Self::CarriageReturn => write!(f, "\u{240d}"),
            Self::LineFeed => {
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


pub struct SerialTokenizer<'a, R: Read>
{
    stream: &'a mut R,
    /// A buffer for handling
    _escape_buf: String,
}

impl<'a, R: Read> SerialTokenizer<'a, R>
{
    pub fn new(stream: &'a mut R) -> Self {
        Self { stream, _escape_buf: String::new() }
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