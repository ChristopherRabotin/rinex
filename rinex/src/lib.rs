//! This library provides a set of tools to parse, analyze,
//! produce and manipulate `RINEX` files.  
//! Refer to README and official documentation, extensive examples of use
//! are provided.  
//! Homepage: <https://github.com/gwbres/rinex>
mod leap;
mod merge;
mod clocks;
//mod gnss_time;

pub mod sv;
pub mod antex;
pub mod meteo;
pub mod types;
pub mod epoch;
pub mod header;
pub mod record;
pub mod hardware;
//pub mod carrier;
pub mod version;
pub mod hatanaka;
pub mod constellation;
pub mod navigation;
pub mod observation;

use std::io::Write;
use thiserror::Error;
use std::str::FromStr;
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};
use chrono::{Datelike, Timelike};

#[cfg(feature = "with-serde")]
#[macro_use]
extern crate serde;

#[macro_export]
/// Returns `true` if given `Rinex` line is a comment
macro_rules! is_comment {
    ($line: expr) => { $line.contains("COMMENT") };
}

#[macro_export]
/// Returns True if 3 letter code 
/// matches a pseudo range (OBS) code
macro_rules! is_pseudo_range_obs_code {
    ($code: expr) => { 
        $code.starts_with("C") // standard 
        || $code.starts_with("P") // non gps old fashion
    };
}

#[macro_export]
/// Returns True if 3 letter code 
/// matches a phase (OBS) code
macro_rules! is_phase_carrier_obs_code {
    ($code: expr) => { $code.starts_with("L") };
}

#[macro_export]
/// Returns True if 3 letter code 
/// matches a doppler (OBS) code
macro_rules! is_doppler_obs_code {
    ($code: expr) => { $code.starts_with("D") };
}

#[macro_export]
/// Returns True if 3 letter code 
/// matches a signal strength (OBS) code
macro_rules! is_sig_strength_obs_code {
    ($code: expr) => { $code.starts_with("S") };
}

#[macro_export]
/// Returns True if 3 letter code
/// matches a temperature observation code
macro_rules! is_temperature_obs_code {
	($code: expr) => {
		$code.eq("TD")
	};
}

#[macro_export]
/// Returns True if 3 letter code
/// matches a pressure observation code
macro_rules! is_pressure_obs_code {
	($code: expr) => {
		$code.eq("PR")
	};
}

#[macro_export]
/// Returns True if 3 letter code
/// matches a moisture / humidity rate observation code
macro_rules! is_humidity_obs_code {
	($code: expr) => {
		$code.eq("HR")
	};
}

#[macro_export]
/// Returns True if 3 letter code
/// matches a wet zenith path delay obs code 
macro_rules! is_wet_zenith_code {
	($code: expr) => {
		$code.eq("ZW")
	};
}

#[macro_export]
/// Returns True if 3 letter code
/// matches a dry zenith path delay obs code 
macro_rules! is_dry_zenith_code {
	($code: expr) => {
		$code.eq("ZD")
	};
}

#[macro_export]
/// Returns True if 3 letter code
/// matches a wind speed obs code 
macro_rules! is_wind_speed_code {
	($code: expr) => {
		$code.eq("WS")
	};
}

#[macro_export]
/// Returns True if 3 letter code
/// matches a rain increment obs code 
macro_rules! is_rain_increment_code {
	($code: expr) => {
		$code.eq("RI")
	};
}

#[macro_export]
/// Returns True if 3 letter code
/// matches a rain increment obs code 
macro_rules! is_hail_indicator_code {
	($code: expr) => {
		$code.eq("HI")
	};
}

/// Returns `str` description, as one letter
/// lowercase, used in RINEX file name to describe 
/// the sampling period. RINEX specifications:   
/// “a” = 00:00:00 - 00:59:59   
/// “b” = 01:00:00 - 01:59:59   
/// [...]   
/// "x" = 23:00:00 - 23:59:59
/// This method expects a chrono::NaiveDateTime as an input
fn hourly_session_str (time: chrono::NaiveTime) -> String {
    let h = time.hour() as u8;
    if h == 23 {
        String::from("x")
    } else {
        let c : char = (h+97).into();
        String::from(c)
    }
}

/// `Rinex` describes a `RINEX` file
#[derive(Clone, Debug)]
pub struct Rinex {
    /// `header` field contains general information
    pub header: header::Header,
    /// `comments` : list of extra readable information,   
    /// found in `record` section exclusively.    
    /// Comments extracted from `header` sections are exposed in `header.comments`
    comments: record::Comments, 
    /// `record` contains `RINEX` file body
    /// and is type and constellation dependent 
    pub record: record::Record,
}

impl Default for Rinex {
    /// Builds a default `RINEX`
    fn default() -> Rinex {
        Rinex {
            header: header::Header::default(),
            comments: record::Comments::new(), 
            record: record::Record::default(), 
        }
    }
}

