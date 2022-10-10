//! Command line tool to parse and analyze `RINEX` files.    
//! Refer to README for command line arguments.    
//! Based on crate <https://github.com/gwbres/rinex>     
//! Homepage: <https://github.com/gwbres/rinex-cli>
use clap::App;
use clap::AppSettings;
use clap::load_yaml;
use std::str::FromStr;
use plotters::prelude::*;
use plotters::coord::{
    types::RangedCoordf64,
    cartesian::Cartesian3d,
};
use std::collections::HashMap;

use rinex::{*, 
    observation::Ssi, observation::LliFlags,
};

mod parser; // user input parser
mod ascii_plot; // `teqc` tiny plot

/* NOTES
 * smart color generation
 * chart
 *      .draw_series()
 *      .style_func(&|&v| {&HLSColor(x).into())
 */

/// Resample given file as possibly requested
fn resample_single_file (rnx: &mut Rinex, matches: clap::ArgMatches) {
    if let Some(hms) = matches.value_of("decim-interval") { 
        if let Ok(interval) = parser::parse_duration(hms) {
            rnx
                .decimate_by_interval_mut(interval)
        }
    }
    if let Some(r) = matches.value_of("decim-ratio") {
        if let Ok(r) = u32::from_str_radix(r, 10) {
            rnx
                .decimate_by_ratio_mut(r)
        }
    }
}

/// Apply desired filters
fn apply_filters (rinex: &mut Rinex, matches: clap::ArgMatches) {
    let epoch_ok_filter = matches.is_present("epoch-ok-filter");
    let epoch_nok_filter = matches.is_present("epoch-nok-filter");
    
    let constell_filter : Option<Vec<Constellation>> = match matches.value_of("constellation-filter") {
        Some(s) => {
            let constellations: Vec<&str> = s.split(",").collect();
            let mut c_filters : Vec<Constellation> = Vec::new();
            for c in constellations {
                if let Ok(constell) = Constellation::from_3_letter_code(c) {
                    c_filters.push(constell)
                } else if let Ok(constell) = Constellation::from_1_letter_code(c) {
                    c_filters.push(constell)
                }
            }
            Some(c_filters)
        },
        _ => None,
    };
    
    let sv_filter : Option<Vec<Sv>> = match matches.value_of("sv-filter") {
        Some(s) => {
            let sv: Vec<&str> = s.split(",").collect();
            let mut sv_filters : Vec<Sv> = Vec::new();
            for s in sv {
                let constell = Constellation::from_str(&s[0..1])
                    .unwrap();
                let prn = u8::from_str_radix(&s[1..], 10)
                    .unwrap();
                sv_filters.push(Sv::new(constell,prn))
            }
            Some(sv_filters)
        },
        _ => None,
    };

    let observ_filter : Option<Vec<&str>> = match matches.value_of("observ-filter") {
        Some(s) => Some(s.split(",").collect()),
        _ => None,
    };
    let lli_mask : Option<u8> = match matches.value_of("lli-mask") {
        Some(s) => Some(u8::from_str_radix(s,10).unwrap()),
        _ => None,
    };
    let ssi_filter : Option<Ssi> = match matches.value_of("ssi-filter") {
        Some(s) => Some(Ssi::from_str(s).unwrap()),
        _ => None,
    };
    
    if epoch_ok_filter {
        rinex
            .epoch_ok_filter_mut()
    }
    if epoch_nok_filter {
        rinex
            .epoch_nok_filter_mut()
    }
    if let Some(ref filter) = constell_filter {
        rinex
            .constellation_filter_mut(filter.to_vec())
    }
    if let Some(ref filter) = sv_filter {
        rinex
            .space_vehicule_filter_mut(filter.to_vec())
    }
    if let Some(ref filter) = observ_filter {
        rinex
            .observable_filter_mut(filter.to_vec())
    }
    if let Some(lli) = lli_mask {
        let mask = LliFlags::from_bits(lli)
            .unwrap();
        rinex
            .lli_filter_mut(mask)
    }
    if let Some(ssi) = ssi_filter {
        rinex
            .minimum_sig_strength_filter_mut(ssi)
    }
}

