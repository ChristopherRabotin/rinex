//! Describes a `RINEX` header, includes
//! rinex header parser and associated methods
use crate::leap;
use crate::antex;
use crate::clocks;
use crate::version;
//use crate::gnss_time;
use crate::{is_comment};
use crate::types::{Type, TypeError};
use crate::constellation;
use crate::merge::MergeError;

use crate::meteo;
use crate::observation;

use std::fs::File;
use thiserror::Error;
use std::str::FromStr;
use std::collections::HashMap;
use std::io::{self, prelude::*, BufReader};

#[cfg(feature = "with-serde")]
use serde::{Serialize, Deserialize};

/// Describes a `CRINEX` (compressed rinex) 
pub const CRINEX_MARKER_COMMENT : &str = "COMPACT RINEX FORMAT";
/// End of Header section reached
pub const HEADER_END_MARKER : &str = "END OF HEADER";

/// GNSS receiver description
#[derive(Clone, Debug)]
#[cfg_attr(feature = "with-serde", derive(Serialize, Deserialize))]
pub struct Rcvr {
    /// Receiver (hardware) model
    pub model: String, 
    /// Receiver (hardware) identification info
    pub sn: String, // serial #
    /// Receiver embedded software info
    pub firmware: String, // firmware #
}

impl Default for Rcvr {
    /// Builds a `default` Receiver
    fn default() -> Rcvr {
        Rcvr {
            model: String::new(),
            sn: String::new(),
            firmware: String::new(),
        }
    }
}

impl std::str::FromStr for Rcvr {
    type Err = std::io::Error;
    fn from_str (line: &str) -> Result<Self, Self::Err> {
        let (id, rem) = line.split_at(20);
        let (make, rem) = rem.split_at(20);
        let (version, _) = rem.split_at(20);
        Ok(Rcvr{
            sn: id.trim().to_string(),
            model: make.trim().to_string(),
            firmware: version.trim().to_string(),
        })
    }
}

#[cfg(feature = "with-serde")]
mod point3d_formatter {
    pub fn serialize<S>(point3d: &Option<rust_3d::Point3D>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let p = point3d.as_ref().unwrap_or(
            &rust_3d::Point3D {
                x: 0.0_f64,
                y: 0.0_f64,
                z: 0.0_f64,
            }
        );
        let s = format!("{},{},{}",p.x,p.y,p.z); 
        serializer.serialize_str(&s)
    }
}

/// Antenna description 
#[derive(Debug, Clone)]
#[cfg_attr(feature = "with-serde", derive(Serialize))]
pub struct Antenna {
    /// Hardware model / make descriptor
    pub model: String,
    /// Serial number / identification number
    pub sn: String,
    /// 3D coordinates of reference point
    #[cfg_attr(feature = "with-serde", serde(with = "point3d_formatter"))]
    pub coords: Option<rust_3d::Point3D>,
    /// height in comparison to ref. point
    pub height: Option<f32>,
    /// eastern eccentricity compared to ref. point
    pub eastern_eccentricity: Option<f32>,
    /// northern eccentricity compare to ref. point
    pub northern_eccentricity: Option<f32>,
}

impl Default for Antenna {
    /// Builds default `Antenna` structure
    fn default() -> Antenna {
        Antenna {
            model: String::new(),
            sn: String::new(),
            coords: None,
            height: None,
            eastern_eccentricity: None,
            northern_eccentricity: None,
        }
    }
}

impl std::str::FromStr for Antenna {
    type Err = std::io::Error;
    fn from_str (line: &str) -> Result<Self, Self::Err> {
        let (id, rem) = line.split_at(20);
        let (make, _) = rem.split_at(20);
        Ok(Antenna{
            sn: String::from(id.trim()),
            model: String::from(make.trim()),
            coords: None,
            height: None,
            eastern_eccentricity: None,
            northern_eccentricity : None,
        })
    }
}

impl Antenna {
    pub fn new (sn: &str, model: &str, 
        coords: Option<rust_3d::Point3D>,
            h : Option<f32>, e: Option<f32>, n: Option<f32>) -> Antenna {
        Antenna {
            sn: sn.to_string(),
            model: model.to_string(),
            coords: coords,
            height: h,
            northern_eccentricity: n,
            eastern_eccentricity: e,
        }
    }
}

#[cfg(feature = "with-serde")]
pub mod datetime_formatter {
    use serde::{Serializer};
    pub fn serialize<S>(datetime: &chrono::NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", datetime.format("%Y-%m-%d %H:%M:%S"));
        serializer.serialize_str(&s)
    }

    /*pub fn deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>, 
    {
        let s = String::deserialize(deserializer)?;
        chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")?
    }*/
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "with-serde", derive(Serialize, Deserialize))]
pub enum MarkerType {
    /// Earth fixed & high precision
    Geodetic,
    /// Earth fixed & low precision
    NonGeodetic,
    /// Generated from network
    NonPhysical,
    /// Orbiting space vehicule
    Spaceborne,
    /// Aircraft, balloon..
    Airborne,
    /// Mobile water craft
    Watercraft,
    /// Mobile terrestrial vehicule
    Groundcraft,
    /// Fixed on water surface
    FixedBuoy,
    /// Floating on water surface
    FloatingBuoy,
    /// Floating on ice
    FloatingIce, 
    /// Fixed on glacier
    Glacier,
    /// Rockets, shells, etc..
    Ballistic,
    /// Animal carrying a receiver
    Animal,
    /// Human being carrying a receiver
    Human,
}

impl Default for MarkerType {
    fn default() -> MarkerType { MarkerType::Geodetic }
}

