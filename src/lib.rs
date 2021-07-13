#![cfg_attr(not(test), no_std)]

//! Driver to working with the esp8266 module over the serial port.
//! 
//! # Warning 
//! 
//! This library is not completed and lack core features and has a lot of bugs and imperfections.
//! And so, it is not ready for production purposes.

pub use crate::{
    module::{Module, AtCommand},
    error::{Error, Result},
    reader_part::ReadData,
    softap::{JoinApConfig, SoftApConfig, WifiMode},
    network_session::{NetworkEvent, NetworkSession},
};
pub use no_std_net as net;

pub use simple_clock as clock;

mod module;
mod error;
mod parser;
mod reader_part;
mod softap;
mod network_session;

#[cfg(test)]
mod tests;
