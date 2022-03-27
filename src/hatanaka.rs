//! hatanaka.rs   
//! structures and macros suites for
//! RINEX OBS files compression and decompression.   
use crate::Type;
use crate::header;
use crate::record;
use crate::is_comment;
use thiserror::Error;
use std::str::FromStr;
use crate::record::Sv;
use std::collections::HashMap;

#[derive(Error, Debug)]
/// Hatanaka Kernel, compression
/// and decompression related errors
pub enum KernelError {
    #[error("order cannot be greater than {0}")]
    OrderTooBig(usize),
    #[error("cannot recover a different type than init type!")]
    TypeMismatch,
}

/// `Dtype` describes the type of data
/// we can compress / decompress
#[derive(Clone, Debug)]
pub enum Dtype {
    /// Numerical is intended to be used    
    /// for observation data and clock offsets
    Numerical(i64),
    /// Text represents epoch descriptors,   
    /// GNSS descriptors,    
    /// observation flags..
    Text(String),
}

impl Default for Dtype {
    fn default() -> Dtype { Dtype::Numerical(0) }
 }

impl Dtype {
    pub fn as_numerical (&self) -> Option<i64> {
        match self {
            Dtype::Numerical(n) => Some(n.clone()),
            _ => None,
        }
    }
    pub fn as_text (&self) -> Option<String> {
        match self {
            Dtype::Text(s) => Some(s.to_string()),
            _ => None,
        }
    }
}

/// Fills given i64 vector with '0'
fn zeros (array : &mut Vec<i64>) {
    for i in 0..array.len() { array[i] = 0 }
}

/// `Kernel` is a structure to compress    
/// or recover data using recursive defferential     
/// equations as defined by Y. Hatanaka.   
/// No compression limitations but maximal order   
/// to be supported must be defined on structure     
/// creation for memory allocation efficiency.
#[derive(Debug, Clone)]
pub struct Kernel {
    /// internal counter
    n: usize,
    /// compression order
    order: usize,
    /// kernel initializer
    init: Dtype,
    /// state vector 
    state: Vec<i64>,
    /// previous state vector 
    p_state: Vec<i64>,
}

impl Kernel {
    /// Builds a new kernel structure.    
    /// m: maximal Hatanaka order for this kernel to ever support,    
    ///    m=5 is hardcoded in CRN2RNX official tool
    pub fn new (m: usize) -> Kernel {
        let mut state : Vec<i64> = Vec::with_capacity(m+1);
        for _ in 0..m+1 { state.push(0) }
        Kernel {
            n: 0,
            order: 0,
            init: Dtype::default(),
            state: state.clone(),
            p_state: state.clone(),
        }
    }

    /// (re)initializes kernel    
    /// order: compression order   
    /// data: kernel initializer
    pub fn init (&mut self, order: usize, data: Dtype) -> Result<(), KernelError> { 
        if order > self.state.len() {
            return Err(KernelError::OrderTooBig(self.state.len()))
        }
        // reset
        self.n = 0;
        zeros(&mut self.state);
        zeros(&mut self.p_state);
        // init
        self.init = data.clone();
        self.p_state[0] = data.as_numerical().unwrap_or(0);
        self.order = order;
        Ok(())
    }

