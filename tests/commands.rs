use std::{
    io::Write,
    net::{IpAddr, SocketAddr, TcpStream},
    str::FromStr,
};

use assert_matches::assert_matches;
use esp8266_wifi_serial::{JoinApConfig, NetworkEvent, SoftApConfig, WifiMode};

use common::default_esp8266_serial_module;

mod common;

#[test]
fn test_module_init() -> anyhow::Result<()> {
    default_esp8266_serial_module().map(drop)
}

#[test]
fn test_module_softap() {
    let module = default_esp8266_serial_module().expect("unable to create module");

    let mut session = SoftApConfig {
        ssid: "test_network",
        password: "12345678",
        channel: 4,
        mode: WifiMode::Open,
    }
    .start(module)
    .expect("unable to start network sesstion");

    session.listen(2048).unwrap();
}

#[test]
fn test_module_joinap() {
    let module = default_esp8266_serial_module().expect("unable to create module");

    let mut session = JoinApConfig {
        ssid: "test_network",
        password: "12345678",
    }
    .join(module)
    .expect("unable to start network sesstion");

    session.listen(2048).unwrap();

    let info = session.get_info().unwrap();

    let addr = SocketAddr::new(
        IpAddr::from_str(&info.listen_address.to_string()).unwrap(),
        2048,
    );
    let mut socket = TcpStream::connect(addr).expect("unable to connect with device");

    assert_matches!(
        nb::block!(session.poll_network_event()).expect("unable to poll network event"),
        NetworkEvent::Connected { .. }
    );

    let msg = b"Hello esp8266\n";
    socket.write_all(msg).unwrap();

    assert_matches!(
        nb::block!(session.poll_network_event()).expect("unable to poll network event"),
        NetworkEvent::DataAvailable { data, .. } => {
            assert_eq!(data.as_ref(), msg);
        }
    );
}
