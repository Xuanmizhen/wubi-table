use super::ParseError;

const MAX_INDEX: usize = (25 << 15) + (25 << 10) + (25 << 5) + 25;
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct WubiCode2 {
    index: usize,
}

impl TryFrom<&[u8]> for WubiCode2 {
    type Error = ParseError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value.len() {
            0 => Err(ParseError::Empty),
            5.. => Err(ParseError::TooLongCode(value.into())),
            mut len => {
                let mut index = 0;
                for ch in value {
                    len -= 1;
                    match ch {
                        b'a'..=b'y' => {
                            let data = (ch - b'a' + 1) as usize;
                            let data = data << (5 * len);
                            index += data;
                        }
                        _ => return Err(ParseError::NotValidChar),
                    }
                }
                Ok(Self { index })
            }
        }
    }
}

impl TryFrom<&str> for WubiCode2 {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.as_bytes().try_into()
    }
}

impl From<WubiCode2> for Vec<u8> {
    fn from(value: WubiCode2) -> Self {
        // debug_assert!(value.index)
        todo!()
    }
}

pub struct FullCodeTable2 {
    code_to_phrases: [Vec<String>; MAX_INDEX + 1],
}

impl FullCodeTable2 {
    pub fn new() -> Self {
        Self {
            code_to_phrases: std::array::from_fn(|_| Default::default()),
        }
    }

    pub fn phrases(&mut self, code: &WubiCode2) -> &mut Vec<String> {
        &mut self.code_to_phrases[code.index]
    }

    pub fn add_phrase(&mut self, code: &WubiCode2, phrase: String) {
        let phrases = self.phrases(code);
        phrases.push(phrase);
    }
}

const CHAR_MIN: char = '\u{2eb3}';
const CHAR_MAX: char = '\u{9fff}';
const CHAR_COUNT: usize = CHAR_MAX as usize - CHAR_MIN as usize + 1;

pub struct SimplifiedCodeTable2 {
    code_to_char: [Option<char>; MAX_INDEX + 1],
    char_to_code: [Vec<WubiCode2>; CHAR_COUNT],
}

impl SimplifiedCodeTable2 {
    pub fn new() -> Self {
        let char_to_code = std::array::from_fn(|_| Vec::new());
        Self {
            code_to_char: [None; _],
            char_to_code,
        }
    }

    pub fn insert(&mut self, code: &WubiCode2, ch: char) -> Result<(), ParseError> {
        match ch {
            CHAR_MIN..=CHAR_MAX => {
                if self.code_to_char[code.index].is_some() {
                    return Err(ParseError::Invalid);
                }
                self.code_to_char[code.index] = Some(ch);
                let ch = ch as usize - CHAR_MIN as usize;
                let v = &mut self.char_to_code[ch];
                if v.capacity() == 0 {
                    v.reserve_exact(3);
                }
                v.push(*code);
                Ok(())
            }
            _ => {
                println!("{:x}", ch as usize);
                Err(ParseError::NotValidChar)
            }
        }
    }

    pub fn shrink_to_fit(&mut self) {
        for v in &mut self.char_to_code {
            v.shrink_to_fit();
        }
    }

    pub fn code_of_char(&self, ch: char) -> Option<&Vec<WubiCode2>> {
        if !(CHAR_MIN..=CHAR_MAX).contains(&ch) {
            return None;
        }
        Some(&self.char_to_code[ch as usize - CHAR_MIN as usize])
    }

    pub fn char_of_code(&self, code: &WubiCode2) -> Option<char> {
        self.code_to_char[code.index]
    }
}

impl Default for SimplifiedCodeTable2 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_wubi_code() {
        assert_eq!(
            WubiCode2::try_from(b"abc".as_slice()),
            Ok(WubiCode2 { index: 1091 })
        );
        assert_eq!(
            WubiCode2::try_from(b"abcd".as_slice()),
            Ok(WubiCode2 {
                index: (1 << 15) + (2 << 10) + (3 << 5) + 4
            })
        );
        assert!(WubiCode2::try_from(b"".as_slice()).is_err());
        assert!(WubiCode2::try_from(b"a\0cd".as_slice()).is_err());
        assert!(WubiCode2::try_from(b"abcde".as_slice()).is_err());
        assert_eq!(
            WubiCode2::try_from(b"yyyy".as_slice()),
            Ok(WubiCode2 { index: MAX_INDEX })
        );
    }
}