    /// Recovers data by computing
    /// recursive differential equation
    pub fn recover (&mut self, data: Dtype) -> Result<Dtype, KernelError> {
        match data {
            Dtype::Numerical(data) => {
                if let Some(_) = self.init.as_numerical() {
                    Ok(self.numerical_data_recovery(data))
                } else {
                    Err(KernelError::TypeMismatch)
                }
            },
            Dtype::Text(data) => {
                if let Some(_) = self.init.as_text() {
                   Ok(self.text_data_recovery(data))
                } else {
                    Err(KernelError::TypeMismatch)
                }
            },
        }
    }
    /// Runs Differential Equation
    /// as defined in Hatanaka compression method
    fn numerical_data_recovery (&mut self, data: i64) -> Dtype {
        self.n += 1;
        self.n = std::cmp::min(self.n, self.order);
        zeros(&mut self.state);
        self.state[self.n] = data;
        for index in (0..self.n).rev() {
            self.state[index] = 
                self.state[index+1] 
                    + self.p_state[index] 
        }
        self.p_state = self.state.clone();
        Dtype::Numerical(self.state[0])
    }
    /// Text is very simple
    fn text_data_recovery (&mut self, data: String) -> Dtype {
        let mut init = self.init
            .as_text()
            .unwrap();
        let l = init.len();
        let mut recovered = String::from("");
        let mut p = init.as_mut_str().chars();
        let mut data = data.as_str().chars();
        for _ in 0..l {
            let next_c = p.next().unwrap();
            if let Some(c) = data.next() {
                if c == '&' { // special whitespace insertion
                    recovered.push_str(" ")
                } else if c.is_ascii_alphanumeric() {
                    recovered.push_str(&c.to_string())
                } else {
                    recovered.push_str(&next_c.to_string())
                }
            } else {
                recovered.push_str(&next_c.to_string())
            }
        }
        // mask might be longer than self
        // in case we need to extend current value
        loop {
            if let Some(c) = data.next() {
                if c == '&' { // special whitespace insertion
                    recovered.push_str(" ")
                } else if c.is_ascii_alphanumeric() {
                    recovered.push_str(&c.to_string())
                }
            } else {
                break
            }
        }
        self.init = Dtype::Text(recovered.clone()); // for next time
        Dtype::Text(String::from(&recovered))
    }
}

