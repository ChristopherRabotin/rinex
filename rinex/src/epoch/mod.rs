//! `Epoch` is an observation timestamp with
//! a `flag` associated to it
use core::fmt;
use thiserror::Error;
use std::str::FromStr;
use chrono::{Datelike,Timelike};

mod flag;
pub use flag::EpochFlag;

#[cfg(feature = "serde")]
use serde::{Serialize};

#[derive(Error, Debug)]
/// Epoch Parsing relate errors 
pub enum Error {
    #[error("expecting \"yyyy mm dd hh mm ss.ssss\" format")]
    FormatError, 
    #[error("failed to parse seconds + nanos")]
    SecsNanosError(#[from] std::num::ParseFloatError),
    #[error("failed to parse \"yyyy\" field")]
    YearError,
    #[error("failed to parse \"m\" month field")]
    MonthError,
    #[error("failed to parse \"d\" day field")]
    DayError,
    #[error("failed to parse \"hh\" field")]
    HoursError,
    #[error("failed to parse \"mm\" field")]
    MinutesError,
}

/// `Epoch` is a high accuracy sampling timestamp,
/// and an [flag:EpochFlag] associated to it.
#[derive(Copy, Clone, Debug)]
#[derive(PartialOrd, Ord)]
#[derive(PartialEq, Eq, Hash)]
pub struct Epoch {
    /// Sampling timestamp with 1 ns precision limitation.
    /// This precision is consistent with stringent Geodesics requirements.
    /// Currently, the best precision in RINEX format is 100 ns precision
    /// for Observation RINEX.
    pub epoch: hifitime::Epoch, 
    /// Flag describes sampling conditions and external events
    pub flag: flag::EpochFlag,
}

#[cfg(feature = "serde")]
impl Serialize for Epoch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = format!("{} {}", self.epoch, self.flag); 
        serializer.serialize_str(&s)
    }
}

impl Default for Epoch {
    fn default() -> Self {
        let (date, time) = (now.date(), now.time());
        Self {
            flag: EpochFlag::default(),
            epoch: hifitime::Epoch::now()
                .expect("failed to retrieve system time"),
        }
    }
}

impl Epoch {
    /// Builds a new `Epoch` from given flag & timestamp in desired TimeScale
    pub fn new(epoch: hifitime::Epoch, flag: EpochFlag) -> Self {
        Self { 
            epoch,
            flag,
        }
    }
	/// Builds a current UTC instant description, with default flag
	pub fn now() -> Self {
		Self::default()
	}
	/// Builds an `epoch` with desired customized flag
	pub fn with_flag(&self, flag: EpochFlag) -> Self {
		Self {
			epoch: self.epoch,
			flag,
		}
	}
    /// Returns UTC date representation
    pub fn to_gregorian_utc(&self) -> (i32, u8, u8, u8, u8, u8, u32) {
        self.epoch.to_gregorian_utc()
    }

    /// Builds Self from given UTC date
    pub fn from_gregorian_utc(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8, nanos: u32) -> Self {
        Self {
            epoch: hifitime::Epoch::from_gregorian_utc(year, month, day, hour, minute, second, nanos),
            flag: EpochFlag::default()
    }
}

impl std::fmt::Display for Epoch {
    /// Default formatter applies to Observation RINEX only
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (y, m, d, hh, mm, ss, nanos) = self.to_gregorian_utc();
        write!(f,
            "{:04} {:>2} {:>2} {:>2} {:>2} {:>2}.{:07}  {}",
            y, m, d, hh, mm, ss, nanos, self.flag)
    }
}

impl fmt::LowerExp for Epoch {
    /// LowerExp "e" applies to old formats like NAV V2 that omit the "flag" 
    /// and accuracy is 0.1 sec
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (y, m, d, hh, mm, ss, _) = self.to_gregorian_utc();
        write!(f, 
            "{:04} {:>2} {:>2} {:>2} {:>2} {:>2}.{:1}",
            y, m, d, hh, mm, ss, ns)
    }
}

