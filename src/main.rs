#![forbid(unsafe_code)]

use std::{fs, io::{self, BufRead as _}};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid format")]
    Invalid,
    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Codepoint does not match character")]
    CodepointMismatch,
}

pub struct WubiEntry {
    phrase: String,
    wubi_code: Vec<u8>,
}

fn parse_line_with_codepoint(line: &str) -> Result<WubiEntry, ParseError> {
    let (codepoint, rest) = line.split_once('\t').ok_or(ParseError::Invalid)?;
    let (ch, wubi) = rest.split_once('\t').ok_or(ParseError::Invalid)?;
    let ch = {
        if ch.chars().count() != 1 {
            return Err(ParseError::Invalid);
        }
        ch.chars().next().expect("Checked above")
    };
    if !(codepoint.starts_with("U+") && u32::from_str_radix(&codepoint[2..], 16)? == ch as u32) {
        return Err(ParseError::CodepointMismatch);
    }
    let mut cnt = 0;
    for b in wubi.as_bytes() {
        if !b.is_ascii_lowercase() {
            return Err(ParseError::Invalid);
        }
        cnt += 1; // TODO: check overflow
    }
    if !(1..=4).contains(&cnt) {
        return Err(ParseError::Invalid);
    }
    Ok(WubiEntry {
        phrase: ch.to_string(),
        wubi_code: wubi.as_bytes().to_vec(),
    })
}

fn parse_line_without_codepoint(line: &str) -> Result<WubiEntry, ParseError> {
    let (ch, wubi) = line.split_once('\t').ok_or(ParseError::Invalid)?;
    let ch = {
        if ch.chars().count() != 1 {
            return Err(ParseError::Invalid);
        }
        ch.chars().next().expect("Checked above")
    };
    let mut cnt = 0;
    for b in wubi.as_bytes() {
        if !b.is_ascii_lowercase() {
            return Err(ParseError::Invalid);
        }
        cnt += 1; // TODO: check overflow
    }
    if !(1..=4).contains(&cnt) {
        return Err(ParseError::Invalid);
    }
    Ok(WubiEntry {
        phrase: ch.to_string(),
        wubi_code: wubi.as_bytes().to_vec(),
    })
}

pub struct WubiTable {
    pub entries: Vec<WubiEntry>,
}