/// Compression / Decompression related errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    IoError(#[from] std::io::Error),
    #[error("this is not a crinex file")]
    NotACrinexError,
    #[error("this is not an observation file")]
    NotObsRinexData,
    #[error("non supported crinex revision")]
    NonSupportedCrinexRevision,
    #[error("CRINEX1 standard mismatch")]
    FaultyCrinex1Format,
    #[error("CRINEX3 standard mismatch")]
    FaultyCrinex3Format,
    #[error("failed to identify sat. vehicule")]
    ParseSvError(#[from] record::ParseSvError),
    #[error("failed to parse integer number")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("data recovery failed")]
    KernelError(#[from] KernelError),
    #[error("data recovery error")]
    DataRecoveryError,
}

/// Structure to decompress a CRINEX file
pub struct Decompressor {
    /// to identify very first few bytes passed
    first_epo : bool,
    /// to determine where we are in the record
    header : bool,
    /// to determine where we are in the record
    clock_offset : bool,
    /// to determine where we are in the record
    pointer : u16,
    /// Epoch decompressor
    epo_krn : Kernel,
    /// Clock offset decompressor
    clk_krn : Kernel,
    /// decompressors
    sv_krn  : HashMap<Sv, Vec<(Kernel, Kernel, Kernel)>>,
}

impl Decompressor {
    /// Creates a new `CRINEX` decompressor tool
    pub fn new (max_order: usize) -> Decompressor {
        Decompressor {
            first_epo : true,
            header : true,
            clock_offset : false,
            pointer : 0,
            epo_krn : Kernel::new(0),
            clk_krn : Kernel::new(max_order),
            sv_krn  : HashMap::new()
        }
    }
    /// Decompresses (recovers) given CRINEX record content.   
    /// Do not feed header data - header must be previously parsed separetely.    
    /// `header` : previously identified RINEX `header` section
    /// `content`: string reference extracted from a CRINEX record.    
    ///           This method is very convenient because `content` can have any shape you can think of.    
    ///           You can feed single CRINEX epoch lines, one at a time.     
    ///           You can pass epochs one at a time (line groupings).   
    ///           You can also pass an entire CRINEX record at once.    
    ///           You can even pass unevenly grouped chunks of epochs, even with COMMENTS unevenly inserted.    
    ///           Just make sure that `content` is never an empty string (at least 1 character),
    ///           which easily happens if you iterate through a CRINEX record with .lines() for instance.   
    ///           And also make sure to follow a standard CRINEX structure, which should always be the case   
    ///           if you iterate over a valid CRINEX.
    /// `result`: returns decompressed (recovered) block from provided block content
    pub fn recover (&mut self, header: &header::Header, content : &str) -> Result<String, Error> {
        // Context sanity checks
        if !header.is_crinex() {
            return Err(Error::NotACrinexError)
        }
        if header.rinex_type != Type::ObservationData {
            return Err(Error::NotObsRinexData)
        }
        // grab useful information for later
        let rnx_version = &header.version;
        let crx_version = header.crinex
            .as_ref()
            .unwrap()
            .version;
        let obs_codes = header.obs_codes
            .as_ref()
            .unwrap();
        // pre defined maximal compression order
        //  ===> to adapt all other kernels accordingly
        let m = self.clk_krn.state.len()-1; 
        let mut result : String = String::new();
        let mut lines = content.lines();
        let mut clock_offset : Option<i64> = None;
        loop {
            let line : &str = match lines.next() {
                Some(l) => l,
                None => break,
            };
            // [0] : COMMENTS
            if is_comment!(line) {
                result.push_str(line); // feed as is..
                result.push_str("\n");
                continue
            }
            // [0*] : special epoch events
            //        with uncompressed descriptor
            if line.starts_with("> ") && !self.first_epo {
                result.push_str(line); // feed as is..
                result.push_str("\n");
                continue
            }
            // [1] recover epoch descriptor 
            if self.header {
                self.recover_epoch_descriptor(crx_version.major, &line)?; 
                self.header = false;
                self.clock_offset = true;
                continue
            };
            // [2] recover clock offset, if any
            if self.clock_offset {
                clock_offset = match line.contains("&") {
                    false => {
                        if let Ok(num) = i64::from_str_radix(line.trim(),10) {
                            Some(num)
                        } else {
                            None // parsing fails on empty line
                        }
                    },
                    true => {
                        // kernel (re)init
                        let (n, rem) = line.split_at(1);
                        let n = u8::from_str_radix(n, 10)?;
                        let (_, num) = rem.split_at(1);
                        let num = i64::from_str_radix(num, 10)?;
                        self.clk_krn.init(
                            n.into(), 
                            Dtype::Numerical(num))
                            .unwrap();
                        Some(num)
                    },
                };
                // we can now fully recover the descriptor line
                let recovered_epoch =  // trick to recover textdiff
                    self.epo_krn.recover(Dtype::Text(String::from(" ")))?
                    .as_text()
                    .unwrap();
                match rnx_version.major {
                    1|2 => { // old RINEX
                        // system # id is appended
                        // and wrapped on as many lines as needed
                        let (epoch, systems) = recovered_epoch.split_at(32);
                        result.push_str(epoch);
                        let mut begin = 0;
                        //while begin < systems.len() {
                        for i in 0..num_integer::div_ceil(systems.len(), 12*3) { //max sv per line
                            let end = std::cmp::min(begin+12*3, systems.len());
                            if i > 0 {
                                result.push_str("                                ")
                            }
                            result.push_str(&systems[begin..end]);
                            result.push_str("\n");
                            begin += 12*3
                        }
                    },
                    _ => { // modern RINEX
                        result.push_str(recovered_epoch.split_at(35).0)
                    }
                };
                if let Some(offset) = clock_offset {
                    result.push_str(&format!("         {:3.12}", (offset as f64)/1000.0_f64))
                }
                result.push_str("\n");
                self.clock_offset = false;
                continue
            }
            // [3] inside epoch content
            let recovered_epoch =  // trick to recover textdiff
                self.epo_krn.recover(Dtype::Text(String::from(" ")))?
                .as_text()
                .unwrap();
            let epo = recovered_epoch.as_str().trim_end();
            println!("EPO : \"{}\"", epo);
            let mut offset : usize =
                2    // Y
                +2+1 // m
                +2+1 // d
                +2+1 // h
                +2+1 // m
                +11  // s
                +1;  // ">" or "&" init marker
            if rnx_version.major > 2 { offset += 2 } // Y is 4 digit
            if epo.starts_with("> ") { offset += 1 } // CRINEX3 has 1 extra whitespace
            let (_, rem) = epo.split_at(offset);
            let (_, rem) = rem.split_at(3); // _ is epoch flag
            let (n, _) = rem.split_at(3);
            let nb_sv = u16::from_str_radix(n.trim(), 10)?;
            //     ---> identify nb of satellite vehicules
            //     ---> identify which system we're dealing with
            //          using recovered header
            //TODO <!> Attention a cet offset 41 en cas de CRINEX1
            let offset : usize = std::cmp::min((41 + 3*(self.pointer+1)).into(), epo.len());
            let system = epo.split_at(offset.into()).0;
            let system = system.split_at(system.len()-3).1; // last 3 XXX
            if rnx_version.major > 2 {
                result.push_str(&system.to_string()); // Modern Rinex Needs this
            }

            let sv = Sv::from_str(system)?;
            let codes = &obs_codes[&sv.constellation];
            if !self.sv_krn.contains_key(&sv) {
                // first time dealing with this system
                // add an entry for each obscode
                let mut v : Vec<(Kernel,Kernel,Kernel)> = Vec::with_capacity(12);
                for _ in codes {
                    let mut kernels = (
                        Kernel::new(m), // OBS
                        Kernel::new(0), // SSI
                        Kernel::new(0), // LLI
                    );
                    // init with BLANK 
                    kernels.1 // LLI
                        .init(0, Dtype::Text(String::from(" "))) // textdiff
                        .unwrap();
                    kernels.2 // SSI
                        .init(0, Dtype::Text(String::from(" "))) // textdiff
                        .unwrap();
                    v.push(kernels)
                }
                self.sv_krn.insert(sv, v); // creates new entry
            }
            
            // try to grab all data,
            // might fail in case it's truncated by compression
            let mut obs_count : usize = 0;
            let mut rem = line.clone();
            let mut obs_data : Vec<Option<i64>> = Vec::with_capacity(12);
            loop {
                if obs_count == codes.len() {
                    // FLAGS fields
                    //  ---> parse & run textdiff on each individual character
                    //   --> then format final output line
                    println!("FLAGS! \"{}\" | {}", rem, rem.len());
                    let mut obs_flags : Vec<(String,String)> = Vec::with_capacity(obs_data.len());
                    // [+] grab all provided and aplly textdiff
                    //     append BLANK in case not provided,
                    //     this approach produces 1 flag (either blank or provided/recovered) 
                    //     to previously provided/recovered OBS data
                    //for i in 0..num_integer::div_ceil(rem.len(), 2) { // {lli,ssi}
                    for i in 0..rem.len() / 2 { // {lli,ssi}
                        let obs = self.sv_krn.get_mut(&sv)
                            .unwrap();
                        obs_flags.push((
                        obs[i]
                            .1 // LLI
                            .recover(Dtype::Text(rem[i*2..i*2+1].to_string()))?
                            .as_text()
                            .unwrap(),
                        obs[i]
                            .2 // SSI
                            .recover(Dtype::Text(rem[i*2+1..i*2+2].to_string()))?
                            .as_text()
                            .unwrap()))
                    }
                    for _ in 0..obs_data.len() - obs_flags.len() {
                        // some flags were not provided
                        // because BLANKED out by compression algorithm
                        obs_flags.push((String::from(" "),String::from(" ")))
                    }
                    for i in 0..obs_data.len() {
                        if let Some(data) = obs_data[i] {
                            // --> data field was found & recovered
                            result.push_str(&format!(" {:13.3}", data as f64 /1000_f64)); // F14.3
                            result.push_str(&obs_flags[i].0); // lli
                            result.push_str(&obs_flags[i].1) // ssi
                        } else {
                            result.push_str("              "); // BLANK data
                            result.push_str(" "); // BLANK lli
                            result.push_str(" "); // BLANK ssi
                        }
                    }
                    result.push_str("\n");
                    break
                }
                let next_wsp = match rem.find(' ') {
                    Some(l_off) => l_off+1,
                    None => { 
                        // line is either terminated by one last compressed code
                        // or empty content
                        // [+] try to parse one last obs 
                        if rem.contains("&") {
                            // kernel (re)init 
                            let index = rem.find("&").unwrap();
                            let (order, rem) = rem.split_at(index);
                            let order = u8::from_str_radix(order.trim(),10)?;
                            let (_, data) = rem.split_at(1);
                            println!("ATTENTION ICI trim ??");
                            let data = i64::from_str_radix(data.trim(), 10)?;
                            let obs = self.sv_krn.get_mut(&sv)
                                .unwrap();
                            obs[obs_count]
                                .0 // OBS
                                .init(
                                    order.into(),
                                    Dtype::Numerical(data))
                                    .unwrap();
                            obs_data.push(Some(data));
                            obs_count += 1
                        } else {
                            // regular compression
                            if let Ok(num) = i64::from_str_radix(rem.trim(),10) {
                                let obs = self.sv_krn.get_mut(&sv)
                                    .unwrap();
                                let recovered = obs[obs_count]
                                    .0 // OBS
                                    .recover(
                                        Dtype::Numerical(num))
                                    .unwrap()
                                    .as_numerical()
                                    .unwrap();
                                obs_data.push(Some(recovered))
                            }
                        }
                        //  --> format this line correctly
                        for i in 0..obs_data.len() {
                            if let Some(data) = obs_data[i] {
                                // --> data field was found & recovered
                                result.push_str(&format!(" {:13.3}", data as f64 /1000_f64)); // F14.3
                                // ---> related flag content
                                let obs = self.sv_krn.get_mut(&sv)
                                    .unwrap();
                                let lli = obs[i]
                                    .1 // LLI
                                    .recover(Dtype::Text(String::from(" "))) // trick to recover
                                    .unwrap()
                                    .as_text()
                                    .unwrap();
                                let ssi = obs[i]
                                    .2 // SSI
                                    .recover(Dtype::Text(String::from(" "))) // trick to recover
                                    .unwrap()
                                    .as_text()
                                    .unwrap();
                                result.push_str(&lli); // FLAG
                                result.push_str(&ssi); // FLAG 
                            } else {
                                result.push_str("              "); // BLANK data
                                result.push_str(" "); // BLANK lli
                                result.push_str(" "); // BLANK ssi
                            }
                        }
                        result.push_str("\n");
                        break // EOL
                    },
                };
                let (roi, r) = rem.split_at(next_wsp);
                rem = r;
                println!("CODE : \"{}\" - ROI \"{}\"", codes[obs_count], roi);
                if roi == " " { // BLANK field
                    obs_count += 1;
                    obs_data.push(None);
                    continue // compressed non existing obs 
                }
                let (init_order, data) : (Option<u16>, i64) = match roi.contains("&") {
                    false => {
                        (None, i64::from_str_radix(roi.trim(),10)?)
                    },
                    true => {
                        let (init_order, remainder) = roi.split_at(1);
                        let (_, data) = remainder.split_at(1);
                        (Some(u16::from_str_radix(init_order, 10)?),
                        i64::from_str_radix(data.trim(), 10)?)
                    },
                };
                if let Some(order) = init_order {
                    //(re)init that kernel
                    let obs = self.sv_krn.get_mut(&sv)
                        .unwrap();
                    obs[obs_count]
                        .0 // OBS
                        .init(
                            order.into(),
                            Dtype::Numerical(data))
                            .unwrap();
                    obs_data.push(Some(data))
                } else {
                    let obs = self.sv_krn.get_mut(&sv)
                        .unwrap();
                    let recovered = obs[obs_count]
                        .0 // OBS
                        .recover(Dtype::Numerical(data))
                        .unwrap();
                    let recovered = recovered
                        .as_numerical()
                        .unwrap();
                    obs_data.push(Some(recovered))
                }
                obs_count +=1
            } // for all OBS
            self.pointer += 1;
            if self.pointer == nb_sv { // nothing else to parse
                self.pointer = 0; // reset
                self.header = true // reset FSM
            }
        }

        Ok(result)
    }
    /// Recovers epoch descriptor from given content
    fn recover_epoch_descriptor (&mut self, revision_major: u8, line: &str) -> Result<String, Error> {
        // CRINEX sanity checks
        if self.first_epo {
            match revision_major {
                1 => {
                    if !line.starts_with("&") {
                        return Err(Error::FaultyCrinex1Format)
                    }
                },
                3 => {
                    if !line.starts_with("> ") {
                        return Err(Error::FaultyCrinex3Format)
                    }
                },
                _ => return Err(Error::NonSupportedCrinexRevision),
            }
        }
        if self.first_epo {
            self.epo_krn.init( // init this kernel
                0, // is always a textdiff
                Dtype::Text(line.to_string()))?;
            self.first_epo = false
        }
        Ok(self.epo_krn.recover(Dtype::Text(line.to_string()))?
            .as_text()
            .unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    /// Tests numerical data recovery    
    /// through Hatanaka decompression.   
    /// Tests data come from an official CRINEX file and
    /// official CRX2RNX decompression tool
    fn test_num_recovery() {
        let mut krn = Kernel::new(5);
        let init : i64 = 25065408994;
        krn.init(3, Dtype::Numerical(init))
            .unwrap();
        let data : Vec<i64> = vec![
            5918760,
            92440,
            -240,
            -320,
            -160,
            -580,
            360,
            -1380,
            220,
            -140,
        ];
        let expected : Vec<i64> = vec![
            25071327754,
            25077338954,
            25083442354,
            25089637634,
            25095924634,
            25102302774,
            25108772414,
            25115332174,
            25121982274,
            25128722574,
        ];
        for i in 0..data.len() {
            let recovered = krn.recover(Dtype::Numerical(data[i]))
                .unwrap()
                    .as_numerical()
                    .unwrap();
            assert_eq!(recovered, expected[i]);
        }
        // test re-init
        let init : i64 = 24701300559;
        krn.init(3, Dtype::Numerical(init))
            .unwrap();
        let data : Vec<i64> = vec![
            -19542118,
            29235,
            -38,
            1592,
            -931,
            645,
            1001,
            -1038,
            2198,
            -2679,
            2804,
            -892,
        ];
        let expected : Vec<i64> = vec![
            24681758441,
            24662245558,
            24642761872,
            24623308975,
            24603885936,
            24584493400,
            24565132368,
            24545801802,
            24526503900,
            24507235983,
            24488000855,
            24468797624,
        ];
        for i in 0..data.len() {
            let recovered = krn.recover(Dtype::Numerical(data[i]))
                .unwrap()
                    .as_numerical()
                    .unwrap();
            assert_eq!(recovered, expected[i]);
        }
    }
    #[test]
    /// Tests Hatanaka Text Recovery algorithm
    fn test_text_recovery() {
        let init = "ABCDEFG 12 000 33 XXACQmpLf";
        let mut krn = Kernel::new(5);
        let masks : Vec<&str> = vec![
            "        13   1 44 xxACq   F",
            " 11 22   x   0 4  y     p  ",
            "              1     ",
            "                   z",
            " ",
        ];
        let expected : Vec<&str> = vec![
            "ABCDEFG 13 001 44 xxACqmpLF",
            "A11D22G 1x 000 44 yxACqmpLF",
            "A11D22G 1x 000144 yxACqmpLF",
            "A11D22G 1x 000144 yzACqmpLF",
            "A11D22G 1x 000144 yzACqmpLF",
        ];
        krn.init(3, Dtype::Text(String::from(init)))
            .unwrap();
        for i in 0..masks.len() {
            let mask = masks[i];
            let result = krn.recover(Dtype::Text(String::from(mask)))
                .unwrap()
                    .as_text()
                    .unwrap();
            assert_eq!(result, String::from(expected[i]));
        }
        // test re-init
        let init = " 2200 123      G 07G08G09G   XX XX";
        krn.init(3, Dtype::Text(String::from(init)))
            .unwrap();
        let masks : Vec<&str> = vec![
            "        F       1  3",
            " x    1 f  f   p",
            " ",
            "  3       4       ",
        ];
        let expected : Vec<&str> = vec![
            " 2200 12F      G107308G09G   XX XX",
            " x200 12f  f   p107308G09G   XX XX",
            " x200 12f  f   p107308G09G   XX XX",
            " x300 12f 4f   p107308G09G   XX XX",
        ];
        for i in 0..masks.len() {
            let mask = masks[i];
            let result = krn.recover(Dtype::Text(String::from(mask)))
                .unwrap()
                    .as_text()
                    .unwrap();
            assert_eq!(result, String::from(expected[i]));
        }
    }
}
