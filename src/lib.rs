//! This package provides a set of tools to parse 
//! `RINEX` files.
//! 
//! Refer to README for example of use.  
//! Homepage: <https://github.com/gwbres/rinex>
mod meteo;
mod gnss_time;
mod navigation;
mod observation;

pub mod sv;
pub mod types;
pub mod epoch;
pub mod header;
pub mod record;
pub mod version;
pub mod hatanaka;
pub mod constellation;

use thiserror::Error;
use std::str::FromStr;
use std::io::Write;

#[macro_export]
/// Returns `true` if given `Rinex` line is a comment
macro_rules! is_comment {
    ($line: expr) => { $line.contains("COMMENT") };
}

/// `Rinex` describes a `RINEX` file
#[derive(Debug)]
pub struct Rinex {
    /// `header` field contains general information
    pub header: header::Header,
    /// `record` contains `RINEX` file body
    /// and is type and constellation dependent 
    pub record: record::Record,
}

impl Default for Rinex {
    /// Builds a default `RINEX`
    fn default() -> Rinex {
        Rinex {
            header: header::Header::default(),
            record: record::Record::default(), 
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
    TypeError(#[from] types::TypeError),
}

impl Rinex {
    /// Builds a new `RINEX` struct from given:
    pub fn new (header: header::Header, record: record::Record) -> Rinex {
        Rinex {
            header,
            record,
        }
    }

    /// splits file into two (header, body) contents
    fn split_body_header (fp: &std::path::Path) -> Result<(String, String), RinexError> {
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

    /// Retruns true if this is an NAV rinex
    pub fn is_navigation_rinex (&self) -> bool { self.header.rinex_type == types::Type::NavigationMessage }
    /// Retruns true if this is an OBS rinex
    pub fn is_observation_rinex (&self) -> bool { self.header.rinex_type == types::Type::ObservationData }
    /// Returns true if this is a METEO rinex
    pub fn is_meteo_rinex (&self) -> bool { self.header.rinex_type == types::Type::MeteorologicalData }

    /// Builds a `RINEX` from given file.
    /// Header section must respect labelization standards,   
    /// some are mandatory.   
    /// Parses record for supported `RINEX` types
    pub fn from_file (fp: &std::path::Path) -> Result<Rinex, RinexError> {
        let (header, body) = Rinex::split_body_header(fp)?;
        let header = header::Header::from_str(&header)?;
        let record = record::build_record(&header, &body)?;
        Ok(Rinex { 
            header,
            record,
        })
    }

    /// Writes self into given file.   
    /// Both header + record will strictly follow RINEX standards.   
    /// Record: supports all known `RINEX` types
    fn to_file (&self, path: &str) -> std::io::Result<()> {
        let mut writer = std::fs::File::create(path)?;
        write!(writer, "{}", self.header.to_string())?;
        self.record.to_file(&self.header, writer)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    /// Tests `Rinex` constructor against all known test resources
    fn test_rinex_constructor() {
        let data_dir = env!("CARGO_MANIFEST_DIR").to_owned() + "/data";
        let test_data = vec![
			"NAV",
			"OBS",
			"CRNX",
			"MET",
		];
        for data in test_data {
            let data_path = std::path::PathBuf::from(
                data_dir.to_owned() +"/" + data
            );
            for revision in std::fs::read_dir(data_path)
                .unwrap() {
                let rev = revision.unwrap();
                let rev_path = rev.path();
                let rev_fullpath = &rev_path.to_str().unwrap(); 
                for entry in std::fs::read_dir(rev_fullpath)
                    .unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    let full_path = &path.to_str().unwrap();
                    let is_hidden = entry
                        .file_name()
                        .to_str()
                        .unwrap()
                        .starts_with(".");
                    if !is_hidden {
                        println!("Parsing file: \"{}\"", full_path);
                        let fp = std::path::Path::new(&path);
                        let rinex = Rinex::from_file(&fp);
                        assert_eq!(rinex.is_err(), false); // 1st basic test
                        let rinex = rinex.unwrap();
                        println!("{:#?}", rinex.header);
                        match data {
                            "NAV" => {
                                // NAV files checks
                                assert_eq!(rinex.header.crinex.is_none(), true);
                                assert_eq!(rinex.is_navigation_rinex(), true);
                                assert_eq!(rinex.header.obs_codes.is_none(), true);
                                assert_eq!(rinex.header.met_codes.is_none(), true);
                                let record = rinex.record.as_nav().unwrap();
                                let mut epochs = record.keys();
                                println!("----- EPOCH #1 ----- \n{:#?}", record[epochs.nth(0).unwrap()]);
                            },
                            "OBS" => {
                                // OBS files checks
                                assert_eq!(rinex.header.crinex.is_none(), true);
                                assert_eq!(rinex.is_observation_rinex(), true);
                                assert_eq!(rinex.header.obs_codes.is_some(), true);
                                assert_eq!(rinex.header.met_codes.is_none(), true);
                                if rinex.header.rcvr_clock_offset_applied {
                                    // epochs should always have a RCVR clock offset
                                    // test that with iterator
                                }
                                let record = rinex.record.as_obs().unwrap();
                                let record = rinex.record.as_obs().unwrap();
                                let mut epochs = record.keys();
                                println!("----- EPOCH #1 ----- \n{:#?}", record[epochs.nth(0).unwrap()]);
                            },
                            "CRNX" => {
                                // compressed OBS files checks
                                assert_eq!(rinex.header.crinex.is_some(), true);
                                assert_eq!(rinex.is_observation_rinex(), true);
                                assert_eq!(rinex.header.obs_codes.is_some(), true);
                                assert_eq!(rinex.header.met_codes.is_none(), true);
                                let record = rinex.record.as_obs().unwrap();
                                let mut epochs = record.keys();
                                println!("----- EPOCH #1 ----- \n{:#?}", record[epochs.nth(0).unwrap()]);
                            },
							"MET" => {
                                // METEO files checks
                                assert_eq!(rinex.header.crinex.is_none(), true);
                                assert_eq!(rinex.is_meteo_rinex(), true);
                                assert_eq!(rinex.header.met_codes.is_some(), true);
                                assert_eq!(rinex.header.obs_codes.is_none(), true);
                                let record = rinex.record.as_meteo().unwrap();
                                let mut epochs = record.keys();
                                println!("----- EPOCH #1 ----- \n{:#?}", record[epochs.nth(0).unwrap()]);
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    use std::process::Command;
    /// Runs `diff` to determines whether f1 & f2 
    /// are strictly identical or not
    fn diff_is_strictly_identical (f1: &str, f2: &str) -> Result<bool, std::string::FromUtf8Error> {
        let output = Command::new("diff")
            .arg("-q")
            .arg("-Z")
            .arg(f1)
            .arg(f2)
            .output()
            .expect("failed to execute \"diff\"");
        let output = String::from_utf8(output.stdout)?;
        Ok(output.len()==0)
    }
    use std::collections::HashMap;
    #[test]
    /// Tests METEO `Rinex` (V2) production method 
    fn test_meteo_v2_rinex_production() {
        // test header
        let mut header = header::Header::default();
        header.version.major = 2;
        header.version.minor = 11;
        header.rinex_type = types::Type::MeteorologicalData;
        header.comments.push(String::from("Solaris x86 5.10|AMD64|cc SC5.8 -xarch=amd64|=+|=+"));
        header.station = String::from("ABVI");
        header.station_id = String::from(" "); 
        // sensors
        let mut sensors : Vec<header::Sensor> = vec![ 
            header::Sensor::new(" ", " ", 0.0, "PR"),
            header::Sensor::new(" ", " ", 0.0, "TD"),
            header::Sensor::new(" ", " ", 0.0, "HR"),
            header::Sensor::new(" ", " ", 0.0, "WS"),
            header::Sensor::new(" ", " ", 0.0, "WD"),
            header::Sensor::new(" ", " ", 0.0, "RI"),
            header::Sensor::new(" ", " ", 0.0, "HI"),
        ];
        header.sensors = Some(sensors);
        // OBS codes
        let met_codes = vec![
            String::from("PR"),
            String::from("TD"),
            String::from("HR"),
            String::from("WS"),
            String::from("WD"),
            String::from("RI"),
            String::from("HI"),
        ];
        header.met_codes = Some(met_codes);
        // RECORD 
        let mut record = meteo::Record::with_capacity(10);
        let epoch = epoch::Epoch::new(
            epoch::str2date("15  1  1  0  0  0").unwrap(),
            epoch::EpochFlag::Ok);
        let mut content : HashMap<String, f32> = HashMap::with_capacity(7);
        content.insert(String::from("PR"), 1018.6);
        content.insert(String::from("TD"), 25.6);
        content.insert(String::from("HR"), 78.9);
        content.insert(String::from("WS"), 3.1);
        content.insert(String::from("WD"), 10.0);
        content.insert(String::from("RI"), 0.0);
        content.insert(String::from("HI"), 0.0);
        record.insert(epoch, content);
        let record = record::Record::MeteoRecord(record);
        let rinex = Rinex::new(header, record);
        rinex.to_file("test").unwrap();
        //let identical = diff_is_strictly_identical("test", "data/MET/V2/abvi0010.15m").unwrap();
        //assert_eq!(identical, true)
    }
}
