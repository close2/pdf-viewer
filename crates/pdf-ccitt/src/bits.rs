//! Reading a fax-coded stream a bit at a time, most significant bit of each byte first.
//!
//! ISO 32000-2 §7.4.6: "CCITT encoding is bit-oriented, not byte-oriented", and "[e]ncoded data
//! shall ordinarily be treated as a continuous, unbroken bit stream". So the reader's position
//! is a bit offset, and a byte boundary matters only where `/EncodedByteAlign` or the end of the
//! data makes it.

/// A position in the encoded bits, and the bits.
#[derive(Debug, Clone)]
pub(crate) struct Bits<'a> {
    data: &'a [u8],
    /// The next bit to read, counted from the first bit of `data`.
    at: usize,
    /// One past the last `1` bit in `data`, so that "only zeros remain" is one comparison: past
    /// this offset there is no codeword left to read, because every codeword and every
    /// end-of-line code holds a `1`.
    last_one: usize,
}

impl<'a> Bits<'a> {
    /// A reader at the first bit of `data`.
    pub(crate) fn new(data: &'a [u8]) -> Self {
        let last_one = data.iter().rposition(|byte| *byte != 0).map_or(0, |index| {
            let byte = data.get(index).copied().unwrap_or(0);
            // The bit after the lowest set bit of the last non-zero byte.
            index
                .saturating_mul(8)
                .saturating_add(8)
                .saturating_sub(byte.trailing_zeros() as usize)
        });
        Self {
            data,
            at: 0,
            last_one,
        }
    }

    /// The bit offset of the next bit.
    pub(crate) fn position(&self) -> usize {
        self.at
    }

    /// Moves to a bit offset a search found.
    pub(crate) fn seek(&mut self, at: usize) {
        self.at = at;
    }

    /// How many bits remain.
    pub(crate) fn remaining(&self) -> usize {
        self.data.len().saturating_mul(8).saturating_sub(self.at)
    }

    /// Whether nothing but zero bits remains.
    pub(crate) fn only_zeros_remain(&self) -> bool {
        self.at >= self.last_one
    }

    /// Whether the data is exhausted: nothing remains but the zero bits that finish the byte the
    /// position is in.
    ///
    /// §7.4.6: "When a filter reaches EOD, it shall always skip to the next byte boundary following
    /// the encoded data" — so the bits up to that boundary are the encoding's padding and not
    /// data. Zero bytes *beyond* it are not padding this clause names, and eight zero bits begin no
    /// codeword, so a decode that meets them meets damage rather than an end.
    pub(crate) fn exhausted(&self) -> bool {
        self.only_zeros_remain() && self.at.next_multiple_of(8) >= self.data.len().saturating_mul(8)
    }

    /// The next `count` bits as an integer, most significant first, with bits past the end of
    /// the data read as zero. `count` is at most 24.
    ///
    /// Reading zeros past the end lets a table lookup at the tail of the data see a full index;
    /// the caller then refuses a codeword whose length runs past [`Self::remaining`], so no bit
    /// that is not in the data is ever *consumed*.
    pub(crate) fn peek(&self, count: u32) -> u32 {
        let first = self.at / 8;
        let mut window = 0_u32;
        for offset in 0..4 {
            let byte = self
                .data
                .get(first.saturating_add(offset))
                .copied()
                .unwrap_or(0);
            window = (window << 8) | u32::from(byte);
        }
        // `at % 8` is below eight and `count` at most 24, so the shift is in 1..=32 and the
        // window of 32 bits always holds `count` bits after the offset.
        let offset = u32::try_from(self.at % 8).unwrap_or(0);
        let kept = window << offset;
        kept.checked_shr(32_u32.saturating_sub(count)).unwrap_or(0)
    }

    /// Consumes `count` bits the caller has already looked at.
    pub(crate) fn skip(&mut self, count: u32) {
        self.at = self.at.saturating_add(count as usize);
    }

    /// Reads one bit.
    pub(crate) fn bit(&mut self) -> Option<bool> {
        if self.remaining() == 0 {
            return None;
        }
        let one = self.peek(1) == 1;
        self.skip(1);
        Some(one)
    }

    /// Moves to the next byte boundary, if not already on one.
    pub(crate) fn align(&mut self) {
        self.at = self.at.next_multiple_of(8);
    }

    /// How many zero bits begin at the current position, counting to the end of the data.
    pub(crate) fn zeros(&self) -> usize {
        self.zeros_from(self.at)
    }

    /// How many zero bits begin at `from`.
    fn zeros_from(&self, from: usize) -> usize {
        let mut at = from;
        let end = self.data.len().saturating_mul(8);
        while at < end {
            let byte = self.data.get(at / 8).copied().unwrap_or(0);
            let within = at % 8;
            // The byte's bits from `within` on, shifted up to the top.
            let rest = byte << within;
            if rest != 0 {
                return at
                    .saturating_sub(from)
                    .saturating_add(rest.leading_zeros() as usize);
            }
            at = at.saturating_add(8_usize.saturating_sub(within));
        }
        end.saturating_sub(from)
    }

    /// Where the next end-of-line pattern at or after `from` begins, counting its fill: the start
    /// of the first run of eleven or more zeros that a `1` follows.
    ///
    /// T.4 section 4.1.2 builds the end-of-line code as a sequence no valid run of codewords can
    /// contain, and §7.4.6 makes it what a damaged row's end is found by — "locating its end in the
    /// encoded data by searching for an `EndOfLine` pattern". `None` where the data holds none.
    pub(crate) fn next_end_of_line(&self, from: usize) -> Option<usize> {
        let end = self.data.len().saturating_mul(8);
        let mut at = from;
        while at < end {
            let zeros = self.zeros_from(at);
            let after = at.saturating_add(zeros);
            if after >= end {
                return None;
            }
            if zeros >= 11 {
                return Some(at);
            }
            // Past the run and the `1` that ended it.
            at = after.saturating_add(1);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek_reads_across_bytes_and_zeros_past_the_end() {
        let mut bits = Bits::new(&[0b1010_0000, 0b0000_0001]);
        assert_eq!(bits.peek(3), 0b101);
        bits.skip(2);
        assert_eq!(bits.peek(14), 0b10_0000_0000_0001);
        bits.skip(14);
        assert_eq!(bits.remaining(), 0);
        assert_eq!(bits.peek(8), 0);
    }

    #[test]
    fn the_data_is_exhausted_at_the_padding_of_its_last_byte() {
        let mut bits = Bits::new(&[0b0000_0100]);
        assert!(!bits.exhausted());
        bits.skip(6);
        assert!(bits.only_zeros_remain() && bits.exhausted());

        let mut bits = Bits::new(&[0b0000_0100, 0]);
        bits.skip(6);
        assert!(bits.only_zeros_remain() && !bits.exhausted());
        assert!(Bits::new(&[]).exhausted());
    }

    #[test]
    fn an_end_of_line_is_eleven_zeros_and_a_one() {
        // 1, then ten zeros and a one (not one), then twelve zeros and a one.
        let data = [0b1000_0000, 0b0001_0000, 0b0000_0000, 0b0100_0000];
        let bits = Bits::new(&data);
        assert_eq!(bits.next_end_of_line(0), Some(12));
        assert_eq!(bits.zeros(), 0);
        assert_eq!(Bits::new(&[0, 0]).next_end_of_line(0), None);
    }
}
