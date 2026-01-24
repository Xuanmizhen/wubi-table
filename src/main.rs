#![forbid(unsafe_code)]

use std::{
    fs,
    io::{self, BufRead as _, Write as _},
};
use table::*;
use thiserror::Error;

pub mod table;

// TODO: refuse 16-bits computer

#[derive(Error, Debug, PartialEq)]
#[non_exhaustive]
pub enum ParseError {
    #[error("empty")]
    Empty,
    #[error("too long code: {0:?}")]
    TooLongCode(Vec<u8>),
    #[error("No '\\t' found: {0}")]
    NoTabFound(String),
    #[error("More than one character found: {0}")]
    MultipleCharacters(String),
    #[error("Not ASCII lowercase")]
    NotValidChar,
    #[error("Invalid format")]
    Invalid,
    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Codepoint does not match character")]
    CodepointMismatch,
}

#[derive(Clone, Debug)]
pub struct WubiEntry {
    phrase: String,
    wubi_code: WubiCode,
}

fn parse_line_with_codepoint(line: &str) -> Result<WubiEntry, ParseError> {
    let (codepoint, rest) = line
        .split_once('\t')
        .ok_or(ParseError::NoTabFound(line.to_string()))?;
    let (ch, wubi) = rest
        .split_once('\t')
        .ok_or(ParseError::NoTabFound(line.to_string()))?;
    let ch = {
        if ch.chars().count() != 1 {
            return Err(ParseError::MultipleCharacters(ch.to_string()));
        }
        ch.chars().next().expect("Checked above")
    };
    if !(codepoint.starts_with("U+") && u32::from_str_radix(&codepoint[2..], 16)? == ch as u32) {
        return Err(ParseError::CodepointMismatch);
    }
    let mut cnt = 0;
    for b in wubi.as_bytes() {
        if !(b'a'..=b'y').contains(b) {
            return Err(ParseError::NotValidChar);
        }
        cnt += 1; // TODO: check overflow
    }
    if !(1..=4).contains(&cnt) {
        return Err(ParseError::Invalid);
    }
    Ok(WubiEntry {
        phrase: ch.to_string(),
        wubi_code: wubi.try_into()?,
    })
}

fn get_lines(read: &mut io::BufReader<fs::File>) -> impl Iterator<Item = String> {
    read.lines().map(|line| line.unwrap())
}

fn main() {
    // env_logger::init();

    let mut simplified = SimplifiedCodeTable::new();
    for i in 1..=3 {
        let file = format!("simplified{i}.txt");
        println!("Loading simplified table from {}", file);
        let mut file = io::BufReader::new(fs::File::open(file).unwrap());
        for line in get_lines(&mut file) {
            let (chars, code) = line.split_once('\t').unwrap();
            let mut chars = chars.chars();
            if let Some(ch) = chars.next() {
                assert!(
                    chars.next().is_none(),
                    "Simplified code is for single character"
                );
                let code = code.try_into().unwrap();
                simplified.insert(&code, ch).unwrap();
            } else {
                todo!()
            }
        }
    }

    println!("Loading full table");
    let mut full = FullCodeTable::new();
    let mut cjk = io::BufReader::new(fs::File::open("CJK.txt").unwrap());
    for line in get_lines(&mut cjk) {
        let entry = parse_line_with_codepoint(line.as_str()).unwrap();
        full.insert(entry);
    }

    println!("Loading phrases");
    let mut phrases = io::BufReader::new(fs::File::open("phrases.txt").unwrap());
    for phrase in get_lines(&mut phrases) {
        let wubi_code = get_code_for_phrase(phrase.as_str(), |ch| {
            let code = full.code(&ch.to_string()).unwrap();
            // *code.last().unwrap()
            *code
        });
        let entry = WubiEntry { phrase, wubi_code };
        full.insert(entry);
    }

    let table = Table::new(simplified, full);

    println!("Generating reverse table");
    let mut reverse_table_file =
        io::BufWriter::new(fs::File::create("wb_nc_reverse_table.txt").unwrap());
    for (wubi_code, ch) in table.reverse_simplified_table() {
        writeln!(reverse_table_file, "{} {}", wubi_code, ch).unwrap();
    }
    for (code, phrases) in table.reverse_filtered_full_table() {
        let phrases = phrases;
        write!(reverse_table_file, "{}", code).unwrap();
        for phrase in phrases {
            write!(reverse_table_file, " {}", phrase).unwrap();
        }
        writeln!(reverse_table_file).unwrap();
    }

    println!("Generating table");
    let mut table_file = io::BufWriter::new(fs::File::create("wb_nc_table.txt").unwrap());
    for (ch, wubi_code) in table.simplified_table() {
        write!(table_file, "{}", ch).unwrap();
        for code in wubi_code {
            write!(table_file, " {}", code).unwrap();
        }
        writeln!(table_file).unwrap();
    }
}
