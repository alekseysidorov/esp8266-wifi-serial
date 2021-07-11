//! Reader part of the esp8266 WiFi implementation.

use core::ops::{Deref, DerefMut};

use embedded_hal::serial;

use crate::Error;

/// A simple `heapless::Vec` alternative backed by the borrowed bytes slice.
#[derive(Debug)]
pub struct ReadBuf<'a> {
    inner: &'a mut [u8],
    len: usize,
}

impl<'a> ReadBuf<'a> {
    pub fn new(inner: &'a mut [u8]) -> Self {
        Self { inner, len: 0 }
    }

    pub fn push(&mut self, byte: u8) -> Result<(), Error> {
        if self.len == self.inner.len() {
            return Err(Error::BufferFull);
        }

        // Safety: we have already checked if this buffer is full,
        // a couple of lines above.
        unsafe {
            self.push_unchecked(byte);
        }
        Ok(())
    }

    pub unsafe fn push_unchecked(&mut self, byte: u8) {
        *self.inner.get_unchecked_mut(self.len) = byte;
        self.len += 1;
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn is_full(&self) -> bool {
        self.len == self.inner.len()
    }

    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }
}

impl<'a> AsRef<[u8]> for ReadBuf<'a> {
    fn as_ref(&self) -> &[u8] {
        &self.inner[0..self.len]
    }
}

impl<'a> AsMut<[u8]> for ReadBuf<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.inner[0..self.len]
    }
}

impl<'a> Deref for ReadBuf<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a> DerefMut for ReadBuf<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

#[derive(Debug)]
pub(crate) struct ReaderPart<'a, Rx> {
    rx: Rx,
    buf: ReadBuf<'a>,
}

impl<'a, Rx> ReaderPart<'a, Rx> {
    pub fn buf(&self) -> &ReadBuf<'a> {
        &self.buf
    }

    pub fn buf_mut(&mut self) -> &mut ReadBuf<'a> {
        &mut self.buf
    }
}

impl<'a, Rx> ReaderPart<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    pub fn new(rx: Rx, buf: &'a mut [u8]) -> Self {
        Self {
            rx,
            buf: ReadBuf::new(buf),
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

    pub fn clear(&mut self) {
        self.buf.clear()
    }
}

/// Buffer with the incoming data received from the module over the serial port.
///
/// A user should handle this data, otherwise, it will be discarded.
pub struct ReadData<'a> {
    pub(crate) inner: &'a mut ReadBuf<'a>,
}

impl<'a> AsRef<[u8]> for ReadData<'a> {
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl<'a> Drop for ReadData<'a> {
    fn drop(&mut self) {
        self.inner.clear()
    }
}

impl<'a> Deref for ReadData<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}
