use std::fmt;

use thiserror::Error;

const TOKEN_BYTES: usize = 32;
const TOKEN_HEX_LEN: usize = TOKEN_BYTES * 2;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    #[error("operating-system entropy source is unavailable")]
    EntropyUnavailable,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum TokenParseError {
    #[error("token must contain exactly {TOKEN_HEX_LEN} hexadecimal characters")]
    InvalidLength,
    #[error("token contains a non-hexadecimal character")]
    InvalidHex,
}

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, PartialEq, Eq, Hash)]
        pub struct $name([u8; TOKEN_BYTES]);

        impl $name {
            pub fn generate() -> Result<Self, IdError> {
                let mut bytes = [0_u8; TOKEN_BYTES];
                getrandom::fill(&mut bytes).map_err(|_| IdError::EntropyUnavailable)?;
                Ok(Self(bytes))
            }

            pub fn from_token(token: &str) -> Result<Self, TokenParseError> {
                decode_token(token).map(Self)
            }

            #[must_use]
            pub fn to_token(&self) -> String {
                encode_token(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(REDACTED)"))
            }
        }
    };
}

opaque_id!(SessionHandle);
opaque_id!(TaskLeaseId);
opaque_id!(ActionId);
opaque_id!(JobId);

fn encode_token(bytes: &[u8; TOKEN_BYTES]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(TOKEN_HEX_LEN);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn decode_token(token: &str) -> Result<[u8; TOKEN_BYTES], TokenParseError> {
    if token.len() != TOKEN_HEX_LEN {
        return Err(TokenParseError::InvalidLength);
    }

    let mut output = [0_u8; TOKEN_BYTES];
    let bytes = token.as_bytes();
    for (index, slot) in output.iter_mut().enumerate() {
        let high = decode_nibble(bytes[index * 2])?;
        let low = decode_nibble(bytes[index * 2 + 1])?;
        *slot = (high << 4) | low;
    }
    Ok(output)
}

fn decode_nibble(value: u8) -> Result<u8, TokenParseError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(TokenParseError::InvalidHex),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trip_is_lossless() {
        let handle = SessionHandle::generate().expect("OS entropy should be available in tests");
        let encoded = handle.to_token();
        let decoded = SessionHandle::from_token(&encoded).expect("generated token must parse");
        assert_eq!(handle, decoded);
        assert_eq!(encoded.len(), TOKEN_HEX_LEN);
    }

    #[test]
    fn debug_never_exposes_token() {
        let handle = SessionHandle::from_token(&"ab".repeat(TOKEN_BYTES))
            .expect("fixed test token must parse");
        let rendered = format!("{handle:?}");
        assert_eq!(rendered, "SessionHandle(REDACTED)");
        assert!(!rendered.contains("abab"));
    }

    #[test]
    fn malformed_tokens_fail_closed() {
        assert_eq!(
            SessionHandle::from_token("abc").expect_err("short token must fail"),
            TokenParseError::InvalidLength
        );
        assert_eq!(
            SessionHandle::from_token(&"zz".repeat(TOKEN_BYTES))
                .expect_err("non-hex token must fail"),
            TokenParseError::InvalidHex
        );
    }

    #[test]
    fn job_ids_are_opaque_and_round_trip() {
        let job = JobId::generate().expect("OS entropy should be available in tests");
        let token = job.to_token();
        assert_eq!(
            JobId::from_token(&token).expect("job token must parse"),
            job
        );
        assert_eq!(format!("{job:?}"), "JobId(REDACTED)");
    }
}
