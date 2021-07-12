use core::format_args;

use embedded_hal::serial;
use heapless::Vec;
use simple_clock::SimpleClock;

use crate::{
    module::{CarretCondition, Module, OkCondition},
    parser::CommandResponse,
    reader_part::{ReadData, ReaderPart},
    Error,
    net::{IpAddr, SocketAddr},
};

#[derive(Debug, PartialEq, Eq)]
pub struct SessionInfo {
    pub softap_address: Option<IpAddr>,
    pub listen_address: IpAddr,
}

/// A session with the typical network operations.
pub struct NetworkSession<Rx, Tx, C, const N: usize>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    C: SimpleClock,
{
    module: Module<Rx, Tx, C, N>,
}

impl<Rx, Tx, C, const N: usize> NetworkSession<Rx, Tx, C, N>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    C: SimpleClock,
{
    pub(crate) fn new(module: Module<Rx, Tx, C, N>) -> Self {
        Self { module }
    }

    /// Begins to listen to the incoming TCP connections on the specified port.
    pub fn listen(&mut self, port: u16) -> crate::Result<()> {
        // Setup a TCP server.
        self.module
            .send_at_command(format_args!("AT+CIPSERVER=1,{}", port))?
            .expect("Malformed command");

        Ok(())
    }

    /// Establishes a TCP connection with the specified IP address, link identifier will
    /// be associated with the given IP address.
    /// Then it will be possible to [send](Self::send) data using this link ID.
    pub fn connect(&mut self, link_id: usize, address: SocketAddr) -> crate::Result<()> {
        self.module
            .send_at_command(format_args!(
                "AT+CIPSTART={},\"{}\",\"{}\",{}",
                link_id,
                "TCP",
                address.ip(),
                address.port(),
            ))?
            .expect("Malformed command");

        Ok(())
    }

    /// Non-blocking polling to get a new network event.
    pub fn poll_network_event(&mut self) -> nb::Result<NetworkEvent<'_, N>, Error> {
        let reader = self.reader_mut();

        let response =
            CommandResponse::parse(reader.buf()).map(|(remainder, event)| (remainder.len(), event));

        if let Some((remaining_bytes, response)) = response {
            let pos = reader.buf().len() - remaining_bytes;
            truncate_buf(reader.buf_mut(), pos);

            let event = match response {
                CommandResponse::Connected { link_id } => NetworkEvent::Connected { link_id },
                CommandResponse::Closed { link_id } => NetworkEvent::Closed { link_id },
                CommandResponse::DataAvailable { link_id, size } => {
                    let current_pos = reader.buf().len();
                    for _ in current_pos..size {
                        let byte = nb::block!(reader.read_byte())?;
                        reader.buf_mut().push(byte).map_err(|_| Error::BufferFull)?;
                    }

                    NetworkEvent::DataAvailable {
                        link_id,
                        data: ReadData::new(reader.buf_mut()),
                    }
                }
                CommandResponse::WifiDisconnect => return Err(nb::Error::WouldBlock),
            };

            return Ok(event);
        }

        reader.read_bytes()?;
        Err(nb::Error::WouldBlock)
    }

    /// Sends data packet via the TCP socket with the link given identifier.
    ///
    /// # Notes
    ///
    /// No more than 2048 bytes can be sent at a time.
    pub fn send<I>(&mut self, link_id: usize, bytes: I) -> crate::Result<()>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let bytes_len = bytes.len();
        // TODO Implement sending of the whole bytes by splitting them into chunks.
        assert!(
            bytes_len < 2048,
            "Total packet size should not be greater than the 2048 bytes"
        );
        assert!(self.reader().buf().is_empty());

        self.module
            .write_command_fmt(format_args!("AT+CIPSEND={},{}", link_id, bytes_len))?;
        self.module.read_until(CarretCondition)?;

        for byte in bytes {
            nb::block!(self.module.writer.write_byte(byte))?;
        }

        self.module
            .read_until(OkCondition)?
            .expect("Malformed command");
        Ok(())
    }

    /// Gets network session information.
    pub fn get_info(&mut self) -> crate::Result<SessionInfo> {
        let info = self.module.get_network_info()?;
        Ok(SessionInfo {
            softap_address: info.ap_ip,
            listen_address: info.sta_ip
        })
    }

    /// Returns a reference to underlying clock instance.
    pub fn clock(&self) -> &C {
        &self.module.clock
    }

    /// Returns an operations timeout.
    pub fn timeout(&self) -> Option<u64> {
        self.module.timeout
    }

    fn reader(&self) -> &ReaderPart<Rx, N> {
        &self.module.reader
    }

    fn reader_mut(&mut self) -> &mut ReaderPart<Rx, N> {
        &mut self.module.reader
    }
}

/// Incoming network event.
#[derive(Debug)]
pub enum NetworkEvent<'a, const N: usize> {
    /// A new peer connected.
    Connected {
        /// Connection identifier.
        link_id: usize,
    },
    /// The connection with the peer is closed.
    Closed {
        /// Connection identifier.
        link_id: usize,
    },
    /// Bytes received from the peer.
    DataAvailable {
        /// Connection identifier.
        link_id: usize,
        /// Received data.
        data: ReadData<'a, N>,
    },
}

// FIXME: Reduce complexity of this operation.
fn truncate_buf<const N: usize>(buf: &mut Vec<u8, N>, at: usize) {
    let buf_len = buf.len();

    assert!(at <= buf_len);

    for from in at..buf_len {
        let to = from - at;
        buf[to] = buf[from];
    }

    // Safety: `u8` is aprimitive type and doesn't have drop implementation so we can just
    // modify the buffer length.
    unsafe {
        buf.set_len(buf_len - at);
    }
}
