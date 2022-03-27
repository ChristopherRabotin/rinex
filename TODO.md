Constellation:
* [ ] constellation::Geo ?    
p44   
"is almost identical to Constellation == glonass
but the records contain the satellite position, velocity, accel, health.."

Doc:
* [ ] epoch: determine longest dead time for a given constellation or sv
* [ ] epoch: time spanning

General :
* [ ] move to buffered reader instead of fs::to\_string() for better
performances, pass BufReader pointer to build\_record method
* [x] cleanup (head, body) splitting
* [x] last epoch seems to always be missed
* [ ] add to::file production method
* [ ] simplify line interations with "for line in lines.next()"

Header:
* [ ] some files crash when parsing 
REC or ANT or ANTENNA or APPROX POSITION XYZ with ParseIntError
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
* [ ] interval ?

Record :
* [x] ObsRecord : add clockoffsets to epoch record
* [x] ObsRecord : introduce Observation(f32,lli,ssi) as payload

Navigation Messages:
* [ ] improve database usage.   
`revision.minor` might be passed and must be used.   
We should parse using the closest revision number

Observation Data:
* [x] parse OBS codes V < 3
* [x] parse OBS codes V > 2
* [x] parse OBS record V < 3
* [x] parse OBS record V > 2
* [x] parse clock offsets and classify them properly
* [ ] rescale raw phase data exceeding F14.3 format by +/- 10E9 accordingly
* [ ] SYS PHASE Shift ?

Ci:
* [ ] OBS: if rcvr clock offsets applied: check epochs do have this field

Hatanaka:
* [x] numerical decompression
* [x] text decompression
* [x] epoch decompression
* [x] double \n inserted on my recovered epochs
* [ ] CRINEX 1 + epoch with flag > 2 (special events)
will not be identified correctly
* [ ] CRINEX 1|3 special epoch content (flag>2)
will be mishandled / corrupted if they are not only made of COMMENTS

Meteo Data:
* [x] parse METEO codes
* [x] parse METEO V < 3
* [x] parse METEO V > 2
* [ ] Sensor Geo position (x,y,z)

Clocks Data:
* [ ] TODO 
