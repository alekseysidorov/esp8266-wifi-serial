#[derive(Debug)]
pub enum Error {
    ReadBuffer,
    WriteBuffer,
    BufferFull,
    Timeout,
}

pub type Result<T> = core::result::Result<T, Error>;
