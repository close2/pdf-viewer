//! How far the worker's page has got, as the notice that carries it across the pipe.
//!
//! A host waiting on `Command::Open` holds the reply's read and nothing else, so the figure a
//! window shows while page one is interpreted (ADR 1811) arrives *inside* that exchange, as
//! `confined_transport::frame::NOTICE` frames the transport hands to a listener. This is their
//! payload: the three numbers of a `pdf_model::progress::Reading`, big-endian, in its field order.
//! What reads it is the privileged side and what wrote it the untrusted one, so a payload of any
//! other length is no figure at all rather than a guess at one.

use pdf_model::progress::Reading;

/// The payload's length: three `u64`.
const LENGTH: usize = 24;

/// A reading as the worker sends it.
pub(crate) fn encode(reading: Reading) -> [u8; LENGTH] {
    let mut bytes = [0u8; LENGTH];
    for (slot, figure) in
        bytes
            .chunks_exact_mut(8)
            .zip([reading.operators, reading.taken, reading.held])
    {
        slot.copy_from_slice(&figure.to_be_bytes());
    }
    bytes
}

/// A reading as the host receives it, or `None` for a payload that is not one.
pub(crate) fn decode(bytes: &[u8]) -> Option<Reading> {
    if bytes.len() != LENGTH {
        return None;
    }
    let mut figures = bytes
        .chunks_exact(8)
        .map(|chunk| chunk.try_into().ok().map(u64::from_be_bytes));
    Some(Reading {
        operators: figures.next()??,
        taken: figures.next()??,
        held: figures.next()??,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reading_crosses_whole_and_nothing_else_is_one() {
        let reading = Reading {
            operators: 28_992_250,
            taken: 19_766_752,
            held: 39_533_504,
        };
        assert_eq!(decode(&encode(reading)), Some(reading));
        assert_eq!(decode(&encode(reading)[..23]), None);
        assert_eq!(decode(&[0u8; 25]), None);
    }
}
