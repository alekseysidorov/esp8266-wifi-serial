//! Reader part of the esp8266 WiFi implementation.

use core::{
    fmt::{self, Write},
    ops::Deref,
};

use embedded_hal::serial;
use heapless::Vec;

use crate::Error;

#[derive(Debug)]
pub(crate) struct ReaderPart<Rx, const N: usize> {
    rx: Rx,
    buf: Vec<u8, N>,
}

impl<Rx, const N: usize> ReaderPart<Rx, N> {
    pub fn buf(&self) -> &Vec<u8, N> {
        &self.buf
    }

    pub fn buf_mut(&mut self) -> &mut Vec<u8, N> {
        &mut self.buf
    }
}

impl<Rx, const N: usize> ReaderPart<Rx, N>
where
    Rx: serial::Read<u8> + 'static,
{
    pub fn new(rx: Rx) -> Self {
        Self {
            rx,
            buf: Vec::new(),
        }
    }

    pub fn read_byte(&mut self) -> nb::Result<u8, crate::Error> {
        self.rx.read().map_err(|x| x.map(|_| Error::ReadBuffer))
    }

    pub fn read_bytes(&mut self) -> nb::Result<(), crate::Error> {
        loop {
            if self.buf.is_full() {
                return Err(nb::Error::WouldBlock);
            }

            let byte = self.read_byte()?;
            // Safety: we have already checked if this buffer is full,
            // a couple of lines above.
            unsafe {
                self.buf.push_unchecked(byte);
            }
        }
    }
}

/// Buffer with the incoming data received from the module over the serial port.
///
/// A user should handle this data, otherwise, it will be discarded.
pub struct ReadData<'a, const N: usize> {
    inner: &'a mut Vec<u8, N>,
    from: usize,
    to: usize,
}

struct PrintAscii<'a>(&'a [u8]);

impl<'a> fmt::Debug for PrintAscii<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        for byte in self.0 {
            f.write_char(*byte as char)?;
        }
        f.write_char('"')?;

        Ok(())
    }
}

impl<'a, const N: usize> fmt::Debug for ReadData<'a, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadData")
            .field("from", &self.from)
            .field("to", &self.to)
            .field("data", &PrintAscii(self.as_ref()))
            .finish()
    }
}

impl<'a, const N: usize> ReadData<'a, N> {
    pub(crate) fn new(inner: &'a mut Vec<u8, N>) -> Self {
        let to = inner.len();
        Self { inner, from: 0, to }
    }

    pub(crate) fn subslice(&mut self, from: usize, to: usize) {
        self.from = from;
        self.to = to;
    }
}

impl<'a, const N: usize> AsRef<[u8]> for ReadData<'a, N> {
    fn as_ref(&self) -> &[u8] {
        &self.inner[self.from..self.to]
    }
}

impl<'a, const N: usize> Drop for ReadData<'a, N> {
    fn drop(&mut self) {
        self.inner.clear()
    }
}

impl<'a, const N: usize> Deref for ReadData<'a, N> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}
