use std::io::Read;

/// Reads a character, if EOF is reached None is returned
pub fn read_char(stream: &mut impl Read) -> Result<Option<char>, std::io::Error> {
    let mut bytes: [u8; 4] = [0, 0, 0, 0];
    for i in 0..4 {
        let x = &mut bytes[i..(i + 1)];
        let read_length = stream.read(x)?;

        if read_length != 1 {
            return if i == 0 {
                Ok(None)
            } else {
                Err(std::io::Error::other(
                    "EOF reached in partial UTF-8 character",
                ))
            };
        }

        if let Some(chunk) = bytes.utf8_chunks().next() {
            let valid_chunk = chunk.valid();
            if !valid_chunk.is_empty() {
                return Ok(Some(valid_chunk.chars().next().unwrap()));
            }
        }
    }

    Err(std::io::Error::other("Invalid UTF-8 character"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_char_can_read_ascii_and_detect_eof() {
        let mut chars = "text".as_bytes();
        let stream = &mut chars;
        assert_eq!(Some('t'), read_char(stream).unwrap());
        assert_eq!(Some('e'), read_char(stream).unwrap());
        assert_eq!(Some('x'), read_char(stream).unwrap());
        assert_eq!(Some('t'), read_char(stream).unwrap());
        assert_eq!(None, read_char(stream).unwrap());
    }

    #[test]
    fn read_char_can_read_utf8() {
        let sparkle_heart: [u8; 4] = [240, 159, 146, 150];
        let stream = &mut &sparkle_heart[..];
        assert_eq!(Some('💖'), read_char(stream).unwrap());
    }

    #[test]
    fn read_char_fails_when_eof_in_middle_of_utf8_char() {
        let half_utf8: [u8; 2] = [240, 159];
        let stream = &mut &half_utf8[..];
        assert_eq!(
            "EOF reached in partial UTF-8 character",
            read_char(stream).unwrap_err().to_string()
        );
    }

    #[test]
    fn read_char_fails_when_invalid_utf_char() {
        let half_utf8: [u8; 4] = [255, 255, 255, 255];
        let stream = &mut &half_utf8[..];
        assert_eq!(
            "Invalid UTF-8 character",
            read_char(stream).unwrap_err().to_string()
        );
    }
}
