#[cfg(test)]
mod test {
    use rinex::*;
    use rinex::epoch;
    use rinex::clocks;
    use rinex::sv::Sv;
    use rinex::constellation::Constellation;
    use rinex::clocks::record::{DataType, System};
    #[test]
    fn v3_usno_example() {
        let test_resource = 
            env!("CARGO_MANIFEST_DIR").to_owned() 
            + "/../test_resources/CLK/V3/USNO1.txt";
        let rinex = Rinex::from_file(&test_resource);
        assert_eq!(rinex.is_ok(), true);
        let rinex = rinex.unwrap();
        assert_eq!(rinex.is_clocks_rinex(), true);
        assert_eq!(rinex.header.clocks.is_some(), true);
        let clocks = rinex.header.clocks
            .as_ref()
            .unwrap();
        assert_eq!(clocks.codes, vec![
            DataType::AS,
            DataType::AR,
            DataType::CR,
            DataType::DR]);
        assert_eq!(clocks.agency, Some(clocks::Agency {
            code: String::from("USN"),
            name: String::from("USNO USING GIPSY/OASIS-II"),
        }));
        assert_eq!(clocks.station, Some(clocks::Station {
            name: String::from("USNO"),
            id: String::from("40451S003"),
        }));
        println!("{:#?}", rinex.record);
        assert_eq!(rinex.epochs().len(), 1);
        let record = rinex.record.as_clock();
        assert_eq!(record.is_some(), true);
        let record = record.unwrap();
        for (e, data_types) in record.iter() {
            assert_eq!(*e, epoch::Epoch {
                date: epoch::str2date("1994 07 14 20 59  0.000000").unwrap(),
                flag: epoch::EpochFlag::Ok,
            });
            for (data_type, systems) in data_types.iter() {
                assert_eq!(systems.len(), 1);
                if *data_type == DataType::AR {
                    for (system, data) in systems.iter() {
                        assert_eq!(*system, System::Station("AREQ".to_string()));
                        assert_eq!(data.bias,  -0.123456789012);
                        assert_eq!(data.bias_sigma, Some(-1.23456789012E+0));
                        assert_eq!(data.rate, Some(-12.3456789012));
                        assert_eq!(data.rate_sigma, Some(-123.456789012));
                    }
                } else if *data_type == DataType::AS {
                    for (system, data) in systems.iter() {
                        assert_eq!(*system, System::Sv(Sv {
                            constellation: Constellation::GPS,
                            prn: 16
                        }));
                    }

                } else if *data_type == DataType::CR {
                    for (system, data) in systems.iter() {
                        assert_eq!(*system, System::Station("USNO".to_string()));
                    }

                } else if *data_type == DataType::DR {
                    for (system, data) in systems.iter() {
                        assert_eq!(*system, System::Station("USNO".to_string()));
                    }

                } else {
                    panic!("identified unexpected data type \"{}\"", data_type);
                }
            }
        }
    }
}
