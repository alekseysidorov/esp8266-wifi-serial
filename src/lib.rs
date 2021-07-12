#![cfg_attr(not(test), no_std)]

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
