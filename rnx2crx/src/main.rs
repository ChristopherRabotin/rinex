mod cli;
use cli::Cli;
use rinex::{
    Error,
    prelude::*,
    version::Version,
    observation::Crinex,
};
fn main() -> Result<(), Error> {
    let cli = Cli::new(); 
    let input_path = cli.input_path();
    // CRINEX attributes
    let mut crinex = Crinex::default();
    if cli.crx1() {
        crinex.version = Version {
            major: 1,
            minor: 0,
        };
    }
    if cli.crx3() {
        crinex.version = Version {
            major: 3,
            minor: 0,
        };
    }
    if let Some(date) = cli.date() {
        let (y, m, d, _, _, _, _) = date.to_gregorian_utc();
        if let Some((hh, mm, ss)) = cli.time() {
            crinex.date = Epoch::from_gregorian_utc(y, m, d, hh, mm, ss, 0);
        } else {
            crinex.date = Epoch::from_gregorian_utc_at_midnight(y, m, d);
        }
    } else if let Some((hh, mm, ss)) = cli.time() {
        let today = Epoch::now().expect("failed to retrieve system time");
        let (y, m, d, _, _, _, _) = today.to_gregorian_utc();
        crinex.date = Epoch::from_gregorian_utc(y, m, d, hh, mm, ss, 0); 
    }

    // output path
    let output_path = match cli.output_path() {
        Some(path) => path.clone(),
        _ => { // deduce from input
            match input_path.strip_suffix("o") {
                Some(prefix) => {
                    prefix.to_owned() + "d"
                },
                _ => {
                    match input_path.strip_suffix("O") {
                        Some(prefix) => {
                            prefix.to_owned() + "D"
                        },
                        _ => {
                            match input_path.strip_suffix("rnx") {
                                Some(prefix) => prefix.to_owned() + "crx",
                                _ => String::from("output.crx"),
                            }
                        },
                    }
                },
            }
        }
    };
    println!("Compressing \"{}\"..", input_path);
    let mut rinex = Rinex::from_file(input_path)?; // parse
    // convert
    rinex.header = rinex.header.clone()
        .with_crinex(crinex);
    rinex.to_file(&output_path)?;
    println!("{} generated", output_path);
    Ok(())
}
