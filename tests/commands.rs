use std::{
    io::Write,
    net::{IpAddr, SocketAddr, TcpStream},
    str::FromStr,
    time::Duration,
};

use assert_matches::assert_matches;
use esp8266_wifi_serial::{JoinApConfig, NetworkEvent, SoftApConfig, WifiMode};

use common::default_esp8266_serial_module;

use crate::common::necessary_env_var;

mod common;

#[test]
#[cfg_attr(
    not(feature = "integration_tests"),
    ignore = "feature \"integration_tests\" is disabled."
)]
fn integration_test_init() -> anyhow::Result<()> {
    default_esp8266_serial_module().map(drop)
}

#[test]
#[cfg_attr(
    not(feature = "integration_tests"),
    ignore = "feature \"integration_tests\" is disabled."
)]
fn integration_test_softap() {
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
#[cfg_attr(
    not(feature = "integration_tests"),
    ignore = "feature \"integration_tests\" is disabled."
)]
fn integration_test_joinap_ok() {
    let module = default_esp8266_serial_module().expect("unable to create module");

    let mut session = JoinApConfig {
        ssid: &necessary_env_var("ESP8266_WIFI_SERIAL_SSID"),
        password: &necessary_env_var("ESP8266_WIFI_SERIAL_PASSWORD"),
    }
    .join(module)
    .expect("unable to start network sesstion");

    session.listen(2048).unwrap();
    let info = session.get_info().unwrap();
    // Get some time to a module to establish a TCP listener.
    std::thread::sleep(Duration::from_millis(50));

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

#[test]
#[cfg_attr(
    not(feature = "integration_tests"),
    ignore = "feature \"integration_tests\" is disabled."
)]
fn integration_test_joinap_fail() {
    let module = default_esp8266_serial_module().expect("unable to create module");

    let err = JoinApConfig {
        ssid: "some weird network",
        password: "my password aaaa",
    }
    .join(module)
    .expect_err("joining to the AP should fail");

    assert_eq!(err, esp8266_wifi_serial::Error::JoinApError);
}
