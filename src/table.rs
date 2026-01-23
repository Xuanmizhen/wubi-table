use super::ParseError;
use crate::WubiEntry2;
use arrayvec::ArrayVec;
use std::{collections::HashMap, fmt};

const INDEX_UPPER_BOUND: usize = 26_u32.strict_pow(4) as usize;

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct WubiCode2 {
    index: u32,
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
                            let data = (ch - b'a' + 1) as u32;
                            let data = data * 26_u32.pow(len as u32);
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

impl From<&WubiCode2> for Vec<u8> {
    fn from(value: &WubiCode2) -> Self {
        debug_assert!((value.index as usize) < INDEX_UPPER_BOUND);
        todo!()
    }
}

impl fmt::Display for WubiCode2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = Vec::from(self);
        f.write_str(std::str::from_utf8(&v).expect("`From` trait should be implemented properly"))
    }
}

pub struct FullCodeTable2 {
    code_to_phrases: Vec<Vec<String>>,
    phrase_to_code: HashMap<String, Vec<WubiCode2>>,
}

impl FullCodeTable2 {
    pub fn new() -> Self {
        let mut code_to_phrases = Vec::with_capacity(INDEX_UPPER_BOUND);
        for _ in 0..INDEX_UPPER_BOUND {
            code_to_phrases.push(Vec::new());
        }
        let phrase_to_code = HashMap::new();
        Self {
            code_to_phrases,
            phrase_to_code,
        }
    }

    pub fn phrases_mut(&mut self, code: &WubiCode2) -> &mut Vec<String> {
        &mut self.code_to_phrases[code.index as usize]
    }

    pub fn code_mut(&mut self, phrase: &String) -> Option<&mut Vec<WubiCode2>> {
        self.phrase_to_code.get_mut(phrase)
    }

    pub fn insert(&mut self, entry: WubiEntry2) {
        if let Some(code_mut) = self.code_mut(&entry.phrase) {
            code_mut.push(entry.wubi_code);
        } else {
            self.phrase_to_code
                .insert(entry.phrase.clone(), vec![entry.wubi_code]);
        }
        self.phrases_mut(&entry.wubi_code).push(entry.phrase);
    }
}

const CHAR_MIN: char = '\u{2eb3}';
const CHAR_MAX: char = '\u{9fff}';
const CHAR_COUNT: usize = ((CHAR_MAX as u16) - CHAR_MIN as u16 + 1) as usize;

pub struct SimplifiedCodeTable2 {
    code_to_char: Box<ArrayVec<Option<char>, INDEX_UPPER_BOUND>>,
    char_to_code: Box<ArrayVec<ArrayVec<WubiCode2, 3>, CHAR_COUNT>>,
}

impl SimplifiedCodeTable2 {
    pub fn new() -> Self {
        let mut code_to_char = Box::new(ArrayVec::new());
        let mut char_to_code = Box::new(ArrayVec::new());
        code_to_char.extend(std::iter::repeat_n(None, INDEX_UPPER_BOUND));
        char_to_code.extend(std::iter::repeat_n(ArrayVec::new(), CHAR_COUNT));
        Self {
            code_to_char,
            char_to_code,
        }
    }

    pub fn exists(&self, ch: char, code: &WubiCode2) -> bool {
        if let Some(original) = self.char_of_code(code) {
            return ch == *original;
        }
        false
    }

    pub fn insert(&mut self, code: &WubiCode2, ch: char) -> Result<(), ParseError> {
        let char_ref = self.char_of_code_mut(code);
        if char_ref.is_some() {
            return Err(ParseError::Invalid);
        }
        *char_ref = Some(ch);
        if let Some(code_ref) = self.code_of_char_mut(ch) {
            if code_ref.len() == 3 {
                println!("{ch} {:?} {:?}", code_ref, code);
            }
            code_ref.push(*code);
            Ok(())
        } else {
            eprintln!("{:x}", ch as usize);
            Err(ParseError::NotValidChar)
        }
    }

    pub fn code_of_char(&self, ch: char) -> Option<&ArrayVec<WubiCode2, 3>> {
        if !(CHAR_MIN..=CHAR_MAX).contains(&ch) {
            return None;
        }
        Some(&self.char_to_code[ch as usize - CHAR_MIN as usize])
    }

    pub fn code_of_char_mut(&mut self, ch: char) -> Option<&mut ArrayVec<WubiCode2, 3>> {
        if !(CHAR_MIN..=CHAR_MAX).contains(&ch) {
            return None;
        }
        Some(&mut self.char_to_code[ch as usize - CHAR_MIN as usize])
    }

    pub fn char_of_code(&self, code: &WubiCode2) -> &Option<char> {
        &self.code_to_char[code.index as usize]
    }

    pub fn char_of_code_mut(&mut self, code: &WubiCode2) -> &mut Option<char> {
        &mut self.code_to_char[code.index as usize]
    }
}

impl Default for SimplifiedCodeTable2 {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Table {
    simplified: SimplifiedCodeTable2,
    full: FullCodeTable2,
}

impl Table {
    pub fn new(simplified: SimplifiedCodeTable2, full: FullCodeTable2) -> Self {
        Self { simplified, full }
    }

    pub fn reverse_simplified_table(&self) -> impl Iterator<Item = (WubiCode2, char)> {
        self.simplified
            .code_to_char
            .iter()
            .enumerate()
            .filter_map(|(index, ch)| {
                ch.map(|ch| {
                    let index = index as u32;
                    (WubiCode2 { index }, ch)
                })
            })
    }
    fn reverse_full_table(
        &self,
    ) -> impl Iterator<Item = (WubiCode2, impl Iterator<Item = &'_ String>)> {
        self.full
            .code_to_phrases
            .iter()
            .enumerate()
            .map(|(index, phrases)| {
                let index = index as u32;
                (WubiCode2 { index }, phrases.iter())
            })
    }
    pub fn reverse_filtered_full_table(
        &self,
    ) -> impl Iterator<Item = (WubiCode2, Box<dyn Iterator<Item = &'_ String> + '_>)> {
        self.reverse_full_table().map(|(code, phrases)| {
            if let Some(simplified_ch) = self.simplified.char_of_code(&code) {
                let phrases = phrases.filter(|phrase| {
                    let mut chars = phrase.chars();
                    chars.next() == Some(*simplified_ch) && chars.next().is_none()
                });
                (code, Box::new(phrases) as Box<dyn Iterator<Item = &String>>)
            } else {
                (code, Box::new(phrases) as Box<dyn Iterator<Item = &String>>)
            }
        })
    }

    pub fn reverse_table(&self) -> impl Iterator<Item = (&'_ String, &'_ Vec<WubiCode2>)> {
        self.full.phrase_to_code.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_wubi_code() {
        assert!(WubiCode2::try_from(b"".as_slice()).is_err());
        assert!(WubiCode2::try_from(b"a\0cd".as_slice()).is_err());
        assert!(WubiCode2::try_from(b"abcde".as_slice()).is_err());
        assert_eq!(
            WubiCode2::try_from(b"yyyy".as_slice()),
            Ok(WubiCode2 {
                index: INDEX_UPPER_BOUND as u32 - 1
            })
        );
    }
}
