use nom::{alt, char, character::streaming::digit1, do_parse, named, opt, tag, IResult};

use crate::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResponse {
    Connected { link_id: usize },
    Closed { link_id: usize },
    DataAvailable { link_id: usize, size: usize },
    WifiDisconnect,
}

fn atoi(digits: &[u8]) -> Option<usize> {
    let mut num: usize = 0;
    let len = digits.len();

    for (i, digit) in digits.iter().enumerate() {
        let digit = (*digit as char).to_digit(10)? as usize;
        let mut exp = 1;
        for _ in 0..(len - i - 1) {
            exp *= 10;
        }
        num += exp * digit;
    }
    Some(num)
}

fn parse_usize(input: &[u8]) -> IResult<&[u8], usize> {
    let (input, digits) = digit1(input)?;
    let num = atoi(digits).unwrap();
    IResult::Ok((input, num))
}

fn parse_u8(input: &[u8]) -> IResult<&[u8], u8> {
    let (input, digits) = digit1(input)?;
    let num = atoi(digits).unwrap() as u8;
    IResult::Ok((input, num))
}

named!(crlf, tag!("\r\n"));

named!(
    connected<CommandResponse>,
    do_parse!(
        opt!(crlf)
            >> link_id: parse_usize
            >> tag!(",CONNECT")
            >> crlf
            >> (CommandResponse::Connected { link_id })
    )
);

named!(
    closed<CommandResponse>,
    do_parse!(
        opt!(crlf)
            >> link_id: parse_usize
            >> tag!(",CLOSED")
            >> crlf
            >> (CommandResponse::Closed { link_id })
    )
);

named!(
    data_available<CommandResponse>,
    do_parse!(
        opt!(crlf)
            >> tag!("+IPD,")
            >> link_id: parse_usize
            >> char!(',')
            >> size: parse_usize
            >> char!(':')
            >> opt!(crlf)
            >> (CommandResponse::DataAvailable { link_id, size })
    )
);

named!(
    wifi_disconnect<CommandResponse>,
    do_parse!(
        opt!(crlf) >> tag!("WIFI DISCONNECT") >> opt!(crlf) >> (CommandResponse::WifiDisconnect)
    )
);

named!(
    parse<CommandResponse>,
    alt!(connected | closed | data_available | wifi_disconnect)
);

impl CommandResponse {
    pub fn parse(input: &[u8]) -> Option<(&[u8], Self)> {
        parse(input).ok()
    }
}

pub struct CifsrResponse {
    pub ap_ip: Option<IpAddr>,
    pub sta_ip: IpAddr,
}

named!(
    parse_ip4_addr<IpAddr>,
    do_parse!(
        opt!(crlf)
            >> a: parse_u8
            >> char!('.')
            >> b: parse_u8
            >> char!('.')
            >> c: parse_u8
            >> char!('.')
            >> d: parse_u8
            >> (IpAddr::V4(Ipv4Addr::new(a, b, c, d)))
    )
);

named!(
    parse_apip<IpAddr>,
    do_parse!(
        opt!(crlf)
            >> tag!("+CIFSR:APIP,")
            >> char!('"')
            >> ip_addr: parse_ip4_addr
            >> char!('"')
            >> opt!(crlf)
            >> (ip_addr)
    )
);

named!(
    parse_staip<IpAddr>,
    do_parse!(
        opt!(crlf)
            >> tag!("+CIFSR:STAIP,")
            >> char!('"')
            >> ip_addr: parse_ip4_addr
            >> char!('"')
            >> opt!(crlf)
            >> (ip_addr)
    )
);

named!(
    cifsr_response<CifsrResponse>,
    do_parse!(
        opt!(crlf)
            >> ap_ip: opt!(parse_apip)
            >> sta_ip: parse_staip
            >> (CifsrResponse { ap_ip, sta_ip })
    )
);

impl CifsrResponse {
    pub fn parse(input: &[u8]) -> Option<(&[u8], Self)> {
        cifsr_response(input).ok()
    }
}


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
