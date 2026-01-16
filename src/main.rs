#![forbid(unsafe_code)]

use trie_rs;
use std::fs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseWubiTableError {
    #[error("Invalid format")]
    Invalid,
    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Codepoint does not match character")]
    CodepointMismatch,
}

pub struct WubiTableOriginal<'a> {
    entries: Vec<WubiEntry<'a>>,
}

struct WubiEntry<'a> {
    character: char,
    wubi_code: &'a str,
}

impl<'a> WubiTableOriginal<'a> {
    fn from_str(s: &'a str) -> Result<Self, ParseWubiTableError> {
        let mut entries = Vec::new();
        for line in s.lines() {
            let (codepoint, rest) = line.split_once('\t').ok_or(ParseWubiTableError::Invalid)?;
            let (ch, wubi) = rest.split_once('\t').ok_or(ParseWubiTableError::Invalid)?;
            let ch = {
                if ch.chars().count() != 1 {
                    return Err(ParseWubiTableError::Invalid);
                }
                ch.chars().next().expect("Checked above")
            };
            if !(codepoint.starts_with("U+")
                && u32::from_str_radix(&codepoint[2..], 16)? == ch as u32)
            {
                return Err(ParseWubiTableError::CodepointMismatch);
            }
            entries.push(WubiEntry {
                character: ch,
                wubi_code: wubi,
            });
        }
        Ok(WubiTableOriginal { entries })
    }
}

fn main() {
    let path = std::env::var("WUBI_TABLE_ORIGINAL").unwrap();
    let content = fs::read_to_string(&path).unwrap();
    let table = WubiTableOriginal::from_str(&content).unwrap();
    println!("Parsed {} entries", table.entries.len());
    println!(
        "First entry: {} {}",
        table.entries[0].character, table.entries[0].wubi_code
    );
}
