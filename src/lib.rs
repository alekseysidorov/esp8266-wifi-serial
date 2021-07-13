// #![cfg_attr(not(test), no_std)]

//! Driver to working with the esp8266 module over the serial port.
//!
//! # Warning
//!
//! This library is not completed and lack core features and has a lot of bugs and imperfections.
//! And so, it is not ready for production purposes.

pub use crate::{
    error::{Error, Result},
    module::{AtCommand, Module},
    network_session::{NetworkEvent, NetworkSession},
    reader_part::ReadData,
    softap::{JoinApConfig, SoftApConfig, WifiMode},
};
pub use no_std_net as net;

pub use simple_clock as clock;

mod error;
mod module;
mod network_session;
mod parser;
mod reader_part;
mod softap;

#[cfg(test)]
mod tests;
