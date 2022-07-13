//! `MeteoData` related structures & methods
use std::io::Write;
use thiserror::Error;
use std::str::FromStr;
use std::collections::{BTreeMap, HashMap};
use crate::epoch;
use crate::header;
use crate::header::Header;

/// Observation Sensor
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "with-serde", derive(Serialize, Deserialize))]
pub struct Sensor {
	/// Model of this sensor
	pub model: String,
	/// Type of sensor
	pub sens_type: String,
	/// Sensor accuracy [°C,..]
	pub accuracy: f32,
	/// Physics measured by this sensor
	pub physics: String,
}

impl Default for Sensor {
    fn default() -> Sensor {
        Sensor {
            model: String::new(),
            sens_type: String::new(),
            physics: String::new(),
            accuracy: 0.0_f32,
        }
    }
}

/// Meteo specific header fields
#[derive(Debug, Clone)]
#[cfg_attr(feature = "with-serde", derive(Serialize, Deserialize))]
pub struct HeaderFields {
	/// Observation types contained in this file
    pub codes: Vec<String>, 
    pub sensors: Vec<Sensor>,
}

/// `Record`: Meteo data files content is
/// raw data sorted by Observation Code and by Epoch.
pub type Record = BTreeMap<epoch::Epoch, HashMap<String, f32>>;

#[derive(Error, Debug)]
/// Meteo Data `Record` parsing specific errors
pub enum RecordError {
    #[error("failed to parse date")]
    ParseDateError(#[from] epoch::ParseDateError),
    #[error("failed to integer number")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("failed to float number")]
    ParseFloatError(#[from] std::num::ParseFloatError),
}

/// Builds `Record` entry for `MeteoData`
pub fn build_record_entry (header: &Header, content: &str) 
        -> Result<(epoch::Epoch, HashMap<String, f32>), RecordError> 
{
    let mut lines = content.lines();
    let mut line = lines.next()
        .unwrap();

	let mut map : HashMap<String, f32> = HashMap::with_capacity(3);

	// epoch.secs is not f32 as usual
	// Y is 4 digit number as usual for V > 2
	//let (date, rem) = line.split_at(offset);
	let (mut y, m, d, h, min, sec, mut offset) : (i32, u32, u32, u32, u32, u32, usize) 
		= match header.version.major > 2 {
		true => {
			(i32::from_str_radix(line[0..5].trim(),10)?, // Y: 4 digit
			u32::from_str_radix(line[5..8].trim(),10)?, // m
			u32::from_str_radix(line[8..11].trim(),10)?, // d
			u32::from_str_radix(line[11..14].trim(),10)?, // h
			u32::from_str_radix(line[14..17].trim(),10)?, // m
			u32::from_str_radix(line[17..20].trim(),10)?, // s
			20)
		},
		false => {
			(i32::from_str_radix(line[0..3].trim(),10)?, // Y: 2 digit
			u32::from_str_radix(line[3..6].trim(),10)?, // m
			u32::from_str_radix(line[6..9].trim(),10)?,// d
			u32::from_str_radix(line[9..12].trim(),10)?,// h
			u32::from_str_radix(line[12..15].trim(),10)?,// m
			u32::from_str_radix(line[15..18].trim(),10)?,// s
			18)
		},
	};
	if y < 100 { // 2 digit nb case
    	if y > 90 {
        	y += 1900
    	} else {
			y += 2000
		}
	}
	let date = chrono::NaiveDate::from_ymd(y,m,d)
		.and_hms(h,min,sec);
	let flag = epoch::EpochFlag::default();
	let epoch = epoch::Epoch::new(date, flag);

	let codes = &header.meteo
        .as_ref()
        .unwrap()
        .codes;
	let n_codes = codes.len();
	let nb_lines : usize = num_integer::div_ceil(n_codes, 8).into(); 
	let mut code_index : usize = 0;

	for i in 0..nb_lines {
		for _ in 0..8 {
			let code = &codes[code_index];
			let obs : Option<f32> = match f32::from_str(&line[offset..offset+7].trim()) {
				Ok(f) => Some(f),
				Err(_) => None,
			};

			if let Some(obs) = obs {
				map.insert(code.to_string(), obs); 
			}
			code_index += 1;
			if code_index >= n_codes {
				break
			}

			offset += 7;
			if offset >= line.len() {
				break
			}
		} // 1:8

		if i < nb_lines-1 {
			if let Some(l) = lines.next() {
				line = l;
			} else {
				break
			}
		}
	} // nb lines
	Ok((epoch, map))
}

/// Pushes meteo record into given file writer
pub fn to_file (header: &header::Header, record: &Record, mut writer: std::fs::File) -> std::io::Result<()> {
    let codes = &header.meteo
        .as_ref()
        .unwrap()
        .codes;
    for epoch in record.keys() {
        write!(writer, " {} ", epoch.date.format("%y %_m %_d %_H %_M %_S").to_string())?;
        for code in codes.iter() { 
            write!(writer, "{:.1}   ", record[epoch].get(code).unwrap())?;
        }
        write!(writer, "\n")?
    }
    Ok(())
}
