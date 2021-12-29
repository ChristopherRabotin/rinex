//! This package provides a set of tools to parse 
//! and analyze RINEX files.
//! 
//! Homepage: <https://github.com/gwbres/rinex>

use thiserror::Error;
use std::str::FromStr;
use scan_fmt::scan_fmt;

mod header;
mod record;
mod version;
mod constellation;

#[macro_export]
macro_rules! is_rinex_comment {
    ($line: expr) => { $line.contains("COMMENT") };
}

/// `Rinex` main structure,
/// describes a `RINEX` file
#[derive(Debug)]
pub struct Rinex {
    header: header::Header,
    records: Vec<record::RinexRecord>,
}

impl Default for Rinex {
    /// Builds a default `RINEX`
    fn default() -> Rinex {
        Rinex {
            header: header::Header::default(),
            records: Vec::new(),
        }
    }
}

#[derive(Error, Debug)]
pub enum RinexError {
    #[error("Header delimiter not found")]
    MissingHeaderDelimiter,
    #[error("Header parsing error")]
    HeaderError(#[from] header::HeaderError),
}

/// macro to return true when a new block record
/// has been identified
pub fn new_record_block (line: &str,
    rinex_type: &header::RinexType,
        constellation: &constellation::Constellation, 
            version: &version::Version) -> bool
{
    let parsed: Vec<&str> = line.split_ascii_whitespace()
        .collect();
    
    match rinex_type {
        header::RinexType::NavigationMessage => {
            let known_sv_identifiers: &'static [char] = 
                &['R','G','E','B','J','C','S']; 
            match constellation {
                constellation::Constellation::Glonass => parsed.len() > 4,
                _ => {
                    match line.chars().nth(0) {
                        Some(c) => known_sv_identifiers.contains(&c), 
                        _ => false
                            //TODO
                            // for some file we end but with "\nxxx"
                            // as the very first item,
                            // current code will drop first payload item
                    }
                }
            }
        },
        _ => false,
    }
}

use record::*;

impl Rinex {
    /// Builds a Rinex struct
    pub fn new (header: header::Header, records: Vec<record::RinexRecord>) -> Rinex {
        Rinex {
            header,
            records,
        }
    }

    /// splits rinex file into two
    /// (header, body) as strings
    fn split_rinex_content (fp: &std::path::Path) -> Result<(String, String), RinexError> {
        let content: String = std::fs::read_to_string(fp)
            .unwrap()
                .parse()
                .unwrap();
        let offset = match content.find(header::HEADER_END_MARKER) {
            Some(offset) => offset+13,
            _ => return Err(RinexError::MissingHeaderDelimiter)
        };
        let (header, body) = content.split_at(offset);
        Ok((String::from(header),String::from(body)))
    }

    /// Builds a `Rinex` from given file.
    /// Input file must respect the whitespace specifications
    /// for the entire header section.   
    /// The header section must respect the labelization standard too.
    pub fn from_file (fp: &std::path::Path) -> Result<Rinex, RinexError> {
        let name = fp.file_name()
            .unwrap();
        let extension = fp.extension()
            .unwrap();
        let extension = extension.to_str()
            .unwrap();

        let (header, body) = Rinex::split_rinex_content(fp)?;
        let header = header::Header::from_str(&header)?;

        let rinex_type = header.get_rinex_type();
        let version = header.get_rinex_version();
        let version_major = version.get_major(); 
        let version_minor = version.get_minor(); 
        let constellation = header.get_constellation();

        let mut body = body.lines();
        let mut line = body.next()
            .unwrap(); // ''END OF HEADER'' /BLANK

        while is_rinex_comment!(line) {
            line = body.next()
                .unwrap()
        }

        let mut eof = false;
        let mut first = true;
        let mut block = String::with_capacity(256*1024); // max. block size
        let mut records: Vec<RinexRecord> = Vec::new();

        loop {
            let parsed: Vec<&str> = line.split_ascii_whitespace()
                .collect();
            
            let new_block = new_record_block(&line, &rinex_type, &constellation, &version); 
            if new_block && !first {
                let record: Option<RinexRecord> = match rinex_type {
                    header::RinexType::NavigationMessage => {
                        if let Ok(record) = 
                            navigation::NavigationRecord::from_string(&version, &constellation, &block) {
                                Some(RinexRecord::RinexNavRecord(record))
                        } else {
                            None
                        }
                    },
                    _ => None,
                };
                
                if record.is_some() {
                    records.push(record.unwrap())
                }
            }

            if new_block {
                if first {
                    first = false
                }
                block.clear()
            }

            block.push_str(&line);
            block.push_str(" ");

            if let Some(l) = body.next() {
                line = l
            } else {
                break
            }

            while is_rinex_comment!(line) {
                if let Some(l) = body.next() {
                    line = l
                } else {
                    eof = true; 
                    break 
                }
            }

            if eof {
                break
            }
        }

        Ok(Rinex{
            header, 
            records,
        })
    }
}

mod test {
    use super::*;
    #[test]
    /// Test `Rinex` constructor
    /// against all valid data resources
    fn test_rinex_constructor() {
        // open test resources
        let test_resources = std::path::PathBuf::from(
            env!("CARGO_MANIFEST_DIR").to_owned() + "/data");
        // walk test resources
        for entry in std::fs::read_dir(test_resources)
            .unwrap() {
            let entry = entry
                .unwrap();
            let path = entry.path();
            if !path.is_dir() { // only files..
                let fp = std::path::Path::new(&path);
                let rinex = Rinex::from_file(&fp);
                assert_eq!(rinex.is_err(), false);
                println!("File: {:?}\n{:#?}", &fp, rinex)
            }
        }
    }
}