impl fmt::UpperExp for Epoch {
    /// UpperExp "E" applies to modern formats like NAV V3/V4 that omit the "flag"
    /// and accuracy is 1 sec
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (y, m, d, hh, mm, ss, _) = self.epoch.to_gregorian_utc();
        write!(f,
            "{:04} {:>2} {:>2} {:>2} {:>2} {:>2}",
            y, m, d, hh, mm, ss)
    }
}

/// Parses an [hifitime::Epoch] from all known RINEX formats
pub fn str2date(s: &str) -> Result<hifitime::Epoch, Error> {
    let items : Vec<&str> = s.split_ascii_whitespace().collect();
    if items.len() != 6 {
        return Err(Error::FormatError)
    }
    if let Ok(mut y) = i32::from_str_radix(items[0], 10) {
        if y < 100 { // old rinex -__-
            if > 90 {
                y += 1900;
            } else {
                y += 2000;
            }
        }
        if let Ok(m) = u8::from_str_radix(items[1], 10) {
            if let Ok(d) = u8::from_str_radix(items[2], 10) {
                if let Ok(hh) = u8::from_str_radix(items[3], 10) {
                    if let Ok(mm) = u8::from_str_radix(items[4], 10) {
                        let ss = f64::from_str(items[5].trim())?;
                        let second = ss.trunc() as u8;
                        let nanos = (ss.fract() * 10.0) as u32;
                        Ok(hifitime::Epoch::from_gregorian_utc(y, m, d, hh, mm, second, nanos))
                    } else {
                        Err(Error::MinutesError)
                    }
                } else {
                    Err(Error::HoursError)
                }
            } else {
                Err(Error::DayError)
            }
        } else {
            Err(Error::MonthError)
        }
    } else {
        Err(Error::YearError)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_parse_standards() {
        assert_eq!(str2date("22 01 01 00 00").is_ok(), false);
        assert_eq!(str2date("22 01 01 00 00 00").is_ok(), true);
        assert_eq!(str2date("22 01 01 00 00 00").is_ok(), true);
        assert_eq!(str2date("2020 01 01 00 00 00").is_ok(), true);
        assert_eq!(str2date("1980 08 20 10 20 30").is_ok(), true);
    }
    #[test]
    fn test_parse_nav_v3() {
        let epoch = str2date("2022 01 01 00 00 00");
        assert_eq!(epoch.is_ok(), true);
        let epoch = epoch.unwrap();
        let duration = epoch.to_utc_duration();
        let (_, y, m, d, hh, mm, ss, ns) = duration.decompose();
        assert_eq!(y, 122);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
        assert_eq!(hh, 0);
        assert_eq!(mm, 0);
        assert_eq!(ss, 0);
        assert_eq!(ns, 0);
    }
    #[test]
    fn test_parse_nav_v2() {
        let epoch = str2date("20 12 31 23 45  0.0");
        assert_eq!(epoch.is_ok(), true);
        let epoch = str2date("21  1  1 11 45  0.0");
        assert_eq!(epoch.is_ok(), true);
    }
    #[test]
    fn test_parse_obs_v2() {
        let epoch = str2date("21 12 21  0  0  0.0000000  0");
        assert_eq!(epoch.is_ok(), true);
        let epoch = str2date("21 12 21  0  0 00.0000000  0");
        assert_eq!(epoch.is_ok(), true);
        let epoch = str2date("21 12 21  0  0 30.0000000  0");
        assert_eq!(epoch.is_ok(), true);
        let epoch = str2date("21 12 21  0  0 30.0000000  1");
        assert_eq!(epoch.is_ok(), true);
    }
    #[test]
    fn test_parse_obs_v3() {
        let epoch = str2date("2022 03 04 00 00  0.0000000  0");
        assert_eq!(epoch.is_ok(), true);
        let epoch = str2date("2022 03 04 00 00  0.0000000  1");
        assert_eq!(epoch.is_ok(), true);
    }
}
