use core::{format_args, ops::Deref};

use embedded_hal::serial;
use heapless::Vec;
use no_std_net::SocketAddr;
use simple_clock::SimpleClock;

use crate::{
    adapter::{Adapter, CarretCondition, OkCondition, ReadPart},
    parser::CommandResponse,
    Error,
};

pub struct WifiSession<Rx, Tx, C>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    C: SimpleClock,
{
    adapter: Adapter<Rx, Tx, C>,
}

impl<Rx, Tx, C> WifiSession<Rx, Tx, C>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    C: SimpleClock,
{
    pub(crate) fn new(mut adapter: Adapter<Rx, Tx, C>) -> Self {
        adapter.reader.buf.clear();
        Self { adapter }
    }

    pub fn listen(&mut self, port: u16) -> crate::Result<SocketAddr> {
        // Setup a TCP server.
        self.adapter
            .send_at_command_fmt(format_args!("AT+CIPSERVER=1,{}", port))?
            .expect("Malformed command");
        self.adapter.clear_reader_buf();

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
        self.adapter.clear_reader_buf();

        Ok(())
    }

    pub fn poll_next_event(&mut self) -> nb::Result<Event<'_, Rx>, Error> {
        self.adapter.reader.poll_next_event()
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
        assert!(self.adapter.reader.buf.is_empty());

        self.adapter
            .write_command_fmt(format_args!("AT+CIPSEND={},{}", link_id, bytes_len))?;
        self.adapter.read_until(CarretCondition)?;

        for byte in bytes {
            nb::block!(self.adapter.writer.write_byte(byte))?;
        }

        self.adapter
            .read_until(OkCondition)?
            .expect("Malformed command");
        self.adapter.clear_reader_buf();
        Ok(())
    }

    pub fn clock(&self) -> &C {
        &self.adapter.clock
    }

    pub fn socket_timeout(&self) -> u64 {
        self.adapter.socket_timeout
    }
}

pub enum Event<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    Connected { link_id: usize },
    Closed { link_id: usize },
    DataAvailable { link_id: usize, data: Data<'a, Rx> },
}

impl<Rx> ReadPart<Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    pub(crate) fn poll_next_event(&mut self) -> nb::Result<Event<'_, Rx>, Error> {
        let response =
            CommandResponse::parse(&self.buf).map(|(remainder, event)| (remainder.len(), event));

        if let Some((remaining_bytes, response)) = response {
            let pos = self.buf.len() - remaining_bytes;
            truncate_buf(&mut self.buf, pos);

            let event = match response {
                CommandResponse::Connected { link_id } => Event::Connected { link_id },
                CommandResponse::Closed { link_id } => Event::Closed { link_id },
                CommandResponse::DataAvailable { link_id, size } => {
                    let current_pos = self.buf.len();
                    for _ in current_pos..size {
                        self.buf
                            .push(nb::block!(self.rx.read()).map_err(|_| Error::ReadBuffer)?)
                            .unwrap();
                    }

                    Event::DataAvailable {
                        link_id,
                        data: Data { inner: self },
                    }
                }
                CommandResponse::WifiDisconnect => return Err(nb::Error::WouldBlock),
            };

            return Ok(event);
        }

        self.read_bytes()?;
        Err(nb::Error::WouldBlock)
    }
}

pub struct Data<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    inner: &'a mut ReadPart<Rx>,
}

impl<'a, Rx> AsRef<[u8]> for Data<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    fn as_ref(&self) -> &[u8] {
        self.inner.buf.as_ref()
    }
}

impl<'a, Rx> Drop for Data<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    fn drop(&mut self) {
        self.inner.buf.clear();
    }
}

impl<'a, Rx> Deref for Data<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.inner.buf.as_ref()
    }
}

// FIXME: Reduce complexity of this operation.
fn truncate_buf<const N: usize>(buf: &mut Vec<u8, N>, at: usize) {
    assert!(at <= buf.len());

    for from in at..buf.len() {
        let to = from - at;
        buf[to] = buf[from];
    }
    // Safety: `u8` is aprimitive type and doesn't have drop implementation so we can just
    // modify the buffer length.
    unsafe {
        buf.set_len(buf.len() - at);
    }
}
