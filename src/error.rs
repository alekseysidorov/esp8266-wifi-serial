/// Possible error types that may happen during manipulating the WiFi module.
///
/// In order to the crate interface simplification, error details have been omitted.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    /// An error occurred during the receiving bytes from the serial port.
    ReadBuffer,
    /// An error occurred during the sending bytes into the serial port.
    WriteBuffer,
    /// Reader buffer is full.
    BufferFull,
    /// Operation timeout reached.
    Timeout,
    /// Unable to join selected access point.
    JoinApError,
}

/// A specialized result type for the operations with the esp8266 module.
pub type Result<T> = core::result::Result<T, Error>;
