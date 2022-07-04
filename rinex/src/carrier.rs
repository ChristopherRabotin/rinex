//! `GNSS` constellations & associated methods
use thiserror::Error;
use serde_derive::{Deserialize, Serialize};

/// Carrier code
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Code {
    /// GPS/GLONASS/QZSS/SBAS L1 C/A,
    C1, 
    /// GPS/GLONASS L1P
    P1,
    /// Beidou B1i
    B1,
    /// Galileo E1
    E1,
    /// GPS / QZSS L2C
    C2, 
    /// GPS / GLONASS L2P
    P2,
    /// Beidou B2i
    B2,
    /// Galileo E5
    E5,
}

#[derive(Debug)]
pub enum CodeError {
    /// Unknown Carrier code identifier
    UnknownCode(String),
}

impl std::str::FromStr for Code {
    type Err = CodeError;
    fn from_str (code: &str) -> Result<Code, CodeError> {
        if code.eq("C1") {
            Ok(Code::C1)
        } else if code.eq("C2") {
            Ok(Code::C2)
        } else if code.contains("P1") {
            Ok(Code::P1)
        } else if code.contains("P2") {
            Ok(Code::P2)
        } else if code.contains("B1") | code.eq("B1i") {
            Ok(Code::B1)
        } else if code.eq("B2") | code.eq("B2i") {
            Ok(Code::B2)
        } else if code.eq("E1") {
            Ok(Code::E1)
        } else if code.eq("E5") | code.eq("E5a") {
            Ok(Code::E5)
        } else {
            Err(CodeError::UnknownCode(code.to_string()))
        }
    }
}

impl std::fmt::Display for Code {
    fn fmt (&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Code::C1 => fmt.write_str("C1"),
            Code::C2 => fmt.write_str("C2"),
            Code::P1 => fmt.write_str("P1"),
            Code::P2 => fmt.write_str("P2"),
            Code::B1 => fmt.write_str("B1"),
            Code::B2 => fmt.write_str("B2"),
            Code::E1 => fmt.write_str("E1"),
            Code::E5 => fmt.write_str("E5"),
        }
    }
}

impl Default for Code {
    /// Builds `C1` as default code
    fn default() -> Code {
        Code::C1
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Channel {
    /// L1 band
    L1,
    /// L2 band
    L2,
    /// L5 band
    L5,
    /// Glonass 1 channel
    G1(u8),
    /// Glonass 2 channel
    G2(u8),
}

#[derive(Error, Debug)]
pub enum ChannelError {
    /// Unable to parse Channel from given string content
    #[error("unable to parse channel from content \"{0}\"")]
    ParseError(String),
    #[error("unable to identify glonass channel from \"{0}\"")]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl std::str::FromStr for Channel {
    type Err = ChannelError; 
    fn from_str (s: &str) -> Result<Self, Self::Err> {
        if s.contains("L1") { 
            Ok(Channel::L1)
        } else if s.contains("L2") {
            Ok(Channel::L2)
        } else if s.contains("L5") {
            Ok(Channel::L5)
        
        } else if s.contains("G1") {
            if s.eq("G1") {
                Ok(Channel::G1(0))
            } else if s.contains("G1(") {
                let items : Vec<&str> = s.split("(").collect();
                let item = items[1].replace(")","");
                Ok(Channel::G1(
                    u8::from_str_radix(&item, 10)?))
            } else {
                Err(ChannelError::ParseError(s.to_string()))
            }
        
        } else if s.contains("G2") {
            if s.eq("G2") {
                Ok(Channel::G2(0))
            } else if s.contains("G2(") {
                let items : Vec<&str> = s.split("(").collect();
                let item = items[1].replace(")","");
                Ok(Channel::G2(
                    u8::from_str_radix(&item, 10)?))
            } else {
                Err(ChannelError::ParseError(s.to_string()))
            }

        } else {
            Err(ChannelError::ParseError(s.to_string())) 
        }
    }
}

impl Channel {
    /// Returns frequency associated to this channel in MHz 
    pub fn carrier_frequency_mhz (&self) -> f64 {
        match self {
            Channel::L1 => 1575.42_f64,
            Channel::L2 => 1227.60_f64,
            Channel::L5 => 1176.45_f64,
            Channel::G1(c) => 1602.0_f64 + (*c as f64 *9.0/16.0), 
            Channel::G2(c) => 1246.06_f64 + (*c as f64 * 7.0/16.0),
        }
    }
    
    /// Returns channel bandwidth in MHz
    pub fn bandwidth_mhz (&self) -> f64 {
        match self {
            Channel::L1 | Channel::G1(_) => 15.345_f64,
            Channel::L2 | Channel::G2(_) => 11.0_f64,
            Channel::L5 => 12.5_f64,
        }
    }
}

mod test {
    use super::*;
    #[test]
    fn test_code() {
        assert_eq!(super::Code::from_str("L1").is_err(),  false);
        assert_eq!(super::Code::from_str("E5a").is_err(), false);
        assert_eq!(super::Code::from_str("E7").is_err(),  true);
        assert_eq!(super::Code::from_str("L1").unwrap().frequency(), 1575.42_f64);
    }
}
