use crate::Extract::*;
use clap::{App, Arg};
use csv::{ReaderBuilder, StringRecord};
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
    ops::Range,
    str,
};

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
        )
        .arg(
            Arg::with_name("chars")
            .value_name("CHARS")
            .short("c")
            .long("chars")
            .help("Selected characters")
            .takes_value(true)
            .conflicts_with("fields")
        )
        .arg(
            Arg::with_name("fields")
            .value_name("FIELDS")
            .short("f")
            .long("fields")
            .help("Selected fields")
            .takes_value(true)
        )
        .get_matches();
    let delimiter = matches.value_of("delimiter").unwrap();
    if delimiter.len() != 1 {
        return Err(From::from(format!("--delim \"{}\" must be a single byte", delimiter)));
    }
    let extract = if matches.is_present("bytes") {
        Bytes(parse_pos(matches.value_of("bytes").unwrap())?)
    } else if matches.is_present("chars") {
        Chars(parse_pos(matches.value_of("chars").unwrap())?)
    } else if matches.is_present("fields") {
        Fields(parse_pos(matches.value_of("fields").unwrap())?)
    } else {
        return Err(From::from("Must have --fields, --bytes, or --chars"));
    };
    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        delimiter: delimiter.bytes().nth(0).unwrap(),
        extract,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    for filename in config.files {
        match open(&filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(file) => {
                match &config.extract {
                    Fields(field_pos) => {
                        let mut reader = ReaderBuilder::new()
                            .has_headers(false)
                            .delimiter(config.delimiter)
                            .from_reader(file);
                        for record in reader.records() {
                            let fields = extract_fields(&record?, field_pos);
                            println!("{}", fields.join(str::from_utf8(&[config.delimiter])?));
                        }
                    }
                    Bytes(byte_pos) => {
                        for line in file.lines() {
                            let bytes = extract_bytes(&line?, byte_pos);
                            println!("{}", bytes);
                        }
                    }
                    Chars(char_pos) => {
                        for line in file.lines() {
                            let chars = extract_chars(&line?, char_pos);
                            println!("{}", chars);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?)))
    }
}

fn parse_pos(range: &str) -> MyResult<PositionList> {
    let value_error = |v: &str| -> Box<dyn Error> {
        From::from(format!("illegal list value: \"{}\"", v))
    };
    if range.is_empty() {
        return Err(From::from("position lists cannot be empty"));
    }
    let parts = range.split(",").collect::<Vec<_>>();
    let mut list: PositionList = vec![];
    for part in parts {
        if part.is_empty() {
            return Err(value_error(range));
        }
        let interval = part.split("-").collect::<Vec<_>>();
        if interval.len() > 2 {
            return Err(value_error(range));
        }
        let bounds = interval.into_iter()
            .map(|endpoint| if endpoint.starts_with("+") {
                Err(value_error(part))
            } else {
                let bound = endpoint.parse::<usize>().map_err(|_| value_error(part))?;
                if bound == 0 {
                    Err(value_error("0"))
                } else {
                    Ok(bound)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let lower = bounds[0];
        if bounds.len() == 1 {
            list.push(lower - 1..lower);
        } else {
            let upper = bounds[1];
            if upper == 0 {
                return Err(value_error("0"));
            }
            if lower >= upper {
                return Err(From::from(format!("First number in range ({}) must be lower than second number ({})", lower, upper)));
            }
            list.push(lower - 1..upper);
        }
    }
    Ok(list)
}

fn extract_chars(line: &str, char_pos: &[Range<usize>]) -> String {
    let chars = line.chars().collect::<Vec<_>>();
    char_pos.iter()
        .cloned()
        .flat_map(|r| {
            r.filter_map(|i| chars.get(i))
        })
        .collect()
}

fn extract_bytes(line: &str, byte_pos: &[Range<usize>]) -> String {
    let bytes = line.as_bytes();
    let extracted = byte_pos.iter()
        .cloned()
        .flat_map(|r| {
            r.filter_map(|i| bytes.get(i)).copied()
        })
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&extracted).into_owned()
}

fn extract_fields(record: &StringRecord, field_pos: &[Range<usize>]) -> Vec<String> {
    field_pos.iter()
        .cloned()
        .flat_map(|r| {
            r.filter_map(|i| record.get(i))
        })
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod unit_tests {
    use csv::StringRecord;
    use super::{parse_pos, extract_chars, extract_bytes, extract_fields};

    #[test]
    fn test_parse_pos() {
        // The empty string is an error
        let res = parse_pos("");
        assert!(res.is_err());

        // Zero is an error
        let res = parse_pos("0");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"");

        let res = parse_pos("0-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"");

        // A leading "+" is an error
        let res = parse_pos("+1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1\"");

        let res = parse_pos("+1-2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1-2\"");

        let res = parse_pos("1-+2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-+2\"");

        // Any non-number is an error
        let res = parse_pos("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"");

        let res = parse_pos("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"");

        let res = parse_pos("1-a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-a\"");

        let res = parse_pos("a-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a-1\"");

        // Ill-shaped ranges
        let res = parse_pos("-");
        assert!(res.is_err());

        let res = parse_pos(",");
        assert!(res.is_err());

        let res = parse_pos("1,");
        assert!(res.is_err());

        let res = parse_pos("1-");
        assert!(res.is_err());

        let res = parse_pos("1-1-1");
        assert!(res.is_err());

        let res = parse_pos("1-1-a");
        assert!(res.is_err());

        // First number must be less than second
        let res = parse_pos("1-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(), 
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_pos("2-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(), 
            "First number in range (2) must be lower than second number (1)"
        );

        // All the following are acceptable
        let res = parse_pos("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("01");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("1,3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("001,0003");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("1-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("0001-03");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("1,7,3-5");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_pos("15,19-20");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }

    #[test]
    fn test_extract_chars() {
        assert_eq!(extract_chars("", &[0..1]), "".to_string());
        assert_eq!(extract_chars("??bc", &[0..1]), "??".to_string());
        assert_eq!(extract_chars("??bc", &[0..1, 2..3]), "??c".to_string());
        assert_eq!(extract_chars("??bc", &[0..3]), "??bc".to_string());
        assert_eq!(extract_chars("??bc", &[2..3, 1..2]), "cb".to_string());
        assert_eq!(extract_chars("??bc", &[0..1, 1..2, 4..5]), "??b".to_string());
    }

    #[test]
    fn test_extract_bytes() {
        assert_eq!(extract_bytes("??bc", &[0..1]), "???".to_string());
        assert_eq!(extract_bytes("??bc", &[0..2]), "??".to_string());
        assert_eq!(extract_bytes("??bc", &[0..3]), "??b".to_string());
        assert_eq!(extract_bytes("??bc", &[0..4]), "??bc".to_string());
        assert_eq!(extract_bytes("??bc", &[3..4, 2..3]), "cb".to_string());
        assert_eq!(extract_bytes("??bc", &[0..2, 5..6]), "??".to_string());
    }

    #[test]
    fn test_extract_fields() {
        let rec = StringRecord::from(vec!["Captain", "Sham", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2]), &["Sham"]);
        assert_eq!(extract_fields(&rec, &[0..1, 2..3]), &["Captain", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1, 3..4]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2, 0..1]), &["Sham", "Captain"]);
    }
}
