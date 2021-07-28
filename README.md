[![Continuous integration](https://github.com/alekseysidorov/esp8266-wifi-serial/actions/workflows/rust.yml/badge.svg)](https://github.com/alekseysidorov/esp8266-wifi-serial/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/esp8266-wifi-serial)](https://crates.io/crates/esp8266-wifi-serial)
[![API reference](https://docs.rs/esp8266-wifi-serial/badge.svg)](https://docs.rs/esp8266-wifi-serial)

# esp8266-wifi-serial

(WIP) Driver to work with the esp8266 module over the serial port.

By using this module you can join the existing access point or creating your own. After a network creation, the module can both listen to incoming TCP connections or connect to other sockets.

```rust
let mut module = Module::new(rx, tx, clock).expect("unable to create module");
// Create a new access point.
let mut session = SoftApConfig {
    ssid: "test_network",
    password: "12345678",
    channel: 4,
    mode: WifiMode::Open,
}
.start(module)
.expect("unable to start network sesstion");
// Start listening for incoming connections on the specified port.
session.listen(2048).unwrap();
// Start an event loop.
loop {
    let event = nb::block!(session.poll_network_event()).expect("unable to poll network event");
    // Some business logic.
}
```

***Warning:** this library is not finished yet and it is not worth using it in mission-critical software, it can burn your hamster.*

The crate was been tested with the `gd32vf103` board.

I will be happy to see new contributions to the development of this crate.
