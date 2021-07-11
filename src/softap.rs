use core::{fmt::Debug, format_args};

use embedded_hal::serial;
use serde::{Deserialize, Serialize};
use simple_clock::SimpleClock;

use crate::{adapter::Adapter, WifiSession};

/// Wifi modes that supported by this module.
#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Eq)]
pub enum WifiMode {
    /// Open network mode without any encryption.
    Open = 0,
    /// WPA PSK encryption mode.
    WpaPsk = 2,
    /// WPA2 PSK encryption mode.
    Wpa2Psk = 3,
    /// Both WPA PSK and WPA2 PSK encryption modes.
    WpaWpa2Psk = 4,
}

/// Software access point configuration parameters.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Eq)]
pub struct SoftApConfig<'a> {
    /// Access point SSID.
    pub ssid: &'a str,
    /// Access point password.
    ///
    /// This field will be ignored if WiFi mode is open.
    pub password: &'a str,
    /// Channel number.
    pub channel: u8,
    /// WiFi mode.
    pub mode: WifiMode,
}

impl<'a> SoftApConfig<'a> {
    /// Creates a software access point with the configuration parameters and establishes
    /// a new WiFi session.
    pub fn start<Rx, Tx, C, const N: usize>(
        self,
        mut adapter: Adapter<Rx, Tx, C, N>,
    ) -> crate::Result<WifiSession<Rx, Tx, C, N>>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,
    {
        self.init(&mut adapter)?;
        Ok(WifiSession::new(adapter))
    }

    fn init<Rx, Tx, C, const N: usize>(
        &self,
        adapter: &mut Adapter<Rx, Tx, C, N>,
    ) -> crate::Result<()>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,
    {
        // Enable SoftAP+Station mode.
        adapter
            .send_at_command_str("AT+CWMODE=3")?
            .expect("Malformed command");

        // Enable multiple connections.
        adapter
            .send_at_command_str("AT+CIPMUX=1")?
            .expect("Malformed command");

        // Start SoftAP.
        adapter
            .send_at_command_fmt(format_args!(
                "AT+CWSAP=\"{}\",\"{}\",{},{}",
                self.ssid, self.password, self.channel, self.mode as u8,
            ))?
            .expect("Malformed command");

        Ok(())
    }
}

/// Configuration parameters describe a connection to the existing access point.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct JoinApConfig<'a> {
    /// Access point SSID.
    pub ssid: &'a str,
    /// Access point password.
    pub password: &'a str,
}

impl<'a> JoinApConfig<'a> {
    /// Joins to the existing access point and establishing a new WiFi session.
    pub fn join<Rx, Tx, C, const N: usize>(
        self,
        mut adapter: Adapter<Rx, Tx, C, N>,
    ) -> crate::Result<WifiSession<Rx, Tx, C, N>>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,
    {
        self.init(&mut adapter)?;
        Ok(WifiSession::new(adapter))
    }

    fn init<Rx, Tx, C, const N: usize>(
        &self,
        adapter: &mut Adapter<Rx, Tx, C, N>,
    ) -> crate::Result<()>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,
    {
        // Enable Station mode.
        adapter
            .send_at_command_str("AT+CWMODE=1")?
            .expect("Malformed command");

        // Enable multiple connections.
        adapter
            .send_at_command_str("AT+CIPMUX=1")?
            .expect("Malformed command");

        // Join the given access point.
        adapter
            .send_at_command_fmt(format_args!(
                "AT+CWJAP=\"{}\",\"{}\"",
                self.ssid, self.password,
            ))?
            .expect("Malformed command");

        Ok(())
    }
}
