use core::format_args;

use embedded_hal::serial;
use heapless::Vec;
use no_std_net::SocketAddr;
use simple_clock::SimpleClock;

use crate::{
    adapter::{Adapter, CarretCondition, OkCondition},
    parser::CommandResponse,
    reader_part::{ReadData, ReaderPart},
    Error,
};

pub struct WifiSession<Rx, Tx, C, const N: usize>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    C: SimpleClock,
{
    adapter: Adapter<Rx, Tx, C, N>,
}

impl<Rx, Tx, C, const N: usize> WifiSession<Rx, Tx, C, N>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    C: SimpleClock,
{
    pub(crate) fn new(mut adapter: Adapter<Rx, Tx, C, N>) -> Self {
        adapter.reader.clear();
        Self { adapter }
    }

    pub fn listen(&mut self, port: u16) -> crate::Result<SocketAddr> {
        // Setup a TCP server.
        self.adapter
            .send_at_command_fmt(format_args!("AT+CIPSERVER=1,{}", port))?
            .expect("Malformed command");

        // Get assigned IP address.
        let ip = self
            .adapter
            .get_softap_address()?
            .ap_ip
            .expect("the IP address for this access point did't assign.");
        Ok(SocketAddr::new(ip, port))
    }

    pub fn connect_to(&mut self, link_id: usize, address: SocketAddr) -> crate::Result<()> {
        self.adapter
            .send_at_command_fmt(format_args!(
                "AT+CIPSTART={},\"{}\",\"{}\",{}",
                link_id,
                "TCP",
                address.ip(),
                address.port(),
            ))?
            .expect("Malformed command");

        Ok(())
    }

    pub fn poll_next_event(&mut self) -> nb::Result<Event<'_, N>, Error> {
        let reader = self.reader_mut();

        let response =
            CommandResponse::parse(reader.buf()).map(|(remainder, event)| (remainder.len(), event));

        if let Some((remaining_bytes, response)) = response {
            let pos = reader.buf().len() - remaining_bytes;
            truncate_buf(reader.buf_mut(), pos);

            let event = match response {
                CommandResponse::Connected { link_id } => Event::Connected { link_id },
                CommandResponse::Closed { link_id } => Event::Closed { link_id },
                CommandResponse::DataAvailable { link_id, size } => {
                    let current_pos = reader.buf().len();
                    for _ in current_pos..size {
                        let byte = nb::block!(reader.read_byte())?;
                        reader.buf_mut().push(byte).map_err(|_| Error::BufferFull)?;
                    }

                    Event::DataAvailable {
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

    pub fn send_to<I>(&mut self, link_id: usize, bytes: I) -> crate::Result<()>
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

        self.adapter
            .write_command_fmt(format_args!("AT+CIPSEND={},{}", link_id, bytes_len))?;
        self.adapter.read_until(CarretCondition)?;

        for byte in bytes {
            nb::block!(self.adapter.writer.write_byte(byte))?;
        }

        self.adapter
            .read_until(OkCondition)?
            .expect("Malformed command");
        Ok(())
    }

    pub fn clock(&self) -> &C {
        &self.adapter.clock
    }

    pub fn socket_timeout(&self) -> u64 {
        self.adapter.socket_timeout
    }

    fn reader(&self) -> &ReaderPart<Rx, N> {
        &self.adapter.reader
    }

    fn reader_mut(&mut self) -> &mut ReaderPart<Rx, N> {
        &mut self.adapter.reader
    }
}

pub enum Event<'a, const N: usize> {
    Connected {
        link_id: usize,
    },
    Closed {
        link_id: usize,
    },
    DataAvailable {
        link_id: usize,
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
