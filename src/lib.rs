use derive_more::Display;
use io::Write;
use serialport::Error as SerialError;
use std::error::Error as StdError;
use std::fmt;
use std::io;

mod enttec;
mod offline;

pub use enttec::EnttecDmxPort;
pub use offline::OfflineDmxPort;

/// Trait for the general notion of a DMX port.
/// This enables creation of an "offline" port to slot into place if an API requires an output.
#[typetag::serde(tag = "type")]
pub trait DmxPort: fmt::Display {
    /// Return the available ports.  The ports will need to be opened before use.
    fn available_ports() -> Result<PortListing, Error>
    where
        Self: Sized;

    /// Return a string identifier for this port.
    fn name(&self) -> &str;

    /// Open the port for writing.  Implementations should no-op if this is
    /// called twice rather than returning an error.  Primarily used to re-open
    /// a port that has be deserialized.
    fn open(&mut self) -> Result<(), Error>;

    /// Close the port.
    fn close(&mut self);

    /// Write a DMX frame out to the port.  If the frame is smaller than the minimum universe size,
    /// it will be padded with zeros.  If the frame is larger than the maximum universe size, the
    /// values beyond the max size will be ignored.
    fn write(&mut self, frame: &[u8]) -> Result<(), Error>;
}

/// A listing of available ports.
type PortListing = Vec<Box<dyn DmxPort>>;

/// Gather up all of the providers and use them to get listings of all ports they have available.
/// Return them as a vector of names plus opener functions.
/// This function does not check whether or not any of the ports are in use already.
pub fn available_ports() -> Result<PortListing, Error> {
    let mut ports = Vec::new();
    ports.extend(OfflineDmxPort::available_ports()?.into_iter());
    ports.extend(EnttecDmxPort::available_ports()?.into_iter());
    Ok(ports)
}

/// Prompt the user to select a port via the command prompt.
pub fn select_port() -> Result<Box<dyn DmxPort>, Error> {
    let mut ports = available_ports()?;
    println!("Available DMX ports:");
    for (i, port) in ports.iter().enumerate() {
        println!("{}: {}", i, port.name());
    }
    let mut port = loop {
        print!("Select a port: ");
        io::stdout().flush()?;
        let input = read_string()?;
        let index = match input.trim().parse::<usize>() {
            Ok(num) => num,
            Err(e) => {
                println!("{}; please enter an integer.", e);
                continue;
            }
        };
        if index >= ports.len() {
            println!("Please enter a value less than {}.", ports.len());
            continue;
        }
        break ports.swap_remove(index);
    };
    port.open()?;
    Ok(port)
}

/// Read a line of input from stdin.
fn read_string() -> Result<String, io::Error> {
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

#[derive(Debug, Display)]
pub enum Error {
    Serial(SerialError),
    IO(std::io::Error),
    PortClosed,
}

impl From<SerialError> for Error {
    fn from(e: SerialError) -> Self {
        Error::Serial(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        use Error::*;
        match *self {
            Serial(ref e) => Some(e),
            IO(ref e) => Some(e),
            PortClosed => None,
        }
    }
}
