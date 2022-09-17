Roadmap 
=======

## RINEX library

- [ ] `epoch` : `EpochFlag::HeaderInformationFollows` is not exploited to this day.  
We might want to update the Header structure, on the fly, with following information

- [ ] `navigation` - `dictionary`: currently only supports floating point and string data to be identified and parsed.
  - [ ] we should allow other native types like boolean or integer numbers, as they are also specified in RINEX standards (at least of the latter)
  - [ ] an interface for `bitflags!` mapping would be ideal for Binary fields
  - [ ] an interface for complex and custom enums mapping would be also ideal and help data analysis 

- [ ] `sampling`: `chrono::duration` is used most of the time to describe a duration.  
The fractional parts ("nanos") is totally unused, we cannot handle periods smaller than 1 second to this day

- [ ] Data production
  - [ ] Find an efficient data production test method (`rinex/tests/production.rs`).   
  `CRX2RNX` test bench is based on official versus generated file comparison
  using "diff -z" (sort of bitwise comparison). We can't use this option in case of data production,
  because header fields order of appearance are very likely to differ.
  - [ ]  Major data production
    - [ ] Observation data production
    - [ ] Navigation data production
  - [ ] Minor data production
    - [ ] Clock data production 
    - [ ] Ionosphere maps production   
    - [ ] Antenna data production 

- [ ] Data compression
  - [ ] Conclude [numerical data compression](https://github.com/gwbres/rinex/blob/main/rinex/src/hatanaka.rs#L164)
  - [ ] Conclude [text data compression](https://github.com/gwbres/rinex/blob/main/rinex/src/hatanaka.rs#L209)
  - [x] Provide a Writer wrapper in similar fashion to existing Reader wrapper for efficient data compression
  - [x] Adjust production method to take advantage of newly available Writer wrapper
  - [ ] Enhance Buffered writer with `Hatanaka` compression
  - [ ] Unlock `CRINEX` data production
  - [ ] `Gzip` decompression failure: understand current issue regarding files marked for `Post Processing`, 
track [opened issue](https://github.com/rust-lang/flate2-rs/issues/316)

- [ ] Post Processing
  - [ ] Conclude the 2D Post processing "double diff"
    - [ ] A NAV + OBS context structure could help ?   
    this is currently inquired in the `differential` branch
  - [ ] Calculations involved in RTK solver? I am not familiar with such calculations

## Command Line application

- [ ] CLI
  - [ ] expose remaining interesting methods ?
  - [ ] conclude the `teqc` mini ascii plot 
  - [ ] Find an efficient method to customize header fields
- [ ]  Data production
  - [ ] provide some interface to efficiently customize the Header section
  - [ ] provide an efficient interface to manage file names to be generated 
- [ ]  Post Processing
  - [ ]  provide efficient interface to 1D and 2D processing methods  
- [ ] Graphical Interface
  - [ ] Provide a visualization method when we're not generating a file
  - [ ] Inquire which framework would be ideal: not too complex, full of features
  - [ ] GUI must be an application feature, for users not interested in such option

## UBLOX application

- [ ] Have an header field attributes customization interface similar to `cli` application
- [ ] Generate Observation Data (requires `observation::to_file` to be completed)
- [ ] Generate Ephemeris Data (requires `navigation::to_file` to be completed)

## Done

- [x] Rinex Post Processing
  - [x] 1D post processing [1D diff()](https://github.com/gwbres/rinex/blob/main/rinex/src/lib.rs#L3023) 
