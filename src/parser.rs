use core::str::FromStr;

use nom::{alt, char, character::streaming::digit1, do_parse, named, opt, tag, IResult};

use crate::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResponse {
    Connected { link_id: u16 },
    Closed { link_id: u16 },
    DataAvailable { link_id: u16, size: u64 },
    WifiDisconnect,
}

fn parse_error(input: &[u8]) -> nom::Err<nom::error::Error<&[u8]>> {
    nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
}

fn atoi<T: FromStr>(input: &[u8]) -> Result<T, nom::Err<nom::error::Error<&[u8]>>> {
    let s = core::str::from_utf8(input).map_err(|_| parse_error(input))?;
    s.parse().map_err(|_| parse_error(input))
}

fn parse_link_id(input: &[u8]) -> IResult<&[u8], u16> {
    let (input, digits) = digit1(input)?;
    let num = atoi(digits)?;
    IResult::Ok((input, num))
}

fn parse_u64(input: &[u8]) -> IResult<&[u8], u64> {
    let (input, digits) = digit1(input)?;
    let num = atoi(digits)?;
    IResult::Ok((input, num))
}

fn parse_u8(input: &[u8]) -> IResult<&[u8], u8> {
    let (input, digits) = digit1(input)?;
    let num = atoi(digits)?;
    IResult::Ok((input, num))
}

named!(crlf, tag!("\r\n"));

named!(
    connected<CommandResponse>,
    do_parse!(
        opt!(crlf)
            >> link_id: parse_link_id
            >> tag!(",CONNECT")
            >> crlf
            >> (CommandResponse::Connected { link_id })
    )
);

named!(
    closed<CommandResponse>,
    do_parse!(
        opt!(crlf)
            >> link_id: parse_link_id
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
            >> link_id: parse_link_id
            >> char!(',')
            >> size: parse_u64
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
    pub sta_ip: Option<IpAddr>,
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
            >> sta_ip: opt!(parse_staip)
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
