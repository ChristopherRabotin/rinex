use rinex::sv;
use rinex::epoch;
use rinex::types::Type;
use rinex::constellation::Constellation;

fn main() {
    println!("**************************");
    println!("   (OBS) RINEX example    ");
    println!("**************************");

    // example file
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/OBS/V3/ALAC00ESP_R_20220090000_01D_30S_MO.rnx";
    let rinex = rinex::Rinex::from_file(&path).unwrap();

    // header information
    assert_eq!(rinex.header.is_crinex(), false);
    assert_eq!(rinex.header.rinex_type, Type::ObservationData);
    assert_eq!(rinex.header.version.major, 3);
    assert_eq!(rinex.header.constellation, Some(Constellation::Mixed)); 
    // leap second field for instance
    // is a major > 3 optionnal field
    assert_eq!(rinex.header.leap.is_some(), true);

    // (OBS) record analysis
    let record = rinex.record
        .as_obs()
        .unwrap();
	
	//////////////////////////////
    // basic record browsing
    //////////////////////////////
    for entry in record.iter() { // over all epochs
        let epoch = entry.0;
        println!("Found epoch: `{:#?}`", epoch); 
        // epochs are 2D (1 per epoch)
        //   clock offsets (if any) : Some(f32)
        //   HashMap<Sv, HashMap<String, ObservationData>> 
        //   : list of observation data, indexed by Observation Code
        //     and sorted by Satellite Vehicule
        /*for vehicule in entry.1.iter() { // over all sat. vehicules
            let sv = vehicule.0;
            println!("Found sat vehicule: `{:#?}`", sv); 
            let data = vehicule.1;
            println!("Data: `{:#?}`", data); 
        }*/
    }
    //////////////////////////////////////
    // basic hashmap [indexing] 
    // is a quick method to grab some data
    //////////////////////////////////////

    // match a specific `epoch`
    //  * `epoch` is a chrono::NaiveDateTime alias
    //     therefore one can use any chrono::NaiveDateTime method
    let to_match = epoch::Epoch::new(
        epoch::str2date("22 01 09 00 00 00").unwrap(),
        epoch::EpochFlag::Ok);
    //    ---> retrieve all data for desired `epoch`
    //         using direct hashmap[indexing]
    let matched = &record[&to_match];
    println!("\n------------- Matching epoch \"{:?}\" ----------\n{:#?}", to_match, matched); 

	// ----> determine available observation codes
	//       for Glonass system
	let obs_codes = &rinex.header.obs_codes
		.unwrap()
		[&Constellation::GPS];
	println!("\n----------- OBS codes for {} system-------\n{:#?}", Constellation::GPS.to_3_letter_code(), obs_codes);
    
    // ----> zoom in on `G01` vehicule for that particular `epoch` 
    let to_match = sv::Sv::new(Constellation::GPS, 01);
    //let matched = &matched[&to_match];
    println!("\n------------- Adding Sv filter \"{:?}\" to previous epoch filter ----------\n{:#?}", to_match, matched); 
    // ----> grab `R24` "C1C" phase observation for that  `epoch`
    //let matched = &matched["C1C"];
    println!("\n------------- \"C1C\" data from previous set ----------\n{:#?}", matched); 
    
    ///////////////////////////////////////////////////
    // advanced:
    // iterators + filters allow complex
    // pattern matching, data filtering and extraction
    ///////////////////////////////////////////////////
    let record = rinex.record
        .as_obs()
        .unwrap();

    // list all epochs
    let epochs: Vec<_> = record
        .keys()
        .map(|k| k.date)
        .collect();
    println!("\n------------- Epochs ----------\n{:#?}", epochs); 
    
    // Build OBS record that contains only Pseudo Range measurements 
    // --> use provided macro to test each obscode and retain only matching data
    let epochs : Vec<_> = record
        .iter()
        .map(|(_epoch, (_clock_offset, sv))| { // record: {key: epochs, values: (array of clock offsets, array of sv data) }
            sv.iter()
                .map(|(_sv, obs)| { // array of sv data: {key: sv, values: array of data)
                    obs.iter()
                        .find(|(code, data)| { // array of data: {key: OBS code, values: ObsData}
                            rinex::is_pseudo_range_obs_code!(code)
                        })
              })
        })
        .collect();
    println!("\n------------- Epochs with PSEUDO RANGE only ----------\n{:#?}", epochs); 

    // Build array of (epoch, Data) where Data is Pseudo Range data only, 
    // for a particuliar vehicule
    let to_match = sv::Sv::new(Constellation::Galileo, 2); // E02
    let data : Vec<_> = record
        .iter()
        .map(|(epoch, (_, sv))| { // record: {key: epochs, values: (array of clock offsets, array of sv) }
            sv.iter()
                .find(|(k, v)| *k == &to_match) // match unique vehicule 
                .map(|(_, obs)| { // from filtered content, apply previous filter
                    obs.iter()
                        .find(|(code, _)| { // obs code kind filter
                            rinex::is_pseudo_range_obs_code!(code)
                        })
                        .map(|(code, data)| (epoch, code, data)) // build returned struct
                })
                .flatten()
        })
        .flatten()
        .collect();
    println!("\n------------- (timestamp + PSEUDO RANGE data) for {:?} vehicule ----------\n{:#?}", to_match, data); 

    // Build array of (epoch, data) for a particular vehicule and a unique observation code
    let data : Vec<_> = record
        .iter()
        .map(|(epoch, (_, sv))| { // record: {key: epochs, values: (array of clock offsets, array of sv) }
            sv.iter()
                .find(|(k, v)| *k == &to_match) // match unique vehicule 
                .map(|(_, obs)| { // from filtered content, apply previous filter
                    obs.iter()
                        .find(|(code, _)| { // obs code kind filter
                            code.as_str() == "C1C" // unique code 
                        })
                        .map(|(code, data)| (epoch, code, data)) // build returned struct
                })
                .flatten()
        })
        .flatten()
        .collect();
    println!("\n------------- (timestamp + PSEUDO RANGE data) for {:?} vehicule ----------\n{:#?}", to_match, data); 

    // Same idea but retain only `valid` observations,
    // meaning: observation that fit the ObservationData.is_ok() condition,
    // refer to API doc 
    let data : Vec<_> = record
        .iter()
        .map(|(epoch, (_, sv))| { // record: {key: epochs, values: (array of clock offsets, array of sv) }
            sv.iter()
                .find(|(k, v)| *k == &to_match) // match unique vehicule 
                .map(|(_, obs)| { // from filtered content, apply previous filter
                    obs.iter()
                        .find(|(obsCode, obsData)| { // obs code kind filter
                            obsCode.as_str() == "C1C" && obsData.is_ok() // unique code 
                        })
                        .map(|(code, data)| (epoch, code, data)) // build returned struct
                })
                .flatten()
        })
        .flatten()
        .collect();
    println!("\n------------- (timestamp + PSEUDO RANGE data) for {:?} vehicule with trusted/meaningful data ----------\n{:#?}", to_match, data); 
    
    // Grab all doppler data for R24 vehicule that have an LLI + SSI flag attached to it
    // without checking their values 
    // meaning: observation that fit the ObservationData.is_ok() condition,
    // refer to API doc 
    let data : Vec<_> = record
        .iter()
        .map(|(epoch, (_, sv))| { // record: {key: epochs, values: (array of clock offsets, array of sv) }
            sv.iter()
                .find(|(k, v)| *k == &to_match) // match unique vehicule 
                .map(|(_, obs)| { // from filtered content, apply previous filter
                    obs.iter()
                        .find(|(obsCode, obsData)| { // obs code kind filter
                            obsCode.as_str() == "C1C" && obsData.is_ok() // unique code 
                        })
                        .map(|(code, data)| (epoch, code, data)) // build returned struct
                })
                .flatten()
        })
        .flatten()
        .collect();
    println!("\n------------- (timestamp + PSEUDO RANGE data) for {:?} vehicule with trusted/meaningful data ----------\n{:#?}", to_match, data); 

    // Grab (Epoch, ObsCode, ObsData) for all data that have a strong signal quality
    let data : Vec<_> = record
        .iter()
        .map(|(epoch, (_, sv))| { // record: {key: epochs, values: (array of clock offsets, array of sv) }
            sv.iter()
                .find(|(k, v)| *k == &to_match) // match unique vehicule 
                .map(|(_, obs)| { // from filtered content, apply previous filter
                    obs.iter()
                        .find(|(_, obsData)| { // obs code kind filter
                            if obsData.ssi.is_some() {
                                obsData.ssi.unwrap().is_excellent()
                            } else {
                                false
                            }
                        })
                        .map(|(code, data)| (epoch, code, data)) // build returned struct
                })
                .flatten()
        })
        .flatten()
        .collect();
}
