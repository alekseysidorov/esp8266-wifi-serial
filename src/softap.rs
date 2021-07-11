use core::{fmt::Debug, format_args};

use embedded_hal::serial;
use serde::{Deserialize, Serialize};
use simple_clock::SimpleClock;

use crate::{adapter::Adapter, WifiSession};

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Eq)]
pub enum WifiMode {
    Open = 0,
    WpaPsk = 2,
    Wpa2Psk = 3,
    WpaWpa2Psk = 4,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Eq)]
pub struct SoftApConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
    pub channel: u8,
    pub mode: WifiMode,
}

impl<'a> SoftApConfig<'a> {
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

        adapter.clear_reader_buf();
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct JoinApConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
}

impl<'a> JoinApConfig<'a> {
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
        adapter.clear_reader_buf();

        Ok(())
    }
}
