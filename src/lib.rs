use crate::Extract::*;
use clap::{App, Arg};
use std::{error::Error, ops::Range};

type MyResult<T> = Result<T, Box<dyn Error>>;
type PositionList = Vec<Range<usize>>;

#[derive(Debug)]
pub enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    delimiter: u8,
    extract: Extract,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("cutr")
        .version("0.1.0")
        .author("Ken C.Y. Leung <kenleung5e28@gmail.com>")
        .about("Rust cut")
        .arg(
            Arg::with_name("files")
            .value_name("FILE")
            .help("Input file(s)")
            .default_value("-")
            .multiple(true)
        )
        .arg(
            Arg::with_name("delimiter")
            .value_name("DELIMITER")
            .short("d")
            .long("delim")
            .help("Field delimiter")
            .default_value("\t")
        )
        .arg(
            Arg::with_name("bytes")
            .value_name("BYTES")
            .short("b")
            .long("bytes")
            .help("Selected bytes")
            .takes_value(true)
            .conflicts_with_all(&["chars", "fields"])
            .required_unless_one(&["chars", "fields"])
        )
        .arg(
            Arg::with_name("chars")
            .value_name("CHARS")
            .short("c")
            .long("chars")
            .help("Selected characters")
            .takes_value(true)
            .conflicts_with("fields")
            .required_unless_one(&["bytes", "fields"])
        )
        .arg(
            Arg::with_name("fields")
            .value_name("FIELDS")
            .short("f")
            .long("fields")
            .help("Selected fields")
            .takes_value(true)
            .required_unless_one(&["bytes", "chars"])
        )
        .get_matches();
    let delimiter = matches.value_of("delim").unwrap();
    if delimiter.len() != 1 {
        return Err(From::from(format!("--delim \"{}\" must be a single byte", delimiter)));
    }
    let extract = if matches.is_present("bytes") {
        Bytes(parse_pos(matches.value_of("bytes").unwrap())?)
    } else if matches.is_present("chars") {
        Chars(parse_pos(matches.value_of("chars").unwrap())?)
    } else {
        Fields(parse_pos(matches.value_of("fields").unwrap())?)
    };
    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        delimiter: delimiter.bytes().nth(0).unwrap(),
        extract,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    println!("{:#?}", config);
    Ok(())
}

fn parse_pos(range: &str) -> MyResult<PositionList> {
    unimplemented!();
}
