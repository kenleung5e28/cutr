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
    if range.is_empty() {
        return Err(From::from("position lists cannot be empty"));
    }
    let parts = range.split(",").collect::<Vec<_>>();
    let mut list: PositionList = vec![];
    for part in parts {
        if part.is_empty() {
            return Err(From::from(format!("illegal list value: \"{}\"", range)));
        }
        let intervals = part.split("-").collect::<Vec<_>>();
        if intervals.len() > 2 {
            return Err(From::from(format!("illegal list value: \"{}\"", range)));
        }
        let mut bounds: Vec<usize> = vec![];
        for endpoint in intervals {
            if endpoint.starts_with("+") {
                return Err(From::from(format!("illegal list value: \"{}\"", part)));
            }
            let bound = endpoint.parse::<usize>()
                .map_err(|_| -> Box<dyn Error> {
                    From::from(format!("illegal list value: \"{}\"", part))
                })?;
            bounds.push(bound);
        }
        let lower = bounds[0];
        if lower == 0 {
            return Err(From::from("illegal list value: \"0\""));
        }
        if bounds.len() == 1 {
            list.push(lower - 1..lower);
        } else {
            let upper = bounds[1];
            if upper == 0 {
                return Err(From::from("illegal list value: \"0\""));
            }
            if lower >= upper {
                return Err(From::from(format!("First number in range ({}) must be lower than second number ({})", lower, upper)));
            }
            list.push(lower - 1..upper);
        }
    }
    Ok(list)
}

#[cfg(test)]
mod unit_tests {
    use super::parse_pos;

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
}
