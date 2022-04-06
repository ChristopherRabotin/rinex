Constellation:
* [ ] constellation::Geo ?    
p44   
"is almost identical to Constellation == glonass
but the records contain the satellite position, velocity, accel, health.."

Examples :
* [ ] epoch: determine longest dead time for a given constellation or sv
* [ ] epoch: time spanning

General :
* [ ] add to::file production method
* [ ] simplify line interations with "for line in lines.next()"

Header:
* [ ] some (V3) files crash when parsing LEAP SECOND field 
* [ ] time of first and last obs parsing is faulty
* [ ] header.antenna.model sometimes appear as dirty, check this out
* [ ] coords [m] system ?
* [x] rcvr - clock offset applied
 * [ ] data compensation to do with this?
 * [x] simplify: set to simple boolean TRUE/FALSE
* [ ] GnssTime + possible conversion needed ?
* [ ] WaveLength fact L1/2 ?
* [ ] Glonass SLOT /freq channel ?
* [ ] Glonass COD/PHS/BIS ?
* [x] interval

Comments :
* [ ] move comments to a separate structure and attach an epoch to them

Special operations:
* [ ] Merged file: 
 * [ ] methods based on comments for merging identification
 * [x] merged NAV
 * [ ] merged OBS
* [ ] File merging (writer)

Record :
* [ ] problem in case of `merged` RINEX at least for NAV data: 
same epoch, new sv: .insert() overwrites previous entry
* [x] ObsRecord : add clockoffsets to epoch record
* [x] ObsRecord : introduce Observation(f32,lli,ssi) as payload

Observation Data:
* [ ] rescale raw phase data exceeding F14.3 format by +/- 10E9 accordingly
* [ ] SYS PHASE Shift ?

Ci:
* [ ] OBS: if rcvr clock offsets applied: check epochs do have this field
* [ ] move src/lib.rs tests to dedicated tests/ folder 

Hatanaka:
* [ ] find some CRINEX with special epoch events (flag>2) and test them
* [ ] CRINEX 1|3 special epoch content (flag>2)
will be mishandled / corrupted if they are not only made of COMMENTS
* [ ] replace `zeros` by `intertools::fill_with()` 

Meteo Data:
* [ ] parse METEO V > 3
* [ ] Sensor Geo position (x,y,z)

Clocks Data:
* [ ] V1, V2 
* [ ] V3
* [ ] V4
