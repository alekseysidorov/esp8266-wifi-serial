use core::fmt::Write;

use embedded_hal::serial;
use simple_clock::{Deadline, SimpleClock};

use crate::{
    error::{Error, Result},
    parser::CifsrResponse,
    reader_part::{ReadData, ReaderPart},
};

pub type RawResponse<'a, const N: usize> = core::result::Result<ReadData<'a, N>, ReadData<'a, N>>;

const NEWLINE: &[u8] = b"\r\n";

#[derive(Debug)]
pub struct Adapter<Rx, Tx, C, const N: usize>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    C: SimpleClock,
{
    pub(crate) reader: ReaderPart<Rx, N>,
    pub(crate) writer: WriterPart<Tx>,
    pub(crate) clock: C,
    pub(crate) socket_timeout: u64,
}

impl<'a, Rx, Tx, C, const N: usize> Adapter<Rx, Tx, C, N>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    C: SimpleClock,
{
    pub fn new(rx: Rx, tx: Tx, clock: C, socket_timeout: u64) -> Result<Self> {
        let mut adapter = Self {
            reader: ReaderPart::new(rx),
            writer: WriterPart { tx },
            clock,
            socket_timeout,
        };
        adapter.init()?;
        Ok(adapter)
    }

    fn init(&mut self) -> Result<()> {
        self.disable_echo()?;
        Ok(())
    }

    fn reset_cmd(&mut self) -> Result<()> {
        self.write_command(b"AT+RST")?;
        self.read_until(ReadyCondition)?;

        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        // FIXME: It is ok to receive errors like "framing" during the reset procedure.
        self.reset_cmd().ok();
        // Workaround to catch the framing errors.
        for _ in 0..100 {
            self.send_at_command_str("ATE1").ok();
        }

        self.disable_echo()?;
        Ok(())
    }

    // FIXME: Get rid of the necessity of the manual `clear_reader_buf` invocations.
    pub fn send_at_command_str(&mut self, cmd: &str) -> Result<RawResponse<'_, N>> {
        self.write_command(cmd.as_ref())?;
        self.read_until(OkCondition)
    }

    pub fn send_at_command_fmt(
        &mut self,
        args: core::fmt::Arguments,
    ) -> Result<RawResponse<'_, N>> {
        self.write_command_fmt(args)?;
        self.read_until(OkCondition)
    }

    fn disable_echo(&mut self) -> Result<()> {
        self.send_at_command_str("ATE0").map(drop)
    }

    pub(crate) fn write_command(&mut self, cmd: &[u8]) -> Result<()> {
        self.writer.write_bytes(cmd)?;
        self.writer.write_bytes(NEWLINE)
    }

    pub(crate) fn write_command_fmt(&mut self, args: core::fmt::Arguments) -> Result<()> {
        self.writer.write_fmt(args)?;
        self.writer.write_bytes(NEWLINE)
    }

    pub(crate) fn read_until<'b, T>(&'b mut self, condition: T) -> Result<T::Output>
    where
        T: Condition<'b, N>,
    {
        let deadline = Deadline::new(&self.clock, self.socket_timeout);
        loop {
            match self.reader.read_bytes() {
                Ok(_) => {
                    if self.reader.buf().is_full() {
                        return Err(Error::BufferFull);
                    }
                }
                Err(nb::Error::WouldBlock) => {}
                Err(nb::Error::Other(_)) => {
                    return Err(Error::ReadBuffer);
                }
            };

            if condition.is_performed(&self.reader.buf()) {
                break;
            }

            deadline.reached().map_err(|_| Error::Timeout)?;
        }

        let read_data = ReadData::new(self.reader.buf_mut());
        Ok(condition.output(read_data))
    }

    pub(crate) fn get_softap_address(&mut self) -> Result<CifsrResponse> {
        // Get assigned SoftAP address.
        let raw_resp = self
            .send_at_command_fmt(format_args!("AT+CIFSR"))?
            .expect("Malformed command");

        let resp = CifsrResponse::parse(&raw_resp).expect("Unknown response").1;
        Ok(resp)
    }
}

pub(crate) trait Condition<'a, const N: usize>: Copy {
    type Output: 'a;

    fn is_performed(self, buf: &[u8]) -> bool;

    fn output(self, buf: ReadData<'a, N>) -> Self::Output;
}

#[derive(Clone, Copy)]
struct ReadyCondition;

impl ReadyCondition {
    const MSG: &'static [u8] = b"ready\r\n";
}

impl<'a, const N: usize> Condition<'a, N> for ReadyCondition {
    type Output = ReadData<'a, N>;

    fn is_performed(self, buf: &[u8]) -> bool {
        buf.ends_with(Self::MSG)
    }

    fn output(self, mut buf: ReadData<'a, N>) -> Self::Output {
        buf.subslice(0, buf.len() - Self::MSG.len());
        buf
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CarretCondition;

impl CarretCondition {
    const MSG: &'static [u8] = b"> ";
}

impl<'a, const N: usize> Condition<'a, N> for CarretCondition {
    type Output = ReadData<'a, N>;

    fn is_performed(self, buf: &[u8]) -> bool {
        buf.ends_with(Self::MSG)
    }

    fn output(self, mut buf: ReadData<'a, N>) -> Self::Output {
        buf.subslice(0, buf.len() - Self::MSG.len());
        buf
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OkCondition;

impl OkCondition {
    const OK: &'static [u8] = b"OK\r\n";
    const ERROR: &'static [u8] = b"ERROR\r\n";
}

impl<'a, const N: usize> Condition<'a, N> for OkCondition {
    type Output = RawResponse<'a, N>;

    fn is_performed(self, buf: &[u8]) -> bool {
        buf.ends_with(Self::OK) || buf.ends_with(Self::ERROR)
    }

    fn output(self, mut buf: ReadData<'a, N>) -> Self::Output {
        if buf.ends_with(Self::OK) {
            buf.subslice(0, buf.len() - Self::OK.len());
            Ok(buf)
        } else {
            buf.subslice(0, buf.len() - Self::ERROR.len());
            Err(buf)
        }
    }
}

#[derive(Debug)]
pub struct WriterPart<Tx> {
    tx: Tx,
}

impl<Tx> WriterPart<Tx>
where
    Tx: serial::Write<u8> + 'static,
{
    fn write_fmt(&mut self, args: core::fmt::Arguments) -> Result<()> {
        let writer = &mut self.tx as &mut (dyn serial::Write<u8, Error = Tx::Error> + 'static);
        writer.write_fmt(args).map_err(|_| Error::WriteBuffer)
    }

    pub(crate) fn write_byte(&mut self, byte: u8) -> nb::Result<(), Error> {
        self.tx
            .write(byte)
            .map_err(|err| err.map(|_| Error::WriteBuffer))
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        for byte in bytes.iter() {
            nb::block!(self.write_byte(*byte))?;
        }
        Ok(())
    }
}
