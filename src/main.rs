#![forbid(unsafe_code)]

use std::{
    cmp::Ordering,
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
    drop(cjk);

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
    drop(phrases);

    let table = Table::new(simplified, full);

    println!("Generating table");
    let mut table_file = io::BufWriter::new(fs::File::create("wb_nc_table.txt").unwrap());
    let mut ios_table_file = io::BufWriter::new(fs::File::create("wb_nc_ios_table.txt").unwrap());
    let mut simplified_table = table.simplified_table();
    let mut full_table = table.filtered_full_table();
    loop {
        match (simplified_table.next(), full_table.next()) {
            (None, None) => break,
            (None, Some((code, phrases))) => {
                write!(table_file, "{code}").unwrap();
                for phrase in phrases {
                    write!(table_file, " {phrase}").unwrap();
                    writeln!(ios_table_file, "{code}={phrase}").unwrap();
                }
                writeln!(table_file).unwrap();
            }
            (Some((code, ch)), None) => {
                writeln!(table_file, "{code} {ch}").unwrap();
                writeln!(ios_table_file, "{code}={ch}").unwrap();
            }
            (Some((simplified_code, simplified_ch)), Some((full_code, full_phrases))) => {
                match simplified_code.cmp(&full_code) {
                    Ordering::Less => {
                        writeln!(table_file, "{simplified_code} {simplified_ch}").unwrap();
                        writeln!(ios_table_file, "{simplified_code}={simplified_ch}").unwrap();
                        write!(table_file, "{full_code}").unwrap();
                        for phrase in full_phrases {
                            write!(table_file, " {phrase}").unwrap();
                            writeln!(ios_table_file, "{full_code}={phrase}").unwrap();
                        }
                        writeln!(table_file).unwrap();
                    }
                    Ordering::Equal => {
                        write!(table_file, "{full_code} {simplified_ch}").unwrap();
                        writeln!(ios_table_file, "{full_code}={simplified_ch}").unwrap();
                        for phrase in full_phrases {
                            write!(table_file, " {phrase}").unwrap();
                            writeln!(ios_table_file, "{full_code}={phrase}").unwrap();
                        }
                        writeln!(table_file).unwrap();
                    }
                    Ordering::Greater => {
                        write!(table_file, "{full_code}").unwrap();
                        for phrase in full_phrases {
                            write!(table_file, " {phrase}").unwrap();
                            writeln!(ios_table_file, "{full_code}={phrase}").unwrap();
                        }
                        writeln!(table_file).unwrap();
                        writeln!(table_file, "{simplified_code} {simplified_ch}").unwrap();
                        writeln!(ios_table_file, "{simplified_code}={simplified_ch}").unwrap();
                    }
                }
            }
        }
    }
    drop(table_file);
    drop(ios_table_file);

    println!("Generating reverse table");
    let mut reverse_table_file =
        io::BufWriter::new(fs::File::create("wb_nc_reverse_table.txt").unwrap());
    let mut simplified_table = table.reverse_simplified_table();
    let mut full_table = table.reverse_filtered_full_table();
    loop {
        match (simplified_table.next(), full_table.next()) {
            (None, None) => break,
            (None, Some((phrase, code))) => {
                writeln!(reverse_table_file, "{phrase} {code}").unwrap();
            }
            (Some((ch, codes)), None) => {
                write!(reverse_table_file, "{ch}").unwrap();
                for code in codes {
                    write!(reverse_table_file, " {code}").unwrap();
                }
                writeln!(reverse_table_file).unwrap();
            }
            (Some((simplified_ch, simplified_codes)), Some((full_phrase, full_code))) => {
                match simplified_ch.to_string().cmp(&full_phrase) {
                    Ordering::Less => {
                        write!(reverse_table_file, "{simplified_ch}").unwrap();
                        for code in simplified_codes {
                            write!(reverse_table_file, " {code}").unwrap();
                        }
                        writeln!(reverse_table_file).unwrap();
                        writeln!(reverse_table_file, "{full_phrase} {full_code}").unwrap();
                    }
                    Ordering::Equal => {
                        write!(reverse_table_file, "{simplified_ch}").unwrap();
                        for code in simplified_codes {
                            write!(reverse_table_file, " {code}").unwrap();
                        }
                        writeln!(reverse_table_file, " {full_code}").unwrap();
                    }
                    Ordering::Greater => {
                        writeln!(reverse_table_file, "{full_phrase} {full_code}").unwrap();
                        write!(reverse_table_file, "{simplified_ch}").unwrap();
                        for code in simplified_codes {
                            write!(reverse_table_file, " {code}").unwrap();
                        }
                        writeln!(reverse_table_file).unwrap();
                    }
                }
            }
        }
    }
}
