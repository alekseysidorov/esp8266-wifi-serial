use std::{cell::{Ref, RefCell, RefMut}, fmt::Debug, io, rc::Rc, sync::{Mutex, MutexGuard}};

use embedded_hal::serial::{Read, Write};
use esp8266_wifi_serial::{clock::SimpleClock, Module};
use once_cell::sync::Lazy;
use serialport::SerialPort;

const BAUD_RATE: u32 = 115200;
const ADAPTER_BUF_CAPACITY: usize = 2048;

const DEFAULT_TIMEOUT_US: u64 = 15_000_000;
const RESET_TIMEOUT_US: u64 = 500_000;

struct ClockImpl;

static ONCE_LOCK: Lazy<Mutex<()>> = Lazy::new(Mutex::default);

pub fn from_debug(err: impl Debug) -> anyhow::Error {
    anyhow::format_err!("{:?}", err)
}

impl SimpleClock for ClockImpl {
    fn now_us(&self) -> u64 {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();

        time.as_micros() as u64
    }
}

#[derive(Clone)]
struct SerialPortWrapper {
    guard: Rc<MutexGuard<'static, ()>>,
    inner: Rc<RefCell<Box<dyn SerialPort>>>,
}

impl SerialPortWrapper {
    fn new(port: Box<dyn SerialPort>) -> Self {
        Self {
            guard: Rc::new(ONCE_LOCK.lock().unwrap()),
            inner: Rc::new(RefCell::new(port)),
        }
    }

    fn borrow(&self) -> Ref<Box<dyn SerialPort>> {
        self.inner.borrow()
    }

    fn borrow_mut(&self) -> RefMut<Box<dyn SerialPort>> {
        self.inner.borrow_mut()
    }
}

impl Read<u8> for SerialPortWrapper {
    type Error = io::Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let bytes_available = self
            .borrow()
            .bytes_to_read()
            .map_err(io::Error::from)
            .map_err(nb::Error::from)?;

        if bytes_available == 0 {
            return Err(nb::Error::WouldBlock);
        }

        let mut buf = [0; 1];
        self.borrow_mut().read(&mut buf).map_err(nb::Error::from)?;
        eprint!("{}", buf[0] as char);
        Ok(buf[0])
    }
}

impl Write<u8> for SerialPortWrapper {
    type Error = io::Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        eprint!("{}", word as char);
        self.borrow_mut()
            .write(&[word])
            .map_err(nb::Error::Other)
            .map(drop)
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        self.borrow_mut().flush().map_err(nb::Error::Other)
    }
}

fn default_serial_port() -> anyhow::Result<SerialPortWrapper> {
    let info = serialport::available_ports()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::format_err!("There is no available serial device."))?;

    let port = SerialPortWrapper::new(serialport::new(info.port_name, BAUD_RATE).open()?);
    Ok(port)
}

pub fn default_esp8266_serial_module() -> anyhow::Result<
    Module<
        impl Read<u8, Error = io::Error>,
        impl Write<u8, Error = io::Error>,
        impl SimpleClock,
        ADAPTER_BUF_CAPACITY,
    >,
> {
    let rx = default_serial_port()?;
    let tx = rx.clone();

    let mut module = Module::new(rx, tx, ClockImpl).map_err(from_debug)?;
    module.set_timeout(Some(RESET_TIMEOUT_US));
    module.reset().map_err(from_debug)?;
    module.set_timeout(Some(DEFAULT_TIMEOUT_US));

    Ok(module)
}