impl WubiTable {
    pub fn build_with_codepoint<R: io::BufRead>(
        reader: R,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(WubiTable {
            entries: reader
                .lines()
                .map(|line| {
                    let line = line?;
                    let entry = parse_line_with_codepoint(&line)?;
                    Ok::<_, Box<dyn std::error::Error>>(entry)
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn build_without_codepoint<R: io::BufRead>(
        reader: R,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(WubiTable {
            entries: reader
                .lines()
                .map(|line| {
                    let line = line?;
                    let entry = parse_line_without_codepoint(&line)?;
                    Ok::<_, Box<dyn std::error::Error>>(entry)
                })
                .collect::<Result<_, _>>()?,
        })
    }

    fn binary_search_code(&self, character: char) -> Option<Vec<u8>> {
        let character = character.to_string();
        let left = self.entries.partition_point(|e| e.phrase < character);
        let right = self.entries.partition_point(|e| e.phrase <= character);
        self.entries[left..right].iter().map(|e| e.wubi_code.clone()).max_by_key(|p| p.len())
    }

    pub fn extend_phrases<I: Iterator<Item = String>>(&mut self, phrases: I) {
        self.entries.sort_by_key(|entry| entry.phrase.clone());
        for phrase in phrases {
            let mut chars = phrase.chars();
            let wubi_code: Vec<u8> = match chars.clone().count() {
                0 | 1 => continue,
                2 => {
                    let first = chars.next().expect("checked");
                    let second = chars.next().expect("checked");
                    let first = self.binary_search_code(first).unwrap();
                    let second = self.binary_search_code(second).unwrap();
                    first
                        .split_at_checked(2)
                        .unwrap()
                        .0
                        .iter()
                        .chain(second.split_at_checked(2).unwrap().0.iter())
                        .cloned()
                        .collect()
                }
                3 => {
                    let first = chars.next().expect("checked");
                    let second = chars.next().expect("checked");
                    let third = chars.next().expect("checked");
                    let first = self.binary_search_code(first).unwrap();
                    let second = self.binary_search_code(second).unwrap();
                    let third = self.binary_search_code(third).unwrap();
                    first
                        .split_at_checked(1)
                        .unwrap()
                        .0
                        .iter()
                        .chain(second.split_at_checked(1).unwrap().0.iter())
                        .chain(third.split_at_checked(2).unwrap().0.iter())
                        .cloned()
                        .collect()
                }
                4.. => {
                    let first = chars.next().expect("checked");
                    let second = chars.next().expect("checked");
                    let third = chars.next().expect("checked");
                    let last = chars.last().expect("checked");
                    let first = self.binary_search_code(first).unwrap();
                    let second = self.binary_search_code(second).unwrap();
                    let third = self.binary_search_code(third).unwrap();
                    let last = self.binary_search_code(last).unwrap();
                    first
                        .split_at_checked(1)
                        .unwrap()
                        .0
                        .iter()
                        .chain(second.split_at_checked(1).unwrap().0.iter())
                        .chain(third.split_at_checked(1).unwrap().0.iter())
                        .chain(last.split_at_checked(1).unwrap().0.iter())
                        .cloned()
                        .collect()
                }
            };

            let entry = WubiEntry { phrase, wubi_code };
            self.entries.insert(self.entries.partition_point(|e| e.phrase < entry.phrase), entry);
        }
    }

    pub fn unique_reverse_table(&mut self) -> bool {
        self.entries
            .sort_by(|a, b| a.phrase.cmp(&b.phrase));
        for i in 1..self.entries.len() {
            if self.entries[i - 1].phrase == self.entries[i].phrase {
                return false;
            }
        }
        true
    }

    pub fn write_reverse_table<W: io::Write>(&mut self, mut writer: W) -> io::Result<()> {
        self.entries.sort_by(|a, b| {
            a.phrase
                .cmp(&b.phrase)
                .then(a.wubi_code.len().cmp(&b.wubi_code.len()))
        });

        let mut iter = self.entries.iter();
        let mut last_entry = {
            if let Some(first) = iter.next() {
                write!(writer, "{} ", first.phrase)?;
                writer.write_all(&first.wubi_code)?;
                first
            } else {
                return Ok(());
            }
        };
        for entry in iter {
            if entry.phrase != last_entry.phrase {
                write!(writer, "\n{} ", entry.phrase)?;
            } else {
                if entry.wubi_code == last_entry.wubi_code {
                    continue;
                }
                writer.write_all(b" ")?;
            }
            writer.write_all(&entry.wubi_code)?;
            last_entry = entry;
        }
        writer.write_all(b"\n")
    }

    pub fn write_table<W: io::Write>(&mut self, mut writer: W) -> io::Result<()> {
        self.entries
            .sort_by(|a, b| a.wubi_code.cmp(&b.wubi_code));

        let mut iter = self.entries.iter();
        let mut last_entry = {
            if let Some(first) = iter.next() {
                writer.write_all(&first.wubi_code)?;
                write!(writer, " {}", first.phrase)?;
                first
            } else {
                return Ok(());
            }
        };
        for entry in iter {
            if entry.wubi_code != last_entry.wubi_code {
                writer.write_all(b"\n")?;
                writer.write_all(&entry.wubi_code)?;
            } else if entry.phrase == last_entry.phrase {
                continue;
            }
            write!(writer, " {}", entry.phrase)?;
            last_entry = entry;
        }
        writer.write_all(b"\n")
    }
}

fn main() {
    let original =
        io::BufReader::new(fs::File::open(std::env::var("WUBI_TABLE_ORIGINAL").unwrap()).unwrap());
    let mut table = WubiTable::build_with_codepoint(original).unwrap();
    assert!(table.unique_reverse_table());

    let simplified = io::BufReader::new(
        fs::File::open(std::env::var("WUBI_TABLE_SIMPLIFIED").unwrap()).unwrap(),
    );
    table.entries.extend(
        WubiTable::build_without_codepoint(simplified)
            .unwrap()
            .entries,
    );

    let mut reverse_table_file =
        io::BufWriter::new(fs::File::create("wb_nc_reverse_table.txt").unwrap());
    table.write_reverse_table(&mut reverse_table_file).unwrap();

    let phrases = io::BufReader::new(
        fs::File::open(std::env::var("WUBI_PHRASES").unwrap()).unwrap(),
    )
    .lines()
    .map(|line| line.unwrap());
    table.extend_phrases(phrases);

    let mut table_file = io::BufWriter::new(fs::File::create("wb_nc_table.txt").unwrap());
    table.write_table(&mut table_file).unwrap();
}
