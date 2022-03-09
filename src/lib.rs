//! This package provides a set of tools to parse 
//! `RINEX` files.
//! 
//! Refer to README for example of use.  
//! Homepage: <https://github.com/gwbres/rinex>
mod meteo;
mod header;
mod version;
mod gnss_time;
mod navigation;
mod observation;

pub mod epoch;
pub mod record;
pub mod constellation;

use thiserror::Error;
use std::str::FromStr;

#[macro_export]
/// Returns `true` if given `Rinex` line is a comment
macro_rules! is_comment {
    ($line: expr) => { $line.contains("COMMENT") };
}

/// Describes all known `RINEX` file types
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Type {
    /// Describes Observation Data (OBS),
    /// Phase & Pseudo range measurements
    ObservationData, 
    /// Describes Navigation Message (NAV)
    /// Ephemeride file
    NavigationMessage,
    /// Describes Meteorological data (Meteo)
    MeteorologicalData,
}

#[derive(Error, Debug)]
/// `Type` related errors
pub enum TypeError {
    #[error("Unknown RINEX type identifier \"{0}\"")]
    UnknownType(String),
}

impl Default for Type {
    /// Builds a default `Type`
    fn default() -> Type { Type::ObservationData }
}

impl Type {
    /// Converts `Self` to str
    pub fn to_str (&self) -> &str {
        match *self {
            Type::ObservationData => "ObservationData",
            Type::NavigationMessage => "NavigationMessage",
            Type::MeteorologicalData => "MeteorologicalData",
        }
    }
    /// Converts `Self` to string
    pub fn to_string (&self) -> String { String::from(self.to_str()) }
}

impl std::str::FromStr for Type {
    type Err = TypeError;
    fn from_str (s: &str) -> Result<Self, Self::Err> {
        if s.eq("NAVIGATION DATA") {
            Ok(Type::NavigationMessage)
        } else if s.contains("NAV DATA") {
            Ok(Type::NavigationMessage)
        } else if s.eq("OBSERVATION DATA") {
            Ok(Type::ObservationData)
        } else if s.eq("METEOROLOGICAL DATA") {
            Ok(Type::MeteorologicalData)
        } else {
            Err(TypeError::UnknownType(String::from(s)))
        }
    }
}

/// `Rinex` describes a `RINEX` file
#[derive(Debug)]
pub struct Rinex {
    pub header: header::RinexHeader,
    pub record: Option<record::Record>,
}

impl Default for Rinex {
    /// Builds a default `RINEX`
    fn default() -> Rinex {
        Rinex {
            header: header::RinexHeader::default(),
            record: None, 
        }
    }
}

#[derive(Error, Debug)]
/// `RINEX` Parsing related errors
pub enum RinexError {
    #[error("Header delimiter not found")]
    MissingHeaderDelimiter,
    #[error("Header parsing error")]
    HeaderError(#[from] header::Error),
    #[error("Rinex type error")]
    TypeError(#[from] TypeError),
}

impl Rinex {
    /// Builds a new `RINEX` struct from given:
    pub fn new (header: header::RinexHeader, record: Option<record::Record>) -> Rinex {
        Rinex {
            header,
            record,
        }
    }

    /// splits rinex file into two (header, body) contents
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

    // Returns Record nth' entry
    //pub fn get_record_nth (&self, nth: usize) 
    //    -> &std::collections::HashMap<String, record::RecordItem> { &self.record[nth] }

    /// Retruns true if this is an NAV rinex
    pub fn is_navigation_rinex (&self) -> bool { self.header.rinex_type == Type::NavigationMessage }
    /// Retruns true if this is an OBS rinex
    pub fn is_observation_rinex (&self) -> bool { self.header.rinex_type == Type::ObservationData }
    /// Returns true if this is a METEO rinex
    pub fn is_meteo_rinex (&self) -> bool { self.header.rinex_type == Type::MeteorologicalData }

    /// Builds a `Rinex` from given file.
    /// Input file must respect the whitespace specifications
    /// for the entire header section.   
    /// The header section must respect the labelization standard too.
    pub fn from_file (fp: &std::path::Path) -> Result<Rinex, RinexError> {
        let (header, body) = Rinex::split_rinex_content(fp)?;
        let header = header::RinexHeader::from_str(&header)?;
        let record : Option<record::Record> = match header.is_crinex() {
            false => Some(record::build_record(&header,&body)?),
            true => None,
        };
        Ok(Rinex { header, record })
    }
}

mod test {
    use super::*;
    #[test]
    /// Tests `Rinex` constructor against all known test resources
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
