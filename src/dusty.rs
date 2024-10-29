use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::str::FromStr;

const LOCATION_FILE: &str = "location.json";

#[derive(Deserialize, Serialize, Debug)]
pub struct DustyLocation {
    floor: i8,
    space: u32,
}

impl Display for DustyLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}.{}", self.floor, self.space)
    }
}

impl FromStr for DustyLocation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let regex = regex::Regex::new(r"P(?P<floor>-?\d+)\.(?P<space>\d+)")?;
        let captures = regex
            .captures(s)
            .ok_or_else(|| anyhow::anyhow!("Invalid Dusty location"))?;
        let floor = captures["floor"].parse()?;
        let space = captures["space"].parse()?;

        Ok(DustyLocation { floor, space })
    }
}

pub fn get_location() -> Result<DustyLocation, anyhow::Error> {
    let mut file =
        File::open(LOCATION_FILE).unwrap_or_else(|_| File::create(LOCATION_FILE).unwrap());
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    
    let loc: DustyLocation = serde_json::from_str(&data)?;
    println!("data: {loc:?}");
    Ok(loc)
}

pub fn set_location(new_location: &str) -> Result<(), anyhow::Error> {
    let location = DustyLocation::from_str(new_location)?;
    let data = serde_json::to_string(&location)?;
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(LOCATION_FILE)?;
    file.write_all(data.as_bytes())
        .map_err(|_| anyhow!("Could not write location file"))
}
