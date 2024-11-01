use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::str::FromStr;

const LOCATION_FILE: &str = "location.json";

pub enum DustyError {
    InvalidFloor,
    InvalidSpace,
    InvalidFormat,
    InternalError,
    FileError,
}

impl Display for DustyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DustyError::InvalidFloor => {
                write!(f, "Invalid floor, please input a floor between [-4,0]")
            }
            DustyError::InvalidSpace => {
                write!(f, "Invalid space, please input a space between [0,400]")
            }
            DustyError::InvalidFormat => write!(
                f,
                "Invalid format, please input a location in the format P<floor>.<space>"
            ),
            DustyError::InternalError => {
                write!(f, "I experienced an internal error, please check my logs")
            }
            DustyError::FileError => {
                write!(f, "Couldn't access Dusty's location file.")
            }
        }
    }
}

impl Debug for DustyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::error::Error for DustyError {}

#[derive(Deserialize, Debug, Serialize)]
pub struct DustyState {
    dusty_location: DustyLocation,
    parked_back: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DustyLocation {
    floor: i8,
    space: u32,
}

impl Display for DustyLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}.{:03}", self.floor, self.space)
    }
}

impl FromStr for DustyLocation {
    type Err = DustyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let regex = regex::Regex::new(r"P(?P<floor>-?\d+)\.(?P<space>\d+)").unwrap();
        let captures = regex.captures(s).ok_or(DustyError::InvalidFormat)?;
        let floor = captures["floor"]
            .parse()
            .map_err(|_| DustyError::InvalidFloor)?;
        let space = captures["space"]
            .parse()
            .map_err(|_| DustyError::InvalidSpace)?;

        if !(-4..=4).contains(&floor) {
            return Err(DustyError::InvalidFloor);
        }
        if !(0..=400).contains(&space) {
            return Err(DustyError::InvalidSpace);
        }

        Ok(DustyLocation { floor, space })
    }
}

fn get_state() -> Result<DustyState, anyhow::Error> {
    let mut file =
        File::open(LOCATION_FILE).map_err(|_| DustyError::FileError)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    let state: DustyState = serde_json::from_str(&data)?;
    set_state(&format!("{}", state.dusty_location), false)?;
    Ok(state)
}

fn set_state(new_location: &str, parked_back: bool) -> Result<(), DustyError> {
    let dusty_location = DustyLocation::from_str(new_location)?;
    let dusty_state = DustyState {
        dusty_location,
        parked_back,
    };
    let data = serde_json::to_string(&dusty_state).map_err(|_| DustyError::InternalError)?;
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(LOCATION_FILE)
        .map_err(|_| DustyError::FileError)?;
    file.write_all(data.as_bytes())
        .map_err(|_| DustyError::InternalError)
}

pub fn handle_dusty_query(message: &str) -> String {
    let response = if message.to_lowercase().as_str() == "where is dusty?" {
        match get_state() {
            Ok(state) => {
                match state.parked_back {
                    true => format!("Dusty is at *{}*", state.dusty_location),
                    false => format!("Someone asked where Dusty is, but hasn't explicitly put it back. *{}* was its last know location", state.dusty_location)
                }
            }
            Err(e) => {
                e.to_string()
            }
        }
    } else if message.to_lowercase().starts_with("dusty is at ") {
        let new_location = message["Dusty is at ".len()..].trim();
        match set_state(new_location, true) {
            Ok(_) => "Got it!".to_string(),
            Err(e) => e.to_string(),
        }
    } else {
        "I don't understand that. Please either ask 'Where is Dusty?', or tell me 'Dusty is at P<floor>.<space>' (example: 'Dusty is at P1.303')".to_string()
    };

    response
}
