#![cfg_attr(not(test), no_std)]

pub use crate::{
    adapter::Adapter,
    error::{Error, Result},
    softap::{JoinApConfig, SoftApConfig, WifiMode},
    wifi_session::{Event, WifiSession},
};
pub use no_std_net as net;
pub use simple_clock as clock;

pub mod error;

mod adapter;
mod parser;
mod softap;
mod wifi_session;
mod reader_part;

#[cfg(test)]
mod tests;