/// Execute user requests on a single file
fn run_single_file_op (
    rnx: &Rinex, 
    matches: clap::ArgMatches, 
    output: Option<&str>)
{
    let plot = matches.is_present("plot");
    let pretty = matches.is_present("pretty");
    let header = matches.is_present("header");
    let observables = matches.is_present("observ");
    let epoch = matches.is_present("epoch");
    let sv = matches.is_present("sv");
    let ssi_range = matches.is_present("ssi-range");
    let constellations = matches.is_present("constellations");
    let sv_per_epoch = matches.is_present("sv-per-epoch");
    let clock_offsets = matches.is_present("clock-offsets");
    let gaps = matches.is_present("gaps");
    let largest_gap = matches.is_present("largest-gap");
    let _sampling_interval = matches.is_present("sampling-interval");
    let cycle_slips = matches.is_present("cycle-slips");

    let mut at_least_one_op = false;

    if header {
        at_least_one_op = true;
        if pretty {
            println!("{}", serde_json::to_string_pretty(&rnx.header).unwrap())
        } else {
            println!("{}", serde_json::to_string_pretty(&rnx.header).unwrap())
        }
    }
    if epoch {
        at_least_one_op = true;
        if pretty {
            println!("{}", serde_json::to_string_pretty(&rnx.epochs()).unwrap())
        } else {
            println!("{}", serde_json::to_string(&rnx.epochs()).unwrap())
        }
    }
    if observables {
        at_least_one_op = true;
        if pretty {
            println!("{}", serde_json::to_string_pretty(&rnx.observables()).unwrap())
        } else {
            println!("{}", serde_json::to_string(&rnx.observables()).unwrap())
        }
    }
    if constellations {
        at_least_one_op = true;
        if pretty {
            println!("{}", serde_json::to_string_pretty(&rnx.constellations()).unwrap())
        } else {
            println!("{}", serde_json::to_string(&rnx.constellations()).unwrap())
        }
    }
    if sv {
        at_least_one_op = true;
        if pretty {
            println!("{}", serde_json::to_string_pretty(&rnx.space_vehicules()).unwrap())
        } else {
            println!("{}", serde_json::to_string(&rnx.space_vehicules()).unwrap())
        }
    }
    if ssi_range {
        at_least_one_op = true;
        // terminal ouput
        if pretty {
            println!("{}", serde_json::to_string_pretty(&rnx.sig_strength_range()).unwrap())
        } else {
            println!("{}", serde_json::to_string(&rnx.sig_strength_range()).unwrap())
        }
    }
    if clock_offsets {
        at_least_one_op = true;
        if pretty {
            println!("{}", serde_json::to_string_pretty(&rnx.receiver_clock_offsets()).unwrap())
        } else {
            println!("{}", serde_json::to_string(&rnx.receiver_clock_offsets()).unwrap())
        }
    }
    if sv_per_epoch {
        at_least_one_op = true;
        if pretty {
        //    println!("{}", serde_json::to_string_pretty(&rnx.space_vehicules_per_epoch()).unwrap())
        } else {
        //    println!("{}", serde_json::to_string(&rnx.space_vehicules_per_epoch()).unwrap())
        }
    }
    if gaps {
        at_least_one_op = true;
        println!("{:#?}", rnx.data_gaps());
    }
    if largest_gap {
        at_least_one_op = true;
        println!("{:#?}", rnx.largest_data_gap_duration());
    }

    if cycle_slips {
        at_least_one_op = true;
        println!("{:#?}", rnx.cycle_slips());
    }

    if !at_least_one_op {
        // user did not request one of the high level ops
        // ==> either print or plot record data
        if plot { // visualization requested
            let record = &rnx.record;
            if let Some(record) = record.as_obs() {
                // Observation viewer
                let observables = &rnx
                    .header
                    .obs
                    .as_ref()
                    .unwrap()
                    .codes;
                // image bg
                let root = BitMapBackend::new(
                    "obs.png",
                    (1024,768)) //TODO Cli::(x_width,y_height)
                    .into_drawing_area();
                root.fill(&WHITE)
                    .unwrap();
                
                // x axis
                let e0 = rnx.first_epoch().unwrap();
                let timestamps: Vec<_> = record.iter()
                    .map(|(epoch, _)| {
                        (epoch.date.timestamp() - e0.date.timestamp()) as f64
                    })
                    .collect();
                let x_axis = (timestamps[0]..timestamps[timestamps.len()-1]);

                // determine (min, max) #PRN 
                //  this is used to adapt colors nicely 
                let (mut min_prn, mut max_prn) = (100, 0);   
                let space_vehicules = rnx.space_vehicules();
                for sv in space_vehicules.iter() {
                    max_prn = std::cmp::max(max_prn, sv.prn);
                    min_prn = std::cmp::min(min_prn, sv.prn);
                }

                // determine (min, max) per Observation Kind
                //   this is used to scale Y axis nicely
                let mut y_min_max: HashMap<String, (f64,f64)> = HashMap::with_capacity(4); // 4 physics known
                for (_, (_, vehicules)) in record.iter() {
                    for (_, observables) in vehicules.iter() {
                        for (code, data) in observables.iter() {
                            if code == "L1" { 
                                if let Some((min,max)) = y_min_max.get_mut("PR") {
                                    if *min > data.obs {
                                        *min = data.obs ;
                                    }
                                    if *max < data.obs {
                                        *max = data.obs ;
                                    }
                                } else {
                                    y_min_max.insert("PR".to_string(), (data.obs, data.obs));
                                }
                            }
                        }
                    }
                }

                //TODO DEBUG
                println!("YMINMAX {:?}", y_min_max);

                // Create a chart per observable kind
                //let mut charts: HashMap<String, 
                //    ChartContext<BitMapBackend, Cartesian2d<RangedCoordf64, RangedCoordf64>>>
                //    = HashMap::with_capacity(4); // 4 different kinds known
                // build y axis
                let (min, max) = y_min_max.get("PR")
                    .unwrap();
                let y_axis = (min-1000.0..max+1000.0);
                // Create a chart
                let mut chart = ChartBuilder::on(&root)
                    .caption("Pseudo Range", ("sans-serif", 50).into_font())
                    .margin(40)
                    .x_label_area_size(30)
                    .y_label_area_size(40)
                    .build_cartesian_2d(
                        x_axis,
                        y_axis)
                    .unwrap();
                // Draw axes
                chart
                    .configure_mesh()
                    .x_desc("Timestamp")
                    .x_labels(30)
                    //.y_label_formatter(&|y| format!("{:02}:{:02}", y.num_minutes(), y.num_seconds() % 60))
                    .y_desc("PR")
                    .y_labels(30)
                    .draw()
                    .unwrap();

                // symbol per carrier
                let symbols = vec!["x","t","o","p"];
                
                // Draw data series
                for (sv_index, sv) in space_vehicules.iter().enumerate() {
                    // one serie per vehicule
                    for (c_index, (constell, observables)) in observables.iter().enumerate() {
                        if constell == &sv.constellation {
                            for observable in observables.iter() {
                                // one chart per obs kind
                                if observable == "L1" {
                                    //<o
                                    //  symbol emphasizes Carrier Signal 
                                    //  color emphsiazes PRN# 
                                    //    color can also be slightly adjusted regarding the presence
                                    //    and value of SSI 
                                    //let color = Palette99::pick(sv_index * obscodes.len())
                                    //    .mix(0.9); //opacity
                                    chart.draw_series(LineSeries::new(
                                        record.iter()
                                            .map(|(epoch, (_, vehicules))| {
                                                vehicules.iter()
                                                    .filter_map(|(vehicule, observables)| {
                                                        if vehicule.constellation == sv.constellation {
                                                            Some(observables.iter()
                                                                .filter_map(|(observable, observation)| {
                                                                    if observable == "L1" {
                                                                        Some((
                                                                            (epoch.date.timestamp() - e0.date.timestamp()) as f64, //x
                                                                             observation.obs))
                                                                    } else {
                                                                        None
                                                                    }
                                                                }))
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                    .flatten()
                                            })
                                            .flatten(),
                                        //&color,
                                        &BLACK,
                                    ))
                                    .unwrap()
                                    .label("L1");
                                }
                            }//L1
                        } // got some obs for desired constellation
                    } // observables iteration
                } // Sv iteration
                
                // Draw Labels & Legend
                //for (_, chart) in charts.iter_mut() {
                    chart
                        .configure_series_labels()
                        .border_style(&BLACK)
                        .draw()
                        .unwrap();
                //}
            } // Observation viewer
        } else {
            // terminal output
            if pretty {
                println!("{}", serde_json::to_string_pretty(&rnx.record).unwrap())
            } else {
                println!("{}", serde_json::to_string(&rnx.record).unwrap())
            }
        }
    }
}

/// Execute `teqc` ops on a single file
fn run_single_file_teqc_op (rnx: &Rinex, matches: clap::ArgMatches) {
    let ascii_plot = matches.is_present("ascii-plot");
    let _split = matches.is_present("split");
    let _split_epoch : Option<Epoch> = match matches.value_of("split") {
        Some(s) => {
            if let Ok(e) = parser::parse_epoch(s) {
                Some(e)
            } else {
                None
            }
        },
        None => None,
    };
    if ascii_plot {
        println!("{}", ascii_plot::ascii_plot(ascii_plot::DEFAULT_X_WIDTH, &rnx, None));
    }
}

/// Execute user requests on two files
fn run_double_file_op (
    rnx_a: &Rinex, 
    rnx_b: &Rinex, 
    matches: clap::ArgMatches,
    output: Option<&str>) {
    let pretty = matches.is_present("pretty");
    let merge = matches.is_present("merge");
    let diff = matches.is_present("diff");

    if diff {
        if let Ok(rnx) = rnx_a.diff(rnx_b) {
            // print remaining record data
            if pretty {
                println!("{}", serde_json::to_string_pretty(&rnx.record).unwrap())
            } else {
                println!("{}", serde_json::to_string(&rnx.record).unwrap())
            }
        } 
    }
    if merge {
        if let Ok(rnx_c) = rnx_a.merge(rnx_b) {
            if rnx_c.to_file("merge.rnx").is_err() {
                panic!("failed to generate new file");
            }
        } else {
            panic!("merge() failed");
        }
    }
}

pub fn main () -> Result<(), std::io::Error> {
	let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml)
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::DeriveDisplayOrder);
	let matches = app.get_matches();

    // General 
    let plot = matches.is_present("plot");
    let mut custom_header: Option<Header> = None;

    if matches.is_present("header-json") {
        let descriptor = matches.value_of("header-json")
            .unwrap();
        match serde_json::from_str::<Header>(descriptor) {
            Ok(hd) => {
                custom_header = Some(hd.clone());
            },
            Err(e) => {
                match std::fs::read_to_string(descriptor) {
                    Ok(content) => {
                        match serde_json::from_str::<Header>(&content) {
                            Ok(hd) => custom_header = Some(hd.clone()),
                            Err(ee) => panic!("failed to interprate header: {:?}", ee),
                        }
                    },
                    Err(_) => {
                        panic!("failed to interprate header: {:?}", e)
                    }
                }
            },
        }
    }

    // files (in)
    let filepaths : Option<Vec<&str>> = match matches.is_present("filepath") {
        true => {
            Some(matches.value_of("filepath")
                .unwrap()
                    .split(",")
                    .collect())
        },
        false => None,
    };
    // files (out)
    let outputs: Vec<&str> = match matches.is_present("output") {
        true => {
            matches.value_of("output")
                .unwrap()
                    .split(",")
                    .collect()
        },
        false => Vec::new(),
    };

    let filepaths = filepaths
        .unwrap(); // input files are mandatory
    // work queue, contains all parsed RINEX
    let mut queue: Vec<Rinex> = Vec::new();

    ////////////////////////////////////////
    // Parse, filter, resample
    ////////////////////////////////////////
    for fp in &filepaths {
        let path = std::path::PathBuf::from(fp);
        let mut rinex = match path.exists() {
            true => {
                if let Ok(r) = Rinex::from_file(fp) {
                    r
                } else {
                    println!("Failed to parse file \"{}\"", fp); 
                    continue
                }
            },
            false => {
                println!("File \"{}\" does not exist", fp);
                continue
            },
        };
        resample_single_file(&mut rinex, matches.clone());
        apply_filters(&mut rinex, matches.clone());
        queue.push(rinex);
    }

    let mut target_is_single_file = true;
    // Merge is a 2=>1 op
    target_is_single_file &= !matches.is_present("merge");
    // Diff and DDiff are 2=>1 ops
    target_is_single_file &= !matches.is_present("diff");
    target_is_single_file &= !matches.is_present("ddiff");
    
    /////////////////////////////////////
    // ops that require only 1 file
    /////////////////////////////////////
    if target_is_single_file {
        for i in 0..queue.len() {
            let output = outputs.get(i);
            run_single_file_op(&queue[i], matches.clone(), output.copied());
            run_single_file_teqc_op(&queue[i], matches.clone());
        }
    }

    /////////////////////////////////////
    // ops that require 2 files
    /////////////////////////////////////
    if !target_is_single_file { // User requested a 2D op
        for i in 0..queue.len()/2 {
            let q_2p = &queue[i*2];
            let q_2p1 = &queue[i*2+1]; 
            let output = outputs.get(i);
            run_double_file_op(&q_2p, &q_2p1, matches.clone(), output.copied());
        }
    }

    /*let pretty = matches.is_present("pretty");

    // `ddiff` special ops,
    // is processed at very last, because it will eventuelly drop
    // all non Observation RINEX.
    // This requires 2 OBS and 1 NAV files
    if matches.is_present("ddiff") {
        let mut nav : Option<Rinex> = None;
        // tries to identify a NAV file in provided list 
        // this stupidly grabs the first one encountered
        for i in 0..queue.len() {
            if queue[i].is_navigation_rinex() {
                nav = Some(queue[i].clone());
            }
        }
        // 
        if let Some(nav) = nav { // got something
            // drop all other RNX
            queue.retain(|q| q.is_observation_rinex());
            // --> apply `ddiff` related to this NAV
            //     on each remaining --file a,b duplets
            for i in 0..queue.len() /2 {
                let q_2p = &queue[i*2];
                let q_2p1 = &queue[i*2+1];
                let ddiff = q_2p.double_diff(q_2p1, &nav);
                if ddiff.is_ok() {
                    // currently just prints the record
                    // but we'll unlock plotting in next releases
                    let rnx = ddiff.unwrap();
                    let rec = rnx.record.as_obs().unwrap();
                    if pretty {
                        println!("{}", serde_json::to_string_pretty(&rec).unwrap())
                    } else {
                        println!("{}", serde_json::to_string(&rec).unwrap())
                    }
                } else {
                    panic!("--ddiff panic'ed with {:?}", ddiff);
                }
            }
        } else {
            panic!("--ddiff requires NAV ephemeris to be provided!");
        }
    }*/
    
    Ok(())
}// main
