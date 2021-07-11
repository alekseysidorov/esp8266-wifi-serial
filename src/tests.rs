use crate::parser::CommandResponse;

#[test]
fn test_parse_connect() {
    let raw = b"1,CONNECT\r\n";
    let event = CommandResponse::parse(raw.as_ref()).unwrap().1;

    assert_eq!(event, CommandResponse::Connected { link_id: 1 })
}

#[test]
fn test_parse_close() {
    let raw = b"1,CLOSED\r\n";
    let event = CommandResponse::parse(raw.as_ref()).unwrap().1;

    assert_eq!(event, CommandResponse::Closed { link_id: 1 })
}

#[test]
fn test_parse_data_available() {
    let raw = b"+IPD,12,6:hello\r\n";
    let event = CommandResponse::parse(raw.as_ref()).unwrap().1;

    assert_eq!(
        event,
        CommandResponse::DataAvailable {
            link_id: 12,
            size: 6
        }
    )
}
