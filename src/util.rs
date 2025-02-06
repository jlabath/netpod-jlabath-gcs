use crate::Request;
use anyhow::{anyhow, Result};
use bendy::decoding::FromBencode;

pub fn decode_request(buffer: &[u8]) -> Result<Request> {
    // Check if the last byte is `e` (ASCII value for 'e') which marks dictionary termination
    if buffer[buffer.len() - 1] == b'e' {
        Request::from_bencode(buffer).map_err(|e| anyhow!("{}", e))
    } else {
        Err(anyhow!("keep reading"))
    }
}
