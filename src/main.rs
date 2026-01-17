#![forbid(unsafe_code)]

use std::{fs, io};
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
    character: char,
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
        character: ch,
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
        character: ch,
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

    pub fn unique_reverse_table(&mut self) -> bool {
        self.entries
            .sort_unstable_by(|a, b| a.character.cmp(&b.character));
        for i in 1..self.entries.len() {
            if self.entries[i - 1].character == self.entries[i].character {
                return false;
            }
        }
        true
    }

    pub fn write_reverse_table<W: io::Write>(&mut self, mut writer: W) -> io::Result<()> {
        self.entries.sort_unstable_by(|a, b| {
            a.character
                .cmp(&b.character)
                .then(a.wubi_code.len().cmp(&b.wubi_code.len()))
        });

        let mut iter = self.entries.iter();
        let mut last_entry = {
            if let Some(first) = iter.next() {
                write!(writer, "{} ", first.character)?;
                writer.write_all(&first.wubi_code)?;
                first
            } else {
                return Ok(());
            }
        };
        for entry in iter {
            if entry.character != last_entry.character {
                write!(writer, "\n{} ", entry.character)?;
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
            .sort_unstable_by(|a, b| a.wubi_code.cmp(&b.wubi_code));

        let mut iter = self.entries.iter();
        let mut last_entry = {
            if let Some(first) = iter.next() {
                writer.write_all(&first.wubi_code)?;
                write!(writer, " {}", first.character)?;
                first
            } else {
                return Ok(());
            }
        };
        for entry in iter {
            if entry.wubi_code != last_entry.wubi_code {
                writer.write_all(b"\n")?;
                writer.write_all(&entry.wubi_code)?;
            } else if entry.character == last_entry.character {
                continue;
            }
            write!(writer, " {}", entry.character)?;
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
    let mut table_file = io::BufWriter::new(fs::File::create("wb_nc_table.txt").unwrap());
    table.write_table(&mut table_file).unwrap();
}
