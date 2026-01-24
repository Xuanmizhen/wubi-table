use super::ParseError;
use crate::WubiEntry;
use arrayvec::ArrayVec;
use std::{collections::HashMap, fmt};

const INDEX_UPPER_BOUND: usize = 26_u32.strict_pow(4) as usize;

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct WubiCode {
    index: u32,
}

impl TryFrom<&[u8]> for WubiCode {
    type Error = ParseError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value.len() {
            0 => Err(ParseError::Empty),
            5.. => Err(ParseError::TooLongCode(value.into())),
            _ => {
                let mut len = 4;
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

impl TryFrom<&str> for WubiCode {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.as_bytes().try_into()
    }
}

impl From<WubiCode> for Vec<u8> {
    fn from(mut value: WubiCode) -> Self {
        debug_assert!((value.index as usize) < INDEX_UPPER_BOUND);

        // TODO: DRY
        let byte1 = value.index / 26_u32.pow(3);
        value.index %= 26_u32.pow(3);
        match byte1 {
            0 => {
                eprintln!("{}", value.index);
                panic!();
            } // should not appear
            byte1 => {
                let byte1 = byte1 as u8 - 1 + b'a';
                let byte2 = value.index / 26_u32.pow(2);
                value.index %= 26_u32.pow(2);
                match byte2 {
                    0 => vec![byte1],
                    byte2 => {
                        let byte2 = byte2 as u8 - 1 + b'a';
                        let byte3 = value.index / 26_u32.pow(1);
                        value.index %= 26_u32.pow(1);
                        match byte3 {
                            0 => vec![byte1, byte2],
                            byte3 => {
                                let byte3 = (byte3 - 1) as u8 + b'a';
                                let byte4 = value.index;
                                match byte4 {
                                    0 => vec![byte1, byte2, byte3],
                                    byte4 => {
                                        let byte4 = (byte4 - 1) as u8 + b'a';
                                        vec![byte1, byte2, byte3, byte4]
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl fmt::Display for WubiCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = Vec::from(*self);
        f.write_str(std::str::from_utf8(&v).expect("`From` trait should be implemented properly"))
    }
}

pub struct FullCodeTable {
    code_to_phrases: Vec<Vec<String>>,
    phrase_to_code: HashMap<String, WubiCode>,
}

impl FullCodeTable {
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

    pub fn phrases_mut(&mut self, code: &WubiCode) -> &mut Vec<String> {
        &mut self.code_to_phrases[code.index as usize]
    }

    pub fn code_mut(&mut self, phrase: &String) -> Option<&mut WubiCode> {
        self.phrase_to_code.get_mut(phrase)
    }

    pub fn code(&self, phrase: &String) -> Option<&WubiCode> {
        self.phrase_to_code.get(phrase)
    }

    pub fn insert(&mut self, entry: WubiEntry) {
        if let Some(code_mut) = self.code_mut(&entry.phrase) {
            // code_mut.push(entry.wubi_code);
            println!("{}", entry.phrase);
            println!("{}", code_mut);
            panic!();
        } else {
            self.phrase_to_code
                .insert(entry.phrase.clone(), entry.wubi_code);
        }
        self.phrases_mut(&entry.wubi_code).push(entry.phrase);
    }
}

impl Default for FullCodeTable {
    fn default() -> Self {
        Self::new()
    }
}

const CHAR_MIN: char = '\u{2eb3}';
const CHAR_MAX: char = '\u{9fff}';
const CHAR_COUNT: usize = ((CHAR_MAX as u16) - CHAR_MIN as u16 + 1) as usize;

pub struct SimplifiedCodeTable {
    code_to_char: Box<ArrayVec<Option<char>, INDEX_UPPER_BOUND>>,
    char_to_code: Box<ArrayVec<ArrayVec<WubiCode, 3>, CHAR_COUNT>>,
}

impl SimplifiedCodeTable {
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

    fn exists(&self, ch: char, code: &WubiCode) -> bool {
        if let Some(original) = self.char_of_code(code) {
            return ch == *original;
        }
        false
    }

    pub fn insert(&mut self, code: &WubiCode, ch: char) -> Result<(), ParseError> {
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

    pub fn code_of_char(&self, ch: char) -> Option<&ArrayVec<WubiCode, 3>> {
        if !(CHAR_MIN..=CHAR_MAX).contains(&ch) {
            return None;
        }
        Some(&self.char_to_code[ch as usize - CHAR_MIN as usize])
    }

    pub fn code_of_char_mut(&mut self, ch: char) -> Option<&mut ArrayVec<WubiCode, 3>> {
        if !(CHAR_MIN..=CHAR_MAX).contains(&ch) {
            return None;
        }
        Some(&mut self.char_to_code[ch as usize - CHAR_MIN as usize])
    }

    pub fn char_of_code(&self, code: &WubiCode) -> &Option<char> {
        &self.code_to_char[code.index as usize]
    }

    pub fn char_of_code_mut(&mut self, code: &WubiCode) -> &mut Option<char> {
        &mut self.code_to_char[code.index as usize]
    }
}

impl Default for SimplifiedCodeTable {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Table {
    simplified: SimplifiedCodeTable,
    full: FullCodeTable,
}

impl Table {
    pub fn new(simplified: SimplifiedCodeTable, full: FullCodeTable) -> Self {
        Self { simplified, full }
    }

    pub fn reverse_simplified_table(&self) -> impl Iterator<Item = (WubiCode, String)> {
        self.simplified
            .code_to_char
            .iter()
            .enumerate()
            .filter_map(|(index, ch)| {
                ch.map(|ch| {
                    let index = index as u32;
                    (WubiCode { index }, ch.to_string())
                })
            })
    }
    fn reverse_full_table(&self) -> impl Iterator<Item = (WubiCode, &Vec<String>)> {
        self.full
            .code_to_phrases
            .iter()
            .enumerate()
            .map(|(index, phrases)| {
                let index = index as u32;
                (WubiCode { index }, phrases)
            })
    }

    pub fn reverse_filtered_full_table(
        &self,
    ) -> impl Iterator<Item = (WubiCode, impl Iterator<Item = &String>)> {
        self.reverse_full_table().filter_map(|(code, phrases)| {
            let phrases: Vec<_> = if let Some(simplified_ch) = self.simplified.char_of_code(&code) {
                let phrases = phrases
                    .iter()
                    .skip_while(|phrase| **phrase != simplified_ch.to_string());
                phrases.collect()
            } else {
                phrases.iter().collect()
            };
            if phrases.is_empty() {
                None
            } else {
                Some((code, phrases.into_iter()))
            }
        })
    }

    pub fn simplified_table(&self) -> impl Iterator<Item = (char, impl Iterator<Item = WubiCode>)> {
        self.simplified
            .char_to_code
            .iter()
            .enumerate()
            .filter_map(|(index, codes)| {
                if codes.is_empty() {
                    return None;
                }
                let ch = char::from_u32(CHAR_MIN as u32 + index as u32).unwrap();
                let codes = codes.iter().map(|code| {
                    let index = code.index;
                    WubiCode { index }
                });
                Some((ch, codes))
            })
    }

    fn full_table(&self) -> impl Iterator<Item = (&String, &WubiCode)> {
        self.full.phrase_to_code.iter()
    }

    pub fn filtered_full_table(&self) -> impl Iterator<Item = (&String, &WubiCode)> {
        self.full_table().filter_map(|(phrase, code)| {
            let mut chars = phrase.chars();
            if let Some(ch) = chars.next()
                && chars.next().is_none()
                && let Some(simplified_codes) = self.simplified.code_of_char(ch)
                && let Some(longest_simplified_code) = simplified_codes.last()
                && longest_simplified_code == code
            {
                None
            } else {
                Some((phrase, code))
            }
        })
    }
}

pub fn get_code_for_phrase(phrase: &str, char_code: impl Fn(char) -> WubiCode) -> WubiCode {
    let mut chars = phrase.chars();
    match chars.clone().count() {
        0 | 1 => panic!(),
        2 => {
            let first = char_code(chars.next().unwrap());
            let second = char_code(chars.next().unwrap());
            let index = first.index / 26_u32.pow(2) * 26_u32.pow(2) + second.index / 26_u32.pow(2);
            WubiCode { index }
        }
        3 => {
            let first = char_code(chars.next().unwrap());
            let second = char_code(chars.next().unwrap());
            let third = char_code(chars.next().unwrap());
            let index = first.index / 26_u32.pow(3) * 26_u32.pow(3)
                + second.index / 26_u32.pow(3) * 26_u32.pow(2)
                + third.index / 26_u32.pow(2);
            WubiCode { index }
        }
        4.. => {
            let first = char_code(chars.next().unwrap());
            let second = char_code(chars.next().unwrap());
            let third = char_code(chars.next().unwrap());
            let last = char_code(chars.last().unwrap());
            let index = first.index / 26_u32.pow(3) * 26_u32.pow(3)
                + second.index / 26_u32.pow(3) * 26_u32.pow(2)
                + third.index / 26_u32.pow(3) * 26_u32.pow(1)
                + last.index / 26_u32.pow(3);
            WubiCode { index }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_wubi_code() {
        assert!(WubiCode::try_from(b"".as_slice()).is_err());
        assert!(WubiCode::try_from(b"a\0cd".as_slice()).is_err());
        assert!(WubiCode::try_from(b"abcde".as_slice()).is_err());
        assert_eq!(
            WubiCode::try_from(b"yyyy".as_slice()),
            Ok(WubiCode {
                index: INDEX_UPPER_BOUND as u32 - 1
            })
        );
    }
}