#[derive(Error, Debug)]
/// `RINEX` Parsing related errors
pub enum Error {
    #[error("header parsing error")]
    HeaderError(#[from] header::Error),
    #[error("record parsing error")]
    RecordError(#[from] record::Error),
}

#[derive(Error, Debug)]
/// `Split` ops related errors
pub enum SplitError {
    #[error("desired epoch is too early")]
    EpochTooEarly,
    #[error("desired epoch is too late")]
    EpochTooLate,
}

impl Rinex {
    /// Builds a new `RINEX` struct from given header & body sections
    pub fn new (header: header::Header, record: record::Record) -> Rinex {
        Rinex {
            header,
            record,
            comments: record::Comments::new(),
        }
    }

    pub fn with_header (&self, header: header::Header) -> Self {
        Rinex {
            header,
            record: self.record.clone(),
            comments: self.comments.clone(),
        }
    }

    /// Filename creation helper,
    /// to match standard specifications.
    pub fn filename (&self) -> String {
        let header = &self.header;
        let rtype = header.rinex_type;
        let nnnn = header.station.as_str()[0..4].to_lowercase(); 
        //TODO:
        //self.header.date should be a datetime object
        //but it is complex to parse..
        let ddd = String::from("DDD"); 
        let epoch : epoch::Epoch = match rtype {
            types::Type::ObservationData => {
                let e : Vec<&epoch::Epoch> = self.record.as_obs()
                    .unwrap()
                    .keys()
                    .collect();
                *e[0]
            },
            types::Type::NavigationData => {
                let e : Vec<&epoch::Epoch> = self.record.as_nav()
                    .unwrap()
                    .keys()
                    .collect();
                *e[0]
            },
            types::Type::MeteoData => {
                let e : Vec<&epoch::Epoch> = self.record.as_meteo()
                    .unwrap()
                    .keys()
                    .collect();
                *e[0]
            },
            types::Type::AntennaData => {
                epoch::Epoch::default()
            },
        };
        if header.version.major < 3 {
            let s = hourly_session_str(epoch.date.time());
            let yy = format!("{:02}", epoch.date.year());
            let t : String = match rtype {
                types::Type::ObservationData => {
                    if header.is_crinex() {
                        String::from("d")
                    } else {
                        String::from("o")
                    }
                },
                types::Type::NavigationData => {
                    if let Some(c) = header.constellation {
                        if c == constellation::Constellation::Glonass {
                            String::from("g")
                        } else { 
                            String::from("n")
                        }
                    } else {
                        String::from("x")
                    }
                },
                types::Type::MeteoData => String::from("m"),
                types::Type::AntennaData => String::from("?"), //TODO
            };
            format!("{}{}{}.{}{}", nnnn, ddd, s, yy, t)
        } else {
            let m = String::from("0");
            let r = String::from("0");
            //TODO: 3 letter contry code, example: "GBR"
            let ccc = String::from("CCC");
            //TODO: data source
            // R: Receiver (hw)
            // S: Stream
            // U: Unknown
            let s = String::from("R");
            let yyyy = format!("{:04}", epoch.date.year());
            let hh = format!("{:02}", epoch.date.hour());
            let mm = format!("{:02}", epoch.date.minute());
            let pp = String::from("00"); //TODO 02d file period, interval ?
            let up = String::from("H"); //TODO: file period unit
            let ff = String::from("00"); //TODO: 02d observation frequency 02d
            //TODO
            //Units of frequency FF. “C” = 100Hz; “Z” = Hz; “S” = sec; “M” = min;
            //“H” = hour; “D” = day; “U” = unspecified
            //NB - _FFU is omitted for files containing navigation data
            let uf = String::from("Z");
            let c : String = match header.constellation {
                Some(c) => c.to_1_letter_code().to_uppercase(),
                _ => String::from("X"),
            };
            let t : String = match rtype {
                types::Type::ObservationData => String::from("O"),
                types::Type::NavigationData => String::from("N"),
                types::Type::MeteoData => String::from("M"),
                types::Type::AntennaData => String::from("?"), //TODO
            };
            let fmt = match header.is_crinex() {
                true => String::from("crx"),
                false => String::from("rnx"),
            };
            format!("{}{}{}{}_{}_{}{}{}{}_{}{}_{}{}_{}{}.{}",
                nnnn, m, r, ccc, s, yyyy, ddd, hh, mm, pp, up, ff, uf, c, t, fmt)
        }
    }

    /// Builds a `RINEX` from given file.
    /// Header section must respect labelization standards,   
    /// some are mandatory.   
    /// Parses record for supported `RINEX` types
    pub fn from_file (path: &str) -> Result<Rinex, Error> {
        let header = header::Header::new(path)?;
        let (record, comments) = record::build_record(path, &header)?;
        Ok(Rinex {
            header,
            record,
            comments,
        })
    }

    /// Retruns true if this is an NAV rinex
    pub fn is_navigation_rinex (&self) -> bool { self.header.rinex_type == types::Type::NavigationData }
    /// Retruns true if this is an OBS rinex
    pub fn is_observation_rinex (&self) -> bool { self.header.rinex_type == types::Type::ObservationData }
    /// Returns true if this is a METEO rinex
    pub fn is_meteo_rinex (&self) -> bool { self.header.rinex_type == types::Type::MeteoData }
    // Returns true if this is a CLOCK rinex
    //pub fn is_clock_rinex (&self) -> bool { self.header.rinex_type == types::Type::ClockData }

    /// Returns `epoch` (sampling timestamp) of first observation
    pub fn first_epoch (&self) -> Option<epoch::Epoch> {
        let epochs = self.epochs_iter();
        if epochs.len() == 0 {
            None
        } else {
            Some(*epochs[0])
        }
    }

    /// Returns `epoch` (sampling timestamp) of last observation
    pub fn last_epoch (&self) -> Option<epoch::Epoch> {
        let epochs = self.epochs_iter();
        if epochs.len() == 0 {
            None
        } else {
            Some(*epochs[epochs.len()-1])
        }
    }

    /// Returns sampling interval for `record`:   
    /// + either directly from `header` if such information was provided
    /// + or by computing the average time interval between two successive epochs.    
    ///   Only `valid` epochs contribute to the calculation in this case.   
    ///   Returns None, in case record contains a unique epoch and calculation is not feasible.
    pub fn sampling_interval (&self) -> Option<std::time::Duration> {
        if let Some(interval) = self.header.sampling_interval {
            Some(std::time::Duration::from_secs(interval as u64))
        } else {
            // build epoch interval histogram 
            let mut histogram : HashMap<i64, u64> = HashMap::new(); // {internval, population}
            let epochs = self.epochs_iter();
            for i in 0..epochs.len()-1 {
                let e_i = epochs.get(i).unwrap();
                if e_i.flag.is_ok() {
                    if let Some(e) = epochs.get(i+1) {
                        if e.flag.is_ok() {
                            // delta(i+1, i) --> positive deltas
                            let delta = (e.date - epochs.get(i).unwrap().date).num_seconds();
                            if histogram.contains_key(&delta) {
                                let prev = histogram.get(&delta).unwrap();
                                histogram.insert(delta, *prev +1); // overwrite 
                            } else {
                                histogram.insert(delta, 1); // new entry
                            }
                        }
                    }
                }
            }
            let mut sorted = histogram
                .iter()
                .sorted_by(|a,b| b.cmp(a));
            //println!("Histogram sorted by Population: {:#?}", sorted); 
            if let Some(largest) = sorted.nth(0) { // largest population found
                Some(std::time::Duration::from_secs(*largest.0 as u64))
            } else { // histogram empty -> weird case(s)
                // record is either empty
                // or contained a unique epoch
                // --> calculation was not feasible
                None 
            }
        }
    }

    /// Returns a list of epochs that present an data gap (dead time: time without data in the record).   
    /// This is determined by comparing the time difference between adjacent epochs to the nominal sampling interval.   
    /// Only `valid` epochs are taken into account in these calculations.    
    /// Granularity is 1 second.
    pub fn dead_times (&self) -> Vec<epoch::Epoch> {
        let sampling_interval = self.sampling_interval();
        let mut result : Vec<epoch::Epoch> = Vec::new();
        let epochs = self.epochs_iter();
        if let Some(interval) = sampling_interval { // got a value to compare to
            for i in 0..epochs.len()-1 {
                let e_i = epochs.get(i).unwrap();
                if e_i.flag.is_ok() {
                    if let Some(e) = epochs.get(i+1) {
                        if e.flag.is_ok() {
                            let delta = (e.date - e_i.date).num_seconds() as u64;
                            if delta > interval.as_secs() {
                                result.push(**e)
                            }
                        }
                    }
                }
            }
        }
        result
    }
    
    /// Returns list of epochs where unusual events happen,    
    /// ie., epochs with an != Ok flag attached to them.    
    /// Use `mask` to provide a specific epoch events filter.    
    /// This method is very useful to determine when special/external events happened
    /// in this `record`, events such as:
    ///   + power cycle failures
    ///   + receiver physically moved (new site occupation)
    ///   + other external events 
    pub fn epoch_anomalies (&self, mask: Option<epoch::EpochFlag>) -> Vec<&epoch::Epoch> { 
        let epochs = self.epochs_iter();
        epochs
            .into_iter()
            .filter(|e| {
                let mut nok = !e.flag.is_ok(); // abnormal epoch
                if let Some(mask) = mask {
                    nok &= e.flag == mask // + match specific event mask
                }
                nok
            })
            .collect()
    }

    /// Returns (if possible) event explanation / description by searching through identified comments,
    /// and returning closest comment (inside record) in time.    
    /// Usually, comments are associated to epoch events (anomalies) to describe what happened.   
    /// This method tries to locate a list of comments that were associated to the given timestamp 
    pub fn event_description (&self, event: epoch::Epoch) -> Option<&str> {
        let comments : Vec<_> = self.comments
            .iter()
            .filter(|(k,_)| *k == &event)
            .map(|(_,v)| v)
            .flatten()
            .collect();
        if comments.len() > 0 {
            Some(comments[0]) // TODO grab all content! by serializing into a single string
        } else {
            None
        }
    } 

    /// Returns `true` if self is a `merged` RINEX file,   
    /// meaning, this file is the combination of two RINEX files merged together.  
    /// This is determined by the presence of a custom yet somewhat standardized `FILE MERGE` comments
    pub fn is_merged (&self) -> bool {
        for (_, content) in self.comments.iter() {
            for c in content {
                if c.contains("FILE MERGE") {
                    return true
                }
            }
        }
        false
    }

    /// Returns list of epochs where RINEX merging operation(s) occurred.    
    /// Epochs are determined either by the pseudo standard `FILE MERGE` comment description,
    /// or by comment epochs inside the record
    pub fn merge_boundaries (&self) -> Vec<chrono::NaiveDateTime> {
        self.header.comments
            .iter()
            .flat_map(|s| {
                if s.contains("FILE MERGE") {
                    let content = s.split_at(40).1.trim();
                    if let Ok(date) = chrono::NaiveDateTime::parse_from_str(content, "%Y%m%d %h%m%s UTC") {
                        Some(date)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Splits merged `records` into seperate `records`.
    /// Returns empty list if self is not a `Merged` file
    pub fn split_record (&self) -> Vec<record::Record> {
        let boundaries = self.merge_boundaries();
        let mut result : Vec<record::Record> = Vec::with_capacity(boundaries.len());
        let epochs = self.epochs_iter();
        let mut t_0 = epochs[0].date;
        for boundary in boundaries {
            let _included : Vec<_> = epochs
                .iter()
                .filter(|e| e.date >= t_0 && e.date < boundary)
                .collect();
            let rec = match self.header.rinex_type {
                types::Type::NavigationData => {
                    let rec : BTreeMap<_, _> = self.record.as_nav().unwrap()
                        .iter()
                        .filter(|(k, _)| k.date >= t_0 && k.date < boundary)
                        .map(|(k, v)| (k.clone(),v.clone()))
                        .collect();
                    record::Record::NavRecord(rec)
                },
                types::Type::ObservationData => {
                    let rec : BTreeMap<_, _> = self.record.as_obs().unwrap()
                        .iter()
                        .filter(|(k, _)| k.date >= t_0 && k.date < boundary)
                        .map(|(k, v)| (k.clone(),v.clone()))
                        .collect();
                    record::Record::ObsRecord(rec)
                },
                types::Type::MeteoData => {
                    let rec : BTreeMap<_, _> = self.record.as_meteo().unwrap()
                        .iter()
                        .filter(|(k, _)| k.date >= t_0 && k.date < boundary)
                        .map(|(k, v)| (k.clone(),v.clone()))
                        .collect();
                    record::Record::MeteoRecord(rec)
                },
                _ => unreachable!("epochs::iter()"),
            };
            result.push(rec);
            t_0 = boundary 
        }
        result
    }

    /// Splits self into two seperate records, at desired `epoch`.
    /// Self does not have to be a `Merged` file
    pub fn split_record_at_epoch (&self, epoch: epoch::Epoch) -> Result<(record::Record,record::Record), SplitError> {
        let epochs = self.epochs_iter();
        if epoch.date < epochs[0].date {
            return Err(SplitError::EpochTooEarly)
        }
        if epoch.date > epochs[epochs.len()-1].date {
            return Err(SplitError::EpochTooLate)
        }
        let rec0 : record::Record = match self.header.rinex_type {
            types::Type::NavigationData => {
                let rec = self.record.as_nav()
                    .unwrap()
                        .iter()
                        .flat_map(|(k, v)| {
                            if k.date < epoch.date {
                                Some((k, v))
                            } else {
                                None
                            }
                        })
                        .map(|(k,v)| (k.clone(),v.clone())) // BTmap collect() derefencing 
                        .collect();
                record::Record::NavRecord(rec)
            },
            types::Type::ObservationData => {
                let rec = self.record.as_obs()
                    .unwrap()
                        .iter()
                        .flat_map(|(k, v)| {
                            if k.date < epoch.date {
                                Some((k, v))
                            } else {
                                None
                            }
                        })
                        .map(|(k,v)| (k.clone(),v.clone())) // BTmap collect() derefencing 
                        .collect();
                record::Record::ObsRecord(rec)
            },
            types::Type::MeteoData => {
                let rec = self.record.as_meteo()
                    .unwrap()
                        .iter()
                        .flat_map(|(k, v)| {
                            if k.date < epoch.date {
                                Some((k, v))
                            } else {
                                None
                            }
                        })
                        .map(|(k,v)| (k.clone(),v.clone())) // BTmap collect() derefencing 
                        .collect();
                record::Record::MeteoRecord(rec)
            },
            _ => unreachable!("epochs::iter()"),
        };
        let rec1 : record::Record = match self.header.rinex_type {
            types::Type::NavigationData => {
                let rec = self.record.as_nav()
                    .unwrap()
                        .iter()
                        .flat_map(|(k, v)| {
                            if k.date >= epoch.date {
                                Some((k, v))
                            } else {
                                None
                            }
                        })
                        .map(|(k,v)| (k.clone(),v.clone())) // BTmap collect() derefencing 
                        .collect();
                record::Record::NavRecord(rec)
            },
            types::Type::ObservationData => {
                let rec = self.record.as_obs()
                    .unwrap()
                        .iter()
                        .flat_map(|(k, v)| {
                            if k.date >= epoch.date {
                                Some((k, v))
                            } else {
                                None
                            }
                        })
                        .map(|(k,v)| (k.clone(),v.clone())) // BTmap collect() derefencing 
                        .collect();
                record::Record::ObsRecord(rec)
            },
            types::Type::MeteoData => {
                let rec = self.record.as_meteo()
                    .unwrap()
                        .iter()
                        .flat_map(|(k, v)| {
                            if k.date >= epoch.date {
                                Some((k, v))
                            } else {
                                None
                            }
                        })
                        .map(|(k,v)| (k.clone(),v.clone())) // BTmap collect() derefencing 
                        .collect();
                record::Record::MeteoRecord(rec)
            },
            _ => unreachable!("epochs::iter()"),
        };
        Ok((rec0,rec1))
    }

    /// Returns epoch iterator for this `RINEX`.
    /// Faillible! if this RINEX is not sorted by `epochs`
    pub fn epochs_iter(&self) -> Vec<&epoch::Epoch> {
        match self.header.rinex_type {
            types::Type::ObservationData => {
                self.record
                    .as_obs()
                    .unwrap()
                    .keys()
                    .collect()
            },
            types::Type::NavigationData => {
                self.record
                    .as_nav()
                    .unwrap()
                    .keys()
                    .collect()
            },
            types::Type::MeteoData => {
                self.record
                    .as_meteo()
                    .unwrap()
                    .keys()
                    .collect()
            },
            _ => panic!("Cannot get an epoch iterator for \"{:?}\"", self.header.rinex_type),
        }
    }

    /// Splits Self into separate `RINEX` structures. 
    /// Header sections are simply copied.
    /// Splits at given epoch (if provided).
    /// Splits at Merging boundary if Self is a valid `Merged` RINEX.
    /// Will panic if no epochs are specified and Self is not a `Merged` RINEX.
    pub fn split (&self, epoch: Option<epoch::Epoch>) -> Result<Vec<Self>, SplitError> {
        if let Some(epoch) = epoch {
            let (r1, r2) = self.split_record_at_epoch(epoch)?;
            let mut result : Vec<Self> = Vec::with_capacity(2);
            result.push(
                Self {
                    header: self.header.clone(),
                    comments: self.comments.clone(),
                    record: r1.clone(),
                });
            result.push(
                Self {
                    header: self.header.clone(),
                    comments: self.comments.clone(),
                    record: r2.clone(),
                });
            Ok(result)
        } else {
            if self.is_merged() {
                let records = self.split_record();
                let mut result : Vec<Self> = Vec::with_capacity(records.len());
                for rec in records {
                    result.push(Self {
                        header: self.header.clone(),
                        comments: self.comments.clone(),
                        record: rec.clone(),
                    })
                }
                Ok(result)
            } else {
                panic!("This is not a merged RINEX")
            }
        }
    }

    /// Merges given RINEX into self, in teqc similar fashion.   
    /// Header sections are combined (refer to header::merge Doc
    /// to understand its behavior).
    /// Resulting self.record (modified in place) remains sorted by 
    /// sampling timestamps.
    pub fn merge (&mut self, other: &Self) -> Result<(), merge::MergeError> {
        self.header.merge(&other.header)?;
        // grab Self:: + Other:: `epochs`
        let (epochs, other_epochs) = (self.epochs_iter(), other.epochs_iter());
        if epochs.len() == 0 { // self is empty
            self.record = other.record.clone();
            Ok(()) // --> self is overwritten
        } else if other_epochs.len() == 0 { // nothing to merge
            Ok(()) // --> self is untouched
        } else {
            // add Merge op descriptor
            let now = chrono::offset::Utc::now();
            self.header.comments.push(format!(
                "rustrnx-{:<20} FILE MERGE          {} UTC", 
                env!("CARGO_PKG_VERSION"),
                now.format("%Y%m%d %H%M%S")));
            // merge op
            match self.header.rinex_type {
                types::Type::NavigationData => {
                    let a_rec = self.record
                        .as_mut_nav()
                        .unwrap();
                    let b_rec = other.record
                        .as_nav()
                        .unwrap();
                    for (k, v) in b_rec {
                        a_rec.insert(*k, v.clone());
                    }
                },
                types::Type::ObservationData => {
                    let a_rec = self.record
                        .as_mut_obs()
                        .unwrap();
                    let b_rec = other.record
                        .as_obs()
                        .unwrap();
                    for (k, v) in b_rec {
                        a_rec.insert(*k, v.clone());
                    }
                },
                types::Type::MeteoData => {
                    let a_rec = self.record
                        .as_mut_meteo()
                        .unwrap();
                    let b_rec = other.record
                        .as_meteo()
                        .unwrap();
                    for (k, v) in b_rec {
                        a_rec.insert(*k, v.clone());
                    }
                },
                _ => unreachable!("epochs::iter()"),
            }
            Ok(())
        }
    }
    
    /// ''cleans up'' self: removes `invalid` epochs
    /// which do not have an Epoch::Ok flag associated to them.
    /// It is only relevant to call this method on Observation Data
    pub fn cleanup (&mut self) {
        let hd = &self.header;
        if hd.rinex_type != types::Type::ObservationData {
            return;
        }
        let rec : Vec<_> = self.record
            .as_obs()
            .unwrap()
            .iter()
            .collect();
        let mut rework = observation::Record::new();  
        for (e, data) in rec {
            if e.flag.is_ok() {
                rework.insert(*e, data.clone());
            }
        }
        self.record = record::Record::ObsRecord(rework)
    }
    
    /// Returns lock loss event timestamps from this Observation Data Record.
    /// Convenient macro to determine timestamp where `lock` was lost
    /// during this acquisition.
    /// Calling this macro on another type of record will panic.
    pub fn lock_loss_events (&self) -> Vec<epoch::Epoch> {
        let mut result : Vec<epoch::Epoch> = Vec::new();
        let rec : Vec<_> = self.record
            .as_obs()
            .unwrap()
            .iter()
            .collect();
        for (e, (_, sv)) in rec {
            for (_, obs) in sv {
                for (_, data) in obs {
                    let flag = data.lli.unwrap_or(observation::lli_flags::OK_OR_UNKNOWN);
                    if flag == observation::lli_flags::LOCK_LOSS {
                        result.push(*e)
                    }
                }
            }
        }
        result
    }

    /// Resamples self to desired sampling interval
    /// interval: desired new sampling interval (1/data rate)
    /// filter: optionnal lamba / function pointer
    /// for user to compensate that resampling operation
    /// and filter data accordingly. Filter lambda should be
    /// a standard convolution operation, the only requirement is
    /// the lambda must work on a f64 and return a f64
    /// signature : lambda(f64) -> f64.
    /// Also note that self.header.sampling_interval field is adaptated to
    /// the operation we just performed.   
    /// if interval < self.sampling_interval   
    ///   + upsampling:
    ///     when no filter operation is specified,
    ///     we simply copy the previous data to increase the sample rate
    /// else (interval >= self.sampling_interval): 
    ///   + downsampling
    /// To learn how to operate this method correctly, (mainly
    /// declare lambda correctly),
    /// refer to the provided example/resampling.rs
    pub fn resample (&mut self, interval: std::time::Duration) {
        if let Some(sampling) = self.sampling_interval() {
            let record : record::Record = match interval < sampling {
                true => {
                    // upsampling
                    record::Record::ObsRecord(observation::Record::new())
                },
                false => {
                    // downsampling
                    let interval = chrono::Duration::from_std(interval).unwrap();
                    let epochs = self.epochs_iter();
                    let nav_record = self.record.as_nav();
                    let obs_record = self.record.as_obs();
                    let met_record = self.record.as_meteo();
                    let mut met_result = meteo::Record::new();
                    let mut nav_result = navigation::Record::new();
                    let mut obs_result = observation::Record::new();
                    let mut curr = epochs[0]; 
                    let mut i : usize = 1;
                    match self.header.rinex_type {
                        types::Type::NavigationData => { 
                            nav_result.insert(
                                *curr, 
                                nav_record 
                                    .unwrap()
                                    .get(curr)
                                    .unwrap()
                                    .clone()
                            );
                        },
                        types::Type::ObservationData => { 
                            obs_result.insert(
                                *curr, 
                                obs_record
                                    .unwrap()
                                    .get(curr)
                                    .unwrap()
                                    .clone()
                            );
                        },
                        types::Type::MeteoData => { 
                            met_result.insert(
                                *curr, 
                                met_record
                                    .unwrap()
                                    .get(curr)
                                    .unwrap()
                                    .clone()
                            );
                        },
                        _ => {}, // N/A
                    }
                    loop {
                        if i == epochs.len() {
                            break
                        }
                        if epochs[i].date - curr.date >= interval {
                            match self.header.rinex_type {
                                types::Type::NavigationData => { 
                                    nav_result.insert(
                                        *epochs[i], 
                                        nav_record
                                            .unwrap()
                                            .get(epochs[i])
                                            .unwrap()
                                            .clone()
                                    );
                                },
                                types::Type::MeteoData => { 
                                    met_result.insert(
                                        *epochs[i], 
                                        met_record
                                            .unwrap()
                                            .get(epochs[i])
                                            .unwrap()
                                            .clone()
                                    );
                                },
                                types::Type::ObservationData => {
                                    obs_result.insert(
                                        *epochs[i], 
                                        obs_record
                                            .unwrap()
                                            .get(epochs[i])
                                            .unwrap()
                                            .clone()
                                    );
                                },
                                _ => {}, // N/A
                            }
                            curr = epochs[i]
                        }
                        i += 1
                    }
                    match self.header.rinex_type {
                        types::Type::NavigationData => record::Record::NavRecord(nav_result),
                        types::Type::ObservationData => record::Record::ObsRecord(obs_result),
                        types::Type::MeteoData => record::Record::MeteoRecord(met_result),
                        _ => panic!("operation not available for this RINEX type"), // N/A
                    }
                },
            };
            self.record = record
        }
    }

    /// Convenient macro to decimate self.record by given sampling factor
    /// (user does not have to care for samplingInterval())
    pub fn decimate (&mut self, factor : u32) {
        if let Some(interval) = self.sampling_interval() {
            self.resample(interval / factor)
        }
    }

    /// Convenient macro to interpolate (upsample) self.record
    /// by given sampling factor
    /// (user does not have to care for samplingInterval())
    pub fn interpolate (&mut self, factor : u32) {
        if let Some(interval) = self.sampling_interval() {
            self.resample(interval * factor)
        }
    }
/*
    /// Corrects all so called `cycle slip` events
    /// in this Observation record. 
    /// Calling this macro on another type of record will panic.
    /// Cycle Slip event is compensated by correcting raw phase data
    /// by the current wavelength factor
    pub fn cycle_slip_correction (&mut self) {
        // [1] grab initial wavelength factor
        let w_0 = self.header.wavelengths.unwrap_or((0,0));
        // [2] iterate through the record
        //     compensate
    }
*/
/*
    /// Convenient filter to only retain data
    /// from this Observation Record that match a given signal quality.
    /// Calling this macro on another type of record will panic.
    /// You have 3 filter choices:
    ///   + 'weak' will accept data with SNR >= 30 dB 
    ///   + 'strong' will accept data with SNR >= 35 dB
    ///   + 'excellent' will accept data with SNR >= 42 dB
    pub fn signal_quality_filter (&mut self, filter: &str) {
        self.record
            .as_mut_obs()
            .unwrap()
            .iter()
            .find(|k, v|) {
                v.({ 

                })
            })
    } 
*/
    
    /// Writes self into given file.   
    /// Both header + record will strictly follow RINEX standards.   
    /// Record: supports all known `RINEX` types
    pub fn to_file (&self, path: &str) -> std::io::Result<()> {
        let mut writer = std::fs::File::create(path)?;
        write!(writer, "{}", self.header.to_string())?;
        self.record.to_file(&self.header, writer)
    }
}

#[cfg(test)]
mod test {
    use super::*;
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
    /*#[test]
    /// Tests record `Decimate()` ops 
    fn test_record_decimation() {
        let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/NAV/V3/AMEL00NLD_R_20210010000_01D_MN.rnx";
        let mut rinex = Rinex::from_file(&path).unwrap();
        let original : Vec<&epoch::Epoch> = rinex.record.as_nav().unwrap().keys().collect();
        println!("LEN {}", original.len());
        
        rinex.resample(std::time::Duration::from_secs(1));
        rinex.resample(std::time::Duration::from_secs(10*60));
    }*/
    #[test]
    /// Tests `Merge()` ops
    fn test_merge_type_mismatch() {
        let test_resources = env!("CARGO_MANIFEST_DIR").to_owned() + "/../test_resources/";
        let path1 = test_resources.to_owned() + "NAV/V3/AMEL00NLD_R_20210010000_01D_MN.rnx";
        let mut r1 = Rinex::from_file(&path1).unwrap();
        let path2 = test_resources.to_owned() + "OBS/V3/LARM0630.22O";
        let r2 = Rinex::from_file(&path2).unwrap();
        assert_eq!(r1.merge(&r2).is_err(), true)
    }
    /*#[test]
    /// Tests `Merge()` ops
    fn test_merge_rev_mismatch() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let path1 = manifest.to_owned() + "/data/NAV/V3/AMEL00NLD_R_20210010000_01D_MN.rnx";
        let mut r1 = Rinex::from_file(&path1).unwrap();
        let path2 = manifest.to_owned() + "/data/NAV/V2/amel0010.21g";
        let r2 = Rinex::from_file(&path2).unwrap();
        assert_eq!(r1.merge(&r2).is_err(), true)
    }*/
    /// Tests `Merge()` ops
    fn test_merge_basic() {
        let test_resources = env!("CARGO_MANIFEST_DIR").to_owned() + "/../test_resources/";
        let path1 = test_resources.to_owned() + "NAV/V3/AMEL00NLD_R_20210010000_01D_MN.rnx";
        let mut r1 = Rinex::from_file(&path1).unwrap();
        let path2 = test_resources.to_owned() + "NAV/V3/CBW100NLD_R_20210010000_01D_MN.rnx";
        let r2 = Rinex::from_file(&path2).unwrap();
        assert_eq!(r1.merge(&r2).is_ok(), true)
        //println!("is merged          : {}", rinex.is_merged_rinex());
        //println!("boundaries: \n{:#?}", rinex.merge_boundaries());
    }
    #[test]
    /// Tests `Rinex` constructor against all known test resources
    fn test_parser() {
        let test_resources = env!("CARGO_MANIFEST_DIR").to_owned() + "/../test_resources/";
        let test_data = vec![
			"NAV",
			"OBS",
			"CRNX",
			"MET",
		];
        for data in test_data {
            let data_path = std::path::PathBuf::from(
                test_resources.to_owned() + data
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
                    let is_test_file = !entry
                        .file_name()
                        .to_str()
                        .unwrap()
                        .ends_with("-copy");
                    if !is_hidden && is_test_file {
                        // PARSER
                        println!("Parsing file: \"{}\"", full_path);
                        let rinex = Rinex::from_file(full_path);
                        assert_eq!(rinex.is_err(), false); // 1st basic test
                        // HEADER
                        let rinex = rinex.unwrap();
                        println!("{:#?}", rinex.header);
                        // RECORD
                        match data {
                            "NAV" => {
                                // NAV files checks
                                assert_eq!(rinex.header.obs.is_none(), true);
                                assert_eq!(rinex.is_navigation_rinex(), true);
                                assert_eq!(rinex.header.meteo.is_none(), true);
                                let record = rinex.record.as_nav().unwrap();
                                println!("----- EPOCHs ----- \n{:#?}", record.keys());
                                let mut epochs = record.keys();
                                // Testing event description finder
                                if let Some(event) = epochs.nth(0) {
                                    // [!] with dummy t0 = 1st epoch timestamp
                                    //     this will actually return `header section` timestamps
                                    println!("EVENT @ {:#?} - description: {:#?}", event, rinex.event_description(*event)); 
                                }
                            },
                            "OBS" => {
                                // OBS files checks
                                let obs = rinex.header.obs.as_ref();
                                assert_eq!(obs.is_some(), true);
                                assert_eq!(rinex.is_observation_rinex(), true);
                                assert_eq!(rinex.header.meteo.is_none(), true);
                                let obs = obs.unwrap();
                                if obs.clock_offset_applied {
                                    // epochs should always have a RCVR clock offset
                                    // test that with iterator
                                }
                                let record = rinex.record
                                    .as_obs()
                                    .unwrap();
                                let mut epochs = record.keys();
                                println!("----- EPOCHs ----- \n{:#?}", record.keys());
                                // Testing event description finder
                                if let Some(event) = epochs.nth(0) {
                                    // [!] with dummy t0 = 1st epoch timestamp
                                    //     this will actually return `header section` timestamps
                                    println!("EVENT @ {:#?} - description: {:#?}", event, rinex.event_description(*event)); 
                                }
                            },
                            "CRNX" => {
                                // compressed OBS files checks
                                assert_eq!(rinex.header.obs.is_some(), true);
                                assert_eq!(rinex.is_observation_rinex(), true);
                                assert_eq!(rinex.header.meteo.is_none(), true);
                                let record = rinex.record.as_obs().unwrap();
                                let mut epochs = record.keys();
                                println!("----- EPOCHs ----- \n{:#?}", epochs); 
                                // Testing event description finder
                                if let Some(event) = epochs.nth(0) {
                                    // [!] with dummy t0 = 1st epoch timestamp
                                    //     this will actually return `header section` timestamps
                                    println!("EVENT @ {:#?} - description: {:#?}", event, rinex.event_description(*event)); 
                                }
                            },
							"MET" => {
                                // METEO files checks
                                assert_eq!(rinex.header.obs.is_none(), true);
                                assert_eq!(rinex.is_meteo_rinex(), true);
                                assert_eq!(rinex.header.meteo.is_some(), true);
                                assert_eq!(rinex.header.obs.is_none(), true);
                                let record = rinex.record.as_meteo().unwrap();
                                let mut epochs = record.keys();
                                println!("----- EPOCHs ----- \n{:#?}", epochs);
                                // Testing event description finder
                                if let Some(event) = epochs.nth(0) {
                                    // [!] with dummy t0 = 1st epoch timestamp
                                    //     this will actually return `header section` timestamps
                                    println!("EVENT @ {:#?} - description: {:#?}", event, rinex.event_description(*event)); 
                                }
                            },
                            _ => {}
                        }
                        // SPECIAL METHODS
                        println!("sampling interval  : {:#?}", rinex.sampling_interval());
                        println!("sampling dead time : {:#?}", rinex.dead_times());
                        println!("abnormal epochs    : {:#?}", rinex.epoch_anomalies(None));
                        // COMMENTS
                        println!("---------- Header Comments ----- \n{:#?}", rinex.header.comments);
                        println!("---------- Body   Comments ------- \n{:#?}", rinex.comments);
                        // MERGED RINEX special ops
                        println!("---------- Merged RINEX special ops -----------\n");
                        println!("is merged          : {}", rinex.is_merged());
                        println!("boundaries: \n{:#?}", rinex.merge_boundaries());
                        // Test RINEX writer 
                        rinex.to_file("output").unwrap();
                        // suppress 
                        let _ = std::fs::remove_file("output");
                        //TODO test bench
                        //let identical = diff_is_strictly_identical("test", "data/MET/V2/abvi0010.15m").unwrap();
                        //assert_eq!(identical, true)
                    }
                }
            }
        }
    }
    #[test]
    fn test_macros() {
        assert_eq!(is_comment!("This is a comment COMMENT"), true);
        assert_eq!(is_comment!("This is a comment"), false);
        assert_eq!(is_pseudo_range_obs_code!("C1P"), true);
        assert_eq!(is_pseudo_range_obs_code!("P1P"), true);
        assert_eq!(is_pseudo_range_obs_code!("L1P"), false);
        assert_eq!(is_phase_carrier_obs_code!("L1P"), true);
        assert_eq!(is_phase_carrier_obs_code!("D1P"), false);
        assert_eq!(is_doppler_obs_code!("D1P"), true);
        assert_eq!(is_doppler_obs_code!("L1P"), false);
        assert_eq!(is_sig_strength_obs_code!("S1P"), true);
        assert_eq!(is_sig_strength_obs_code!("L1P"), false);
        assert_eq!(is_temperature_obs_code!("TD"), true);
        assert_eq!(is_temperature_obs_code!("td"), false);
        assert_eq!(is_humidity_obs_code!("HR"), true);
        assert_eq!(is_humidity_obs_code!("RH"), false);
        assert_eq!(is_wet_zenith_code!("ZW"), true);
        assert_eq!(is_wet_zenith_code!("RI"), false);
        assert_eq!(is_wind_speed_code!("WS"), true);
        assert_eq!(is_wind_speed_code!("HI"), false);
        assert_eq!(is_rain_increment_code!("RI"), true);
        assert_eq!(is_rain_increment_code!("HI"), false);
        assert_eq!(is_hail_indicator_code!("HI"), true);
        assert_eq!(is_hail_indicator_code!("RI"), false);
    }
    #[test]
    fn test_shared_methods() {
        let time = chrono::NaiveTime::from_str("00:00:00").unwrap();
        assert_eq!(hourly_session_str(time), "a");
        let time = chrono::NaiveTime::from_str("00:30:00").unwrap();
    }
}
