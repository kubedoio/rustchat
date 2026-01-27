use uuid::Uuid;

const MM_ID_ALPHABET: &[u8; 32] = b"ybndrfg8ejkmcpqxot1uwisza345h769";

pub fn encode_mm_id(uuid: Uuid) -> String {
    let bytes = *uuid.as_bytes();
    encode_base32(bytes)
}

pub fn decode_mm_id(id: &str) -> Option<Uuid> {
    if id.len() != 26 {
        return None;
    }

    let bytes = decode_base32(id)?;
    Some(Uuid::from_bytes(bytes))
}

pub fn parse_mm_or_uuid(id: &str) -> Option<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        return Some(uuid);
    }

    decode_mm_id(id)
}

fn encode_base32(bytes: [u8; 16]) -> String {
    let mut output = String::with_capacity(26);
    let mut buffer: u32 = 0;
    let mut bits: u8 = 0;

    for byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;

        while bits >= 5 {
            let index = ((buffer >> (bits - 5)) & 0x1f) as usize;
            output.push(MM_ID_ALPHABET[index] as char);
            bits -= 5;
        }
    }

    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1f) as usize;
        output.push(MM_ID_ALPHABET[index] as char);
    }

    output
}

fn decode_base32(input: &str) -> Option<[u8; 16]> {
    let mut buffer: u32 = 0;
    let mut bits: u8 = 0;
    let mut bytes: Vec<u8> = Vec::with_capacity(16);

    for ch in input.chars() {
        let value = decode_char(ch)? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;

        while bits >= 8 {
            let byte = ((buffer >> (bits - 8)) & 0xff) as u8;
            bytes.push(byte);
            bits -= 8;
        }
    }

    if bytes.len() < 16 {
        return None;
    }

    let mut out = [0u8; 16];
    out.copy_from_slice(&bytes[..16]);
    Some(out)
}

fn decode_char(ch: char) -> Option<u8> {
    let lower = ch.to_ascii_lowercase();
    for (idx, byte) in MM_ID_ALPHABET.iter().enumerate() {
        if *byte as char == lower {
            return Some(idx as u8);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_uuid_to_mm_id() {
        let uuid = Uuid::new_v4();
        let encoded = encode_mm_id(uuid);
        let decoded = decode_mm_id(&encoded).expect("decode should succeed");
        assert_eq!(uuid, decoded);
        assert_eq!(encoded.len(), 26);
    }
}