impl std::str::FromStr for MarkerType {
    type Err = std::io::Error; 
    /// Builds a MarkerType from given code descriptor.
    /// This method is not case sensitive
    fn from_str (code: &str) -> Result<Self, Self::Err> {
        match code.to_uppercase().as_str() {
            "GEODETIC" => Ok(Self::Geodetic),
            "NON GEODETIC" => Ok(Self::NonGeodetic),
            "NON_PHYSICAL" => Ok(Self::NonPhysical),
            "SPACE BORNE" => Ok(Self::Spaceborne),
            "AIR BORNE" => Ok(Self::Airborne),
            "WATER CRAFT" => Ok(Self::Watercraft),
            "GROUND CRAFT" => Ok(Self::Groundcraft),
            "FIXED BUOY" => Ok(Self::FixedBuoy),
            "FLOATING BUOY" => Ok(Self::FloatingBuoy),
            "FLOATING ICE" => Ok(Self::FloatingIce),
            "GLACIER" => Ok(Self::Glacier),
            "BALLISTIC" => Ok(Self::Ballistic),
            "ANIMAL" => Ok(Self::Animal),
            "HUMAN" => Ok(Self::Human),
            _ => Ok(Self::default()), 
        }
    }
}

/// Describes `RINEX` file header
#[derive(Clone, Debug)]
#[cfg_attr(feature = "with-serde", derive(Serialize))]
pub struct Header {
    /// revision for this `RINEX`
    pub version: version::Version, 
    /// type of `RINEX` file
    pub rinex_type: Type, 
    /// specific `GNSS` constellation system,
	/// may not exist for RINEX files 
    pub constellation: Option<constellation::Constellation>, 
    /// comments extracted from `header` section
    pub comments : Vec<String>,
    /// program name
    pub program: String, 
    /// program `run by`
    pub run_by: String,
    /// program's `date`
    pub date: String, 
    /// station label
    pub station: String, 
    /// station identifier
    pub station_id: String, 
    /// optionnal station URL 
    pub station_url: String, 
    /// name of observer
    pub observer: String, 
    /// name of production agency
    pub agency: String, 
    /// optionnal receiver placement infos
    pub marker_type: Option<MarkerType>,
    /// optionnal hardware (receiver) infos
    pub rcvr: Option<Rcvr>, 
    /// optionnal antenna infos
    pub ant: Option<Antenna>, 
    /// optionnal leap seconds infos
    pub leap: Option<leap::Leap>, 
    /// station approxiamte coordinates
    #[cfg_attr(feature = "with-serde", serde(with = "point3d_formatter"))]
    pub coords: Option<rust_3d::Point3D>, 
    /// optionnal observation wavelengths
    pub wavelengths: Option<(u32,u32)>, 
    /// optionnal sampling interval (s)
    pub sampling_interval: Option<f32>, 
    /// optionnal file license
    pub license: String,
    /// optionnal Object Identifier (IoT)
    pub doi: String,
    /// optionnal GPS/UTC time difference
    pub gps_utc_delta: Option<u32>,
    /// processing:   
    /// optionnal data scaling
    pub data_scaling: Option<f64>,
    // optionnal ionospheric compensation param(s)
    //ionospheric_corr: Option<Vec<IonoCorr>>,
    // possible time system correction(s)
    //gnsstime_corr: Option<Vec<gnss_time::GnssTimeCorr>>,
    //////////////////////////////////
    // OBSERVATION
    //////////////////////////////////
    pub obs: Option<observation::HeaderFields>,
    //////////////////////////////////
    // Meteo 
    //////////////////////////////////
    pub meteo: Option<meteo::HeaderFields>,
    //////////////////////////////////
    // Clocks fields 
    //////////////////////////////////
    /// Clock Data analysis production center
    pub analysis_center: Option<clocks::AnalysisCenter>,
    /// Clock Data observation codes
    pub clk_codes: Option<Vec<String>>,
    //////////////////////////////////
    // Antex
    //////////////////////////////////
    /// Optionnal Antex fields
    pub antex: Option<antex::HeaderFields>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("CRINEX related content mismatch")]
    CrinexFormatError,
    #[error("RINEX version is not supported '{0}'")]
    VersionNotSupported(String),
    #[error("Line \"{0}\" should begin with Rinex version \"x.yy\"")]
    VersionFormatError(String),
    #[error("rinex type error")]
    TypeError(#[from] TypeError),
    #[error("constellation error")]
    ConstellationError(#[from] constellation::Error),
    #[error("failed to parse leap from \"{0}\"")]
    LeapParsingError(#[from] leap::Error),
    #[error("failed to parse antenna / receiver infos")]
    AntennaRcvrError(#[from] std::io::Error),
    #[error("failed to parse integer value")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("failed to parse float value")]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error("failed to parse date")]
    DateParsingError(#[from] chrono::ParseError),
    #[error("failed to parse ANTEX fields")]
    AntexParsingError(#[from] antex::Error),
}

impl Default for Header {
    fn default() -> Header {
        Header {
            version: version::Version::default(), 
            rinex_type: Type::default(),
            constellation: Some(constellation::Constellation::default()),
            comments: Vec::new(),
            program: String::new(),
            run_by: String::new(),
            date: String::new(),
            station: String::new(),
            station_id: String::new(),
            observer: String::new(),
            agency: String::new(),
            marker_type: None,
            station_url: String::new(),
            doi: String::new(),
            license: String::new(),
            leap: None,
            gps_utc_delta: None,
            // hardware
            rcvr: None,
            ant: None,
            coords: None, 
            wavelengths: None,
            // processing
            data_scaling: None,
            //ionospheric_corr: None,
            //gnsstime_corr: None,
            sampling_interval: None,
            /////////////////////////
            // OBSERVATION
            /////////////////////////
            obs: None,
            /////////////////////////
            // OBSERVATION / METEO
            /////////////////////////
			meteo: None,
            /////////////////////////
            // Clocks
            /////////////////////////
            analysis_center: None,
            clk_codes: None,
            /////////////////////////
            // Antex
            /////////////////////////
            antex: None,
        }
    }
}

impl Header {
    /// Builds a `Header` from local file content
    pub fn new (path: &str) -> Result<Header, Error> { 
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut crinex : Option<observation::Crinex> = None;
        let mut crnx_version = version::Version::default(); 
        let mut rinex_type = Type::default();
        let mut constellation : Option<constellation::Constellation> = None;
        let mut version = version::Version::default();
        let mut comments   : Vec<String> = Vec::new();
        let mut program    = String::new();
        let mut run_by     = String::new();
        let mut date       = String::new();
        let mut station    = String::new();
        let mut station_id = String::new();
        let mut observer   = String::new();
        let mut agency     = String::new();
        let mut license    = String::new();
        let mut doi        = String::new();
        let mut station_url= String::new();
        let mut marker_type : Option<MarkerType> = None; 
        // hardware
        let mut ant        : Option<Antenna> = None;
        let mut ant_coords : Option<rust_3d::Point3D> = None;
        let mut ant_hen    : Option<(f32,f32,f32)> = None;
        let mut rcvr       : Option<Rcvr>    = None;
        // other
        let mut leap       : Option<leap::Leap> = None;
        let mut sampling_interval: Option<f32> = None;
        let mut coords     : Option<rust_3d::Point3D> = None;
        // (OBS)
        let mut obs_clock_offset_applied = false;
        let mut obs_code_lines : u8 = 0; 
        let mut current_code_syst = constellation::Constellation::default(); // to keep track in multi line scenario + Mixed constell 
        let mut obs_codes  : HashMap<constellation::Constellation, Vec<String>> = HashMap::with_capacity(constellation::CONSTELLATION_LENGTH);
        // (OBS/METEO)
		let mut met_codes  : Vec<String> = Vec::new();
		let mut met_sensors: Vec<meteo::Sensor> = Vec::with_capacity(3);
        // CLOCKS
        let mut analysis_center : Option<clocks::AnalysisCenter> = None;
        // ANTEX
        let mut pcv : Option<antex::Pcv> = None;
        let mut ant_relative_values = String::from("AOAD/M_T");
        let mut ref_ant_sn : Option<String> = None;

        for l in reader.lines() {
            let line = &l.unwrap();
            ///////////////////////////////
            // [0] COMMENTS
            ///////////////////////////////
            if is_comment!(line) {
                let comment = line.split_at(60).0;
                // --> storing might be useful
                comments.push(comment.trim_end().to_string());
                continue
            }
            //////////////////////////////////////
            // [1] CRINEX Special field [1]
            /////////////////////////////////////
            else if line.contains("CRINEX VERS") {
                let version = line.split_at(20).0;
                crnx_version = version::Version::from_str(version.trim())?
            //////////////////////////////////////
            // [1*] CRINEX Special field [2]
            /////////////////////////////////////
            } else if line.contains("CRINEX PROG / DATE") {
                let (pgm, remainder) = line.split_at(20);
                let (_, remainder) = remainder.split_at(20);
                let date = remainder.split_at(20).0.trim();
                crinex = Some(
                    observation::Crinex {
                        version: crnx_version, 
                        prog: pgm.trim().to_string(),
                        date: chrono::NaiveDateTime::parse_from_str(date, "%d-%b-%y %H:%M")?
                    })
            }
            
            ////////////////////////////////////////
            // [2*] ANTEX special RINEX
            ////////////////////////////////////////
            else if line.contains("ANTEX VERSION / SYST") {
                let line = line.split_at(60).0;
                let (vers, system) = line.split_at(8);
                version = version::Version::from_str(vers.trim())?;
                constellation = Some(constellation::Constellation::from_str(system.trim())?);
            } 
            else if line.contains("PCV TYPE / REFANT") {
                let line = line.split_at(60).0;
                let (pcv_str, rem) = line.split_at(20);
                let (ref_type, rem) = rem.split_at(20);
                let (ref_sn, _) = rem.split_at(20);
                pcv = Some(antex::Pcv::from_str(pcv_str)?);
                if ref_type.trim().len() > 0 {
                    ant_relative_values = ref_type.trim().to_string();
                }
                if ref_sn.trim().len() > 0 {
                    ref_ant_sn = Some(ref_sn.trim().to_string())
                }
            }

            ///////////////////////////////////////
            // ==> from now on
            // RINEX classical header attributes
            ///////////////////////////////////////
            else if line.contains("RINEX VERSION / TYPE") {
                let (vers, rem) = line.split_at(20);
                let (type_str, rem) = rem.split_at(20); 
                let (constell_str, _) = rem.split_at(20);
                rinex_type = Type::from_str(type_str.trim())?;
                if type_str.contains("GLONASS") {
                    // special case, sometimes GLONASS NAV
                    // drops the constellation field cause it's implied
                    constellation = Some(constellation::Constellation::Glonass)
                } else if type_str.contains("METEOROLOGICAL DATA") {
                    // these files are not tied to a constellation system,
                    // therefore, do not have this field
                    constellation = None
                } else { // regular files
                    constellation = Some(constellation::Constellation::from_str(constell_str.trim())?)
                }
                version = version::Version::from_str(vers.trim())?;
                if !version.is_supported() {
                    return Err(Error::VersionNotSupported(vers.to_string()))
                }
            }
            else if line.contains("PGM / RUN BY / DATE") {
                let (pgm, rem) = line.split_at(20);
                program = pgm.trim().to_string();
                let (rb, rem) = rem.split_at(20);
                run_by = match rb.trim().eq("") {
                    true => String::from("Unknown"),
                    false => rb.trim().to_string(), 
                };
                let (date_str, _) = rem.split_at(20);
                date = date_str.trim().to_string()
            }
            else if line.contains("MARKER NAME") {
                station = line.split_at(20).0.trim().to_string()
            } else if line.contains("MARKER NUMBER") {
                station_id = line.split_at(20).0.trim().to_string()
            } else if line.contains("MARKER TYPE") {
                let code = line.split_at(20).0.trim();
                marker_type = Some(MarkerType::from_str(code).unwrap());
            
            } else if line.contains("OBSERVER / AGENCY") {
                let (content, _) = line.split_at(60);
                let (obs, ag) = content.split_at(20);
                observer = obs.trim().to_string();
                agency = ag.trim().to_string()

            } else if line.contains("REC # / TYPE / VERS") {
                rcvr = Some(Rcvr::from_str(&line)?) 

			} else if line.contains("SENSOR MOD/TYPE/ACC") {
				let (content, _) = line.split_at(60);
				let (model, rem) = content.split_at(20);
				let (stype, rem) = rem.split_at(20+6);
				let (accuracy, rem) = rem.split_at(7+4);
				let accuracy = f32::from_str(accuracy.trim())?;
				let (physics, _) = rem.split_at(2);
				met_sensors.push(
                    meteo::Sensor {
                        model: model.trim().to_string(),
                        sens_type: stype.trim().to_string(),
                        accuracy,
                        physics: physics.trim().to_string(),
                    }
                )
            
            } else if line.contains("ANT # / TYPE") {
                ant = Some(Antenna::from_str(&line)?)
            
            } else if line.contains("LEAP SECOND") {
                leap = Some(leap::Leap::from_str(line.split_at(40).0)?)

            } else if line.contains("DOI") {
                let (content, _) = line.split_at(40); //  TODO: confirm please
                doi = content.trim().to_string()

            } else if line.contains("MERGED FILE") {
                //TODO V > 3 nb# of merged files

            } else if line.contains("STATION INFORMATION") {
                let (url, _) = line.split_at(40); //TODO confirm please 
                station_url = url.trim().to_string()

            } else if line.contains("LICENSE OF USE") {
                let (lic, _) = line.split_at(40); //TODO confirm please 
                license = lic.trim().to_string()
            
            } else if line.contains("WAVELENGTH FACT L1/2") {
                //TODO

            } else if line.contains("APPROX POSITION XYZ") {
                let items: Vec<&str> = line.split_ascii_whitespace()
                    .collect();
                let (x, y, z): (f64,f64,f64) = 
                    (f64::from_str(items[0].trim())?,
                    f64::from_str(items[1].trim())?,
                    f64::from_str(items[2].trim())?);
                coords = Some(rust_3d::Point3D::new(x,y,z))

            } else if line.contains("ANTENNA: DELTA H/E/N") {
                let (h, rem) = line.split_at(15);
                let (e, rem) = rem.split_at(15);
                let (n, _) = rem.split_at(15);
                ant_hen = Some((
                    f32::from_str(h.trim())?,
                    f32::from_str(e.trim())?,
                    f32::from_str(n.trim())?))

            } else if line.contains("ANTENNA: DELTA X/Y/Z") {
                let items: Vec<&str> = line.split_ascii_whitespace()
                    .collect();
                let (x, y, z): (f64,f64,f64) = 
                    (f64::from_str(items[0].trim())?,
                    f64::from_str(items[1].trim())?,
                    f64::from_str(items[2].trim())?);
                ant_coords = Some(rust_3d::Point3D::new(x,y,z))

            } else if line.contains("ANTENNA: B.SIGHT XYZ") {
                //TODO
            } else if line.contains("ANTENNA: ZERODIR XYZ") {
                //TODO
            } else if line.contains("CENTER OF MASS: XYZ") {
                //TODO
            } else if line.contains("ANTENNA: PHASECENTER") {
                //TODO
            
            } else if line.contains("RCV CLOCK OFFS APPL") {
                let ok_str = line.split_at(20).0.trim();
                obs_clock_offset_applied = i32::from_str_radix(ok_str, 10)? != 0

            } else if line.contains("# OF SATELLITES") {
                // will always appear prior PRN/#OBS
                // determines nb of satellites in observation file
                //let (nb, _) = line.split_at(10);
                //obs_nb_sat = u32::from_str_radix(nb.trim(), 10)?

            } else if line.contains("PRN / # OF OBS") {
                let (sv, _) = line.split_at(7);
                if sv.trim().len() > 0 {
                    
                }
                // lists all Sv
                //let items: Vec<&str> = line.split_ascii_whitespace()
                //    .collect();
                 
            } else if line.contains("SYS / PHASE SHIFT") {
                //TODO
            } else if line.contains("SYS / PVCS APPLIED") {
                // RINEX::ClockData specific 
                // + satellite system (G/R/E/C/I/J/S)
                // + programe name to apply Phase Center Variation
                // + source of corrections (url)
                // <o repeated for each satellite system
                // <o blank field when no corrections applied
            } else if line.contains("# / TYPES OF DATA") {
                // RINEX::ClockData specific 
                // + number of different clock data types stored
                // + list of clock data  types
            } else if line.contains("TYPES OF OBS") { 
                // RINEX OBS code descriptor (V < 3) 
                // ⚠ ⚠ could either be observation or meteo data
                if obs_code_lines == 0 {
                    // [x] OBS CODES 1st line 
                    let (rem, _) = line.split_at(60); // cleanup
                    let (n_codes, rem) = rem.split_at(6);
                    let n_codes = u8::from_str_radix(n_codes.trim(), 10)?;
                    obs_code_lines = num_integer::div_ceil(n_codes, 9); // max. per line
                    // --> parse this line 
                    let codes : Vec<String> = rem
                        .split_ascii_whitespace()
                        .map(|r| r.trim().to_string())
                        .collect();
                    if rinex_type == Type::ObservationData {
                        match constellation {
                            Some(constellation::Constellation::Mixed) => {
                                // Old RINEX + Mixed Constellation:
                                // description is not accurate enough to determine which
                                // code will be measured for which constellation
                                // ---> copy them for all known constellations 
                                let constells : Vec<constellation::Constellation> = vec![
                                    constellation::Constellation::GPS,
                                    constellation::Constellation::Glonass,
                                    constellation::Constellation::Galileo,
                                    constellation::Constellation::Beidou,
                                    constellation::Constellation::Sbas(constellation::Augmentation::default()),
                                    constellation::Constellation::QZSS,
                                ];
                                for i in 0..constells.len() {
                                    obs_codes.insert(constells[i], codes.clone());
                                } 
                            },
                            Some(constellation) => {
                                obs_codes.insert(constellation, codes.clone());
                            },
                            None => unreachable!("OBS rinex with no constellation specified"),
                        }
                    } else if rinex_type == Type::MeteoData {
                        for c in codes {
                            met_codes.push(c);
                        }
                    }
                    obs_code_lines -= 1
                } else {
                    // [*] OBS CODES following line(s) 
                    // --> parse this line 
                    let (rem, _) = line.split_at(60); // cleanup
                    let codes : Vec<String> = rem
                        .split_ascii_whitespace()
                        .map(|r| r.trim().to_string())
                        .collect(); 
                    if rinex_type == Type::ObservationData {
                        // retrieve correspond system and append codes with new values 
                        let to_retrieve : Vec<constellation::Constellation> = match constellation {
                            Some(constellation::Constellation::Mixed) => {
                                vec![ // Old OBS Data + Mixed constellation ==> no means to differentiate
                                    constellation::Constellation::GPS,
                                    constellation::Constellation::Glonass,
                                    constellation::Constellation::Galileo,
                                    constellation::Constellation::Beidou,
                                    constellation::Constellation::Sbas(constellation::Augmentation::default()),
                                    constellation::Constellation::QZSS,
                                ]
                            },
                            Some(c) => vec![c],
                            None => unreachable!("OBS rinex with no constellation specified"),
                        };
                        for r in to_retrieve {
                            // retrieve map being built
                            if let Some(mut prev) = obs_codes.remove(&r) {
                                // increment obs code map
                                for code in &codes {
                                    prev.push(code.to_string());
                                }
                                obs_codes.insert(r, prev); // (re)insert
                            } 
                        }
                    } else if rinex_type == Type::MeteoData {
                        // simple append, list is simpler
                        for c in codes {
                            met_codes.push(c)
                        }
                    }
                    obs_code_lines -= 1
                }

            } else if line.contains("SYS / # / OBS TYPES") {
                // RINEX OBS code descriptor (V > 2) 
                if obs_code_lines == 0 {
                    // [x] OBS CODES 1st line
                    let (line, _) = line.split_at(60); // cleanup 
                    let (identifier, rem) = line.split_at(1);
                    let (n_codes, rem) = rem.split_at(5);
                    let n_codes = u8::from_str_radix(n_codes.trim(), 10)?;
                    obs_code_lines = num_integer::div_ceil(n_codes, 13); // max. per line
                    // --> parse this line
                    let codes : Vec<String> = rem
                        .split_ascii_whitespace()
                        .map(|r| r.trim().to_string())
                        .collect();
                    current_code_syst = constellation::Constellation::from_1_letter_code(identifier)?;
                    obs_codes.insert(current_code_syst, codes);
                } else {
                    let rem = line.split_at(60).0; // cleanup
                    // --> parse this line
                    let codes : Vec<String> = rem
                        .split_ascii_whitespace()
                        .map(|r| r.trim().to_string())
                        .collect();
                    // retrieve map being built
                    if let Some(mut prev) = obs_codes.remove(&current_code_syst) {
                        // increment obs code map
                        for code in codes {
                            prev.push(code);
                        }
                        obs_codes.insert(current_code_syst, prev); // (re)insert)
                    }
                } 
                obs_code_lines -= 1
            } else if line.contains("ANALYSIS CENTER") {
                let line = line.split_at(60).0;
                let (code, agency) = line.split_at(3);
                analysis_center = Some(clocks::AnalysisCenter::new(code.trim(), agency.trim()));

            } else if line.contains("# / TYPES OF DATA") {
                //TODO
                /*let line = line.split_at(60).0;
                let (n, rem) = line.split_at(10); // TODO
                let n = u8::from_str_radix(n,10)?;
                let mut line = rem.clone();
                for i in 0..n { // parse CLOCKS codes
                    let (code, rem) = line.split_at(10); // TODO
                    clocks_code.push(code);
                    line = rem.clone()
                }*/
         
            } else if line.contains("SIGNAL STRENGHT UNIT") {
                //TODO
            } else if line.contains("INTERVAL") {
                let intv = line.split_at(20).0.trim();
                sampling_interval = Some(f32::from_str(intv)?)

            } else if line.contains("GLONASS SLOT / FRQ #") {
                //TODO
            } else if line.contains("GLONASS COD/PHS/BIS") {
                //TODO

            } else if line.contains("ION ALPHA") { 
                //TODO
                //0.7451D-08 -0.1490D-07 -0.5960D-07  0.1192D-06          ION ALPHA           

            } else if line.contains("ION BETA") {
                //TODO
                //0.9011D+05 -0.6554D+05 -0.1311D+06  0.4588D+06          ION BETA            
            } else if line.contains("IONOSPHERIC CORR") {
                // TODO
                // GPSA 0.1025E-07 0.7451E-08 -0.5960E-07 -0.5960E-07
                // GPSB 0.1025E-07 0.7451E-08 -0.5960E-07 -0.5960E-07

            } else if line.contains("TIME SYSTEM CORR") {
                // TODO
                // GPUT 0.2793967723E-08 0.000000000E+00 147456 1395
            
            } else if line.contains("DELTA-UTC") {
                //TODO
                //0.931322574615D-09 0.355271367880D-14   233472     1930 DELTA-UTC: A0,A1,T,W
            }
        }
        
        let ant : Option<Antenna> = match ant {
            Some(antenna) => {
                Some(Antenna::new(
                    &antenna.sn, 
                    &antenna.model, 
                    ant_coords, 
                    Some(ant_hen.unwrap_or((0.0_f32,0.0_f32,0.0_f32)).0), 
                    Some(ant_hen.unwrap_or((0.0_f32,0.0_f32,0.0_f32)).1), 
                    Some(ant_hen.unwrap_or((0.0_f32,0.0_f32,0.0_f32)).2), 
                ))
            },
            _ => None,
        };

        Ok(Header{
            version: version,
            rinex_type,
            constellation,
            comments,
            program,
            run_by,
            date,
            station,
            station_id,
            agency,
            observer,
            license,
            doi,
            station_url,
            marker_type,
            rcvr, 
            ant, 
            leap,
            coords: coords,
            wavelengths: None,
            gps_utc_delta: None,
            sampling_interval: sampling_interval,
            data_scaling: None,
            //ionospheric_corr: None,
            //gnsstime_corr: None,
            ///////////////////////
            // OBSERVATION
            ///////////////////////
            obs: {
                if obs_codes.len() > 0 {
                    Some(observation::HeaderFields {
                        crinex: crinex.clone(),
                        codes: obs_codes.clone(),
                        clock_offset_applied: obs_clock_offset_applied,
                    })
                } else {
                    None
                }
            },
            ////////////////////////
            // OBSERVATION / METEO
            ////////////////////////
            meteo: {
                if met_codes.len() > 0 {
                    Some(meteo::HeaderFields {
                        codes: met_codes.clone(),
                        sensors: met_sensors.clone(),
                    })
                } else {
                    None
                }
            },
            ///////////////////////
            // CLOCKS
            ///////////////////////
            clk_codes: None,
            analysis_center,
            ///////////////////////
            // ANTEX
            ///////////////////////
            antex: {
                if let Some(pcv) = pcv {
                    Some(antex::HeaderFields {
                        pcv,
                        relative_values: ant_relative_values,
                        reference_sn: ref_ant_sn,
                    })
                } else {
                    None
                }
            },
        })
    }
    /// `Merges` self and given header
    /// we call this maethod when merging two rinex record
    /// to create the optimum combined/total RINEX file.
    /// This is not a feature of teqc.
    /// When merging:  
    ///  + retains oldest revision number  
    ///  + constellation remains identical if self & `b` share the same constellation,
    ///   otherwise, self::constellation is upgraded to `mixed`.  
    ///  + `b` comments are retained, header section comments are not analyzed   
    ///  + prefers self::attriutes over `b` attributes  
    ///  + appends (creates) `b` attributes that do not exist in self
    ///TODO: sampling interval special case
    ///TODO: rcvr_clock_offset_applied special case :
    /// apply/modify accordingly
    ///TODO: data scaling special case: apply/modify accordingly
    pub fn merge (&mut self, header: &Self) -> Result<(), MergeError> {
        if self.rinex_type != header.rinex_type {
            return Err(MergeError::FileTypeMismatch)
        }

        let (a_rev, b_rev) = (self.version, header.version);
        let (a_cst, b_cst) = (self.constellation, header.constellation);
        // constellation upgrade ?
        if a_cst != b_cst {
            self.constellation = Some(constellation::Constellation::Mixed)
        }
        // retain oldest revision
        self.version = std::cmp::min(a_rev, b_rev);
        for c in &header.comments {
            self.comments.push(c.to_string()) 
        } 
        // leap second new info ?
        if let Some(leap) = header.leap {
            if self.leap.is_none() {
                self.leap = Some(leap)
            }
        }
        if let Some(delta) = header.gps_utc_delta {
            if self.gps_utc_delta.is_none() {
                self.gps_utc_delta = Some(delta)
            }
        }
        if let Some(rcvr) = &header.rcvr {
            if self.rcvr.is_none() {
                self.rcvr = Some(Rcvr {
                    model: rcvr.model.clone(),
                    sn: rcvr.sn.clone(),
                    firmware: rcvr.firmware.clone(),
                })
            }
        }
        if let Some(ant) = &header.ant {
            if self.ant.is_none() {
                self.ant = Some(Antenna {
                    model: ant.model.clone(),
                    sn: ant.sn.clone(),
                    coords: ant.coords.clone(),
                    height: ant.height,
                    eastern_eccentricity: ant.eastern_eccentricity,
                    northern_eccentricity: ant.northern_eccentricity,
                })
            }
        }
        //TODO append new array
        /*if let Some(a) = &header.sensors {
            if let Some(b) = &self.sensors {
                for sens in a {
                    if !b.contains(sens) {
                        b.push(*sens)
                    }
                }
            } else {
                self.sensors = Some(a.to_vec())
            }
        }*/
        if let Some(coords) = &header.coords {
            if self.coords.is_none() {
                self.coords = Some(rust_3d::Point3D {
                    x: coords.x,
                    y: coords.y,
                    z: coords.z,
                })
            }
        }
        if let Some(wavelengths) = header.wavelengths {
            if self.wavelengths.is_none() {
                self.wavelengths = Some(wavelengths)
            }
        }
        //TODO as mut ref
        /*if let Some(a) = &header.obs_codes {
            if let Some(&mut b) = self.obs_codes.as_ref() {
                for (k, v) in a {
                    b.insert(*k, v.to_vec());
                }
            } else {
                self.obs_codes = Some(a.clone())
            }
        }*/
        
        /*if let Some(a) = header.data_scaling {
            if let Some(b) = self.data_scaling {

            } else {

            }
        } else {
            if let Some(b) = self.data_scaling {

            }
        }*/

        Ok(())
    }
    
    /// Returns true if self is a `Compressed RINEX`
    pub fn is_crinex (&self) -> bool { 
        if let Some(obs) = &self.obs {
            obs.crinex.is_some()
        } else {
            false
        }
    }

    /// Creates a Basic Header structure
    /// for NAV RINEX
    pub fn basic_nav() -> Self {
        Self::default()
            .with_type(Type::NavigationData)
    }
    
    /// Creates a Basic Header structure
    /// for OBS RINEX
    pub fn basic_obs() -> Self {
        Self::default()
            .with_type(Type::ObservationData)
    }

    /// Returns Header structure with desired RINEX type
    pub fn with_type(&self, t: Type) -> Self {
        let mut s = self.clone();
        s.rinex_type = t;
        s
    } 

    /// Adds general information to Self
    pub fn with_general_infos (&self, program: &str, run_by: &str, agency: &str) -> Self {
        let mut s = self.clone();
        s.program = program.to_string();
        s.run_by = run_by.to_string();
        s.agency = agency.to_string();
        s
    }

    /// Adds receiver information to self
    pub fn with_rcvr (&self, r: Rcvr) -> Self {
        let mut s = self.clone();
        s.rcvr = Some(r);
        s
    }
    
    /// Adds antenna information to self
    pub fn with_antenna (&self, a: Antenna) -> Self {
        let mut s = self.clone();
        s.ant = Some(a);
        s
    }

    /// Adds desired constellation to Self
    pub fn with_constellation (&self, c: constellation::Constellation) -> Self {
        let mut s = self.clone();
        s.constellation = Some(c);
        s
    }

    /// adds comments to Self
    pub fn with_comments (&self, c: Vec<String>) -> Self {
        let mut s = self.clone();
        s.comments = c.clone();
        s
    }
}

impl std::fmt::Display for Header {
    /// `header` formatter, mainly for 
    /// `RINEX` file production purposes
    fn fmt (&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_crinex() {
            // two special header lines
        }
        // RINEX VERSION / TYPE 
        write!(f, "{:6}.{:02}           ", self.version.major, self.version.minor)?;
        match self.rinex_type {
            Type::NavigationData => {
                match self.constellation {
                    Some(constellation::Constellation::Glonass) => {
                        // Glonass Special case
                        write!(f,"{:<20}", "G: GLONASS NAV DATA")?;
                        write!(f,"{:<20}", "")?;
                        write!(f,"{}", "RINEX VERSION / TYPE\n")?
                    },
                    Some(c) => {
                        write!(f,"{:<20}", "NAVIGATION DATA")?;
                        write!(f,"{:<20}", c.to_1_letter_code())?;
                        write!(f,"{:<20}", "RINEX VERSION / TYPE\n")?
                    },
                    _ => panic!("constellation must be specified when formatting a NavigationData") 
                }
            },
            Type::ObservationData => {
                match self.constellation {
                    Some(c) => {
                        write!(f,"{:<20}", "OBSERVATION DATA")?;
                        write!(f,"{:<20}", c.to_1_letter_code())?;
                        write!(f,"{:<20}", "RINEX VERSION / TYPE\n")?
                    },
                    _ => panic!("constellation must be specified when formatting ObservationData")
                }
            },
            Type::MeteoData => {
                write!(f,"{:<20}", "METEOROLOGICAL DATA")?;
                write!(f,"{:<20}", "")?;
                write!(f,"{:<20}", "RINEX VERSION / TYPE\n")?
            },
            Type::AntennaData => {}, //TODO
        }
        // COMMENTS 
        for comment in self.comments.iter() {
            write!(f, "{:<60}", comment)?;
            write!(f, "COMMENT\n")?
        }
        // PGM / RUN BY / DATE
        write!(f, "{:<20}", self.program)?;
        write!(f, "{:<20}", self.run_by)?;
        write!(f, "{:<20}", self.date)?; //TODO
        write!(f, "{}", "PGM / RUN BY / DATE\n")?; 
        // OBSERVER / AGENCY
        write!(f, "{:<20}", self.observer)?;
        write!(f, "{:<40}", self.agency)?;
        write!(f, "OBSERVER / AGENCY\n")?; 
        // MARKER NAME
        write!(f, "{:<20}", self.station)?;
        write!(f, "{:<40}", " ")?;
        write!(f, "{}", "MARKER NAME\n")?;
        // MARKER NUMBER
        write!(f, "{:<20}", self.station_id)?;
        write!(f, "{:<40}", " ")?;
        write!(f, "{}", "MARKER NUMBER\n")?;
        // ANT
        if let Some(ant) = &self.ant {
            write!(f, "{:<20}", ant.sn)?;
            write!(f, "{:<40}", ant.model)?;
            write!(f, "{}", "ANT # / TYPE\n")?;
            if let Some(coords) = &ant.coords {
                write!(f, "{:<20}", coords.x)?;
                write!(f, "{:<20}", coords.y)?;
                write!(f, "{:<20}", coords.z)?;
                write!(f, "{}", "APPROX POSITION XYZ\n")?
            }
            if let Some(h) = &ant.height {
                write!(f, "{:<20}", h)?;
                write!(f, "{:<20}", ant.eastern_eccentricity.unwrap_or(0.0_f32))?;
                write!(f, "{:<20}", ant.northern_eccentricity.unwrap_or(0.0_f32))?;
                write!(f, "{}", "ANTENNA: DELTA H/E/N\n")?
            }
        }
        // RCVR
        if let Some(rcvr) = &self.rcvr {
            write!(f, "{:<20}", rcvr.sn)?; 
            write!(f, "{:<20}", rcvr.model)?; 
            write!(f, "{:<20}", rcvr.firmware)?;
            write!(f, "REC # / TYPE / VERS\n")?
        }
        // INTERVAL
        if let Some(interval) = &self.sampling_interval {
            write!(f, "{:10.3}", interval)?;
            write!(f, "{:<50}", "")?;
            write!(f, "INTERVAL\n")?
        }
        // OBS codes
        match self.rinex_type {
            Type::ObservationData => {
                if let Some(obs) = &self.obs {
                    match self.version.major {
                        1|2 => { // old revisions
                            for (_constell, codes) in obs.codes.iter() {
                                let mut line = format!("{:6}", codes.len()); 
                                for i in 0..codes.len() {
                                    if (i+1)%11 == 0 {
                                        line.push_str("# / TYPES OF OBS\n");
                                        write!(f, "{}", line)?;
                                        line.clear();
                                        line.push_str(&format!("{:<6}", ""));
                                    }
                                    line.push_str(&format!(" {:>5}", codes[i]));
                                }
                                line.push_str(&format!("{:<width$}", "", width=60-line.len()));
                                line.push_str("# / TYPES OF OBS\n"); 
                                write!(f, "{}", line)?;
                                break // only once
                            }
                        },
                        _ => { // modern revisions
                            for (constell, codes) in obs.codes.iter() {
                                let mut line = format!("{:<4}", constell.to_1_letter_code());
                                line.push_str(&format!("{:2}", codes.len())); 
                                for i in 0..codes.len() {
                                    if (i+1)%14 == 0 {
                                        line.push_str(&format!("{:<width$}", "", width=60-line.len()));
                                        line.push_str("SYS / # / OBS TYPES\n"); 
                                        write!(f, "{}", line)?;
                                        line.clear();
                                        line.push_str(&format!("{:<6}", ""));
                                    }
                                    line.push_str(&format!(" {}", codes[i]))
                                }
                                line.push_str(&format!("{:<width$}", "", width=60-line.len()));
                                line.push_str("SYS / # / OBS TYPES\n"); 
                                write!(f, "{}", line)?
                            }
                        },
                    }
                } else {
                    panic!("Observation RINEX with no `obs codes` specified")
                }
            },
            Type::MeteoData => {
                if let Some(obs) = &self.meteo {
                    let codes = &obs.codes;
                    let mut line = format!("{:6}", codes.len()); 
                    for i in 0..codes.len() {
                        if (i+1)%9 == 0 {
                            line.push_str("# / TYPES OF OBS\n");
                            write!(f, "{}", line)?;
                            line.clear();
                            line.push_str(&format!("{:<6}", ""));
                        }
                        line.push_str(&format!(" {:>5}", codes[i]));
                    }
                    line.push_str(&format!("{:<width$}", "", width=60-line.len()));
                    line.push_str("# / TYPES OF OBS\n"); 
                    write!(f, "{}", line)?;
                } else {
                    panic!("Meteo RINEX with no `obs codes` specified")
                }
            },
            _ => {},
        }
        // LEAP
        if let Some(leap) = &self.leap {
            write!(f, "{:6}", leap.leap)?;
            if let Some(delta) = &leap.delta_tls {
                write!(f, "{:6}", delta)?;
                write!(f, "{:6}", leap.week.unwrap_or(0))?;
                write!(f, "{:6}", leap.day.unwrap_or(0))?;
                if let Some(system) = &leap.system {
                    write!(f, "{:<10}", system.to_3_letter_code())?
                } else {
                    write!(f, "{:<10}", " ")?
                }
            } else {
                write!(f, "{:<40}", " ")?
            }
            write!(f, "LEAP SECONDS\n")?
        }
        // SENSOR(s)
        if let Some(meteo) = &self.meteo {
            let sensors = &meteo.sensors;
            for sensor in sensors {
                write!(f, "{:<20}", sensor.model)?; 
                write!(f, "{:<30}", sensor.sens_type)?; 
                write!(f, "{:1.1}", sensor.accuracy)?; 
                write!(f, "{:<5}", sensor.physics)?; 
                write!(f, "SENSOR MOD/TYPE/ACC\n")?
            }
        }
        // END OF HEADER
        write!(f, "{:>74}", "END OF HEADER\n")
    }
}
