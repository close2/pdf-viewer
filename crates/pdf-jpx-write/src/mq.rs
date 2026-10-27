//! The MQ arithmetic encoder of ISO/IEC 15444-1 Annex C.2, the coder every coding pass of
//! Annex D writes through.
//!
//! Annex C.2 is informative — the standard fixes the decoder (C.3) and lets an encoder be
//! anything whose output that decoder reads — and this is its procedures as the figures draw
//! them: `INITENC` (C.2.8), `CODEMPS` and `CODELPS` (C.2.5), `RENORME` (C.2.6), `BYTEOUT` with its
//! bit stuffing after a `0xFF` (C.2.7) and `FLUSH` (C.2.9), over ISO/IEC 15444-1 Table C.2's
//! probability states.

/// ISO/IEC 15444-1 Table C.2, one row per state: the probability estimate `Qe`, the state after
/// a more probable symbol, the state after a less probable one, and whether a less probable symbol
/// exchanges the sense of the more probable one.
const STATES: [(u32, u8, u8, bool); 47] = [
    (0x5601, 1, 1, true),
    (0x3401, 2, 6, false),
    (0x1801, 3, 9, false),
    (0x0AC1, 4, 12, false),
    (0x0521, 5, 29, false),
    (0x0221, 38, 33, false),
    (0x5601, 7, 6, true),
    (0x5401, 8, 14, false),
    (0x4801, 9, 14, false),
    (0x3801, 10, 14, false),
    (0x3001, 11, 17, false),
    (0x2401, 12, 18, false),
    (0x1C01, 13, 20, false),
    (0x1601, 29, 21, false),
    (0x5601, 15, 14, true),
    (0x5401, 16, 14, false),
    (0x5101, 17, 15, false),
    (0x4801, 18, 16, false),
    (0x3801, 19, 17, false),
    (0x3401, 20, 18, false),
    (0x3001, 21, 19, false),
    (0x2801, 22, 19, false),
    (0x2401, 23, 20, false),
    (0x2201, 24, 21, false),
    (0x1C01, 25, 22, false),
    (0x1801, 26, 23, false),
    (0x1601, 27, 24, false),
    (0x1401, 28, 25, false),
    (0x1201, 29, 26, false),
    (0x1101, 30, 27, false),
    (0x0AC1, 31, 28, false),
    (0x09C1, 32, 29, false),
    (0x08A1, 33, 30, false),
    (0x0521, 34, 31, false),
    (0x0441, 35, 32, false),
    (0x02A1, 36, 33, false),
    (0x0221, 37, 34, false),
    (0x0141, 38, 35, false),
    (0x0111, 39, 36, false),
    (0x0085, 40, 37, false),
    (0x0049, 41, 38, false),
    (0x0025, 42, 39, false),
    (0x0015, 43, 40, false),
    (0x0009, 44, 41, false),
    (0x0005, 45, 42, false),
    (0x0001, 45, 43, false),
    (0x5601, 46, 46, false),
];

/// The contexts Annex D codes with: nine of significance, five of sign, three of magnitude
/// refinement, the run-length context and the uniform one (D.3, ISO/IEC 15444-1 Table D.7).
pub(crate) const CONTEXTS: usize = 19;
/// D.3.4's run-length context.
pub(crate) const RUN: usize = 17;
/// D.3.4's uniform context, which codes a run's position.
pub(crate) const UNIFORM: usize = 18;

/// One code-block's arithmetic coder: the registers of C.2.1 and the output so far.
pub(crate) struct Encoder {
    /// The interval register `A`, sixteen bits.
    interval: u32,
    /// The code register `C`: the carry bit, the eight output bits, three spacer bits and the
    /// sixteen fraction bits (ISO/IEC 15444-1 Table C.1).
    code: u32,
    /// `CT`, the shifts left before a byte is complete.
    count: u32,
    /// The bytes, with the byte before `BPST` first — C.2.8's `B` before any byte is written,
    /// which is never part of the result.
    bytes: Vec<u8>,
    /// Each context's state index and its more probable symbol.
    contexts: [(u8, u8); CONTEXTS],
}

impl Encoder {
    /// `INITENC` (C.2.8, Figure C.10), with ISO/IEC 15444-1 Table D.7's initial states: every
    /// context at state 0 with a more probable symbol of 0, except the uniform context at 46,
    /// the run-length context at 3 and the significance context of no significant neighbour at 4.
    pub(crate) fn new() -> Self {
        let mut contexts = [(0, 0); CONTEXTS];
        contexts[0] = (4, 0);
        contexts[RUN] = (3, 0);
        contexts[UNIFORM] = (46, 0);
        Self {
            interval: 0x8000,
            code: 0,
            // The byte before BPST is zero, not 0xFF, so no spurious stuff and CT stays 12.
            count: 12,
            bytes: vec![0],
            contexts,
        }
    }

    /// Codes decision `bit` in context `context` (C.2.4's `ENCODE`).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "A holds at least 0x8000 on entry (RENORME's invariant) and Qe is at most \
                  0x5601, so the subtraction cannot wrap; C stays below 2^28 between bytes \
                  (ISO/IEC 15444-1 Table C.1's register layout), so adding Qe cannot overflow"
    )]
    pub(crate) fn encode(&mut self, context: usize, bit: u8) {
        let (state, more_probable) = self.contexts[context];
        let (qe, next_more, next_less, switch) = STATES[usize::from(state)];
        self.interval -= qe;
        if bit == more_probable {
            // CODEMPS, Figure C.7: the conditional exchange where A fell below Qe.
            if self.interval & 0x8000 == 0 {
                if self.interval < qe {
                    self.interval = qe;
                } else {
                    self.code += qe;
                }
                self.contexts[context].0 = next_more;
                self.renormalise();
            } else {
                self.code += qe;
            }
        } else {
            // CODELPS, Figure C.6, with the conditional exchange.
            if self.interval < qe {
                self.code += qe;
            } else {
                self.interval = qe;
            }
            if switch {
                self.contexts[context].1 = 1 - more_probable;
            }
            self.contexts[context].0 = next_less;
            self.renormalise();
        }
    }

    /// `RENORME` (C.2.6, Figure C.8): doubles A and C until A is at least 0x8000 again, taking
    /// a byte out each time CT reaches zero.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "CT is at least 1 here (BYTEOUT resets it to 7 or 8 at zero), and A and C are \
                  below 2^16 and 2^28 before each doubling"
    )]
    fn renormalise(&mut self) {
        loop {
            self.interval <<= 1;
            self.code <<= 1;
            self.count -= 1;
            if self.count == 0 {
                self.byte_out();
            }
            if self.interval & 0x8000 != 0 {
                break;
            }
        }
    }

    /// `BYTEOUT` (C.2.7, Figure C.9): moves the completed byte out of C, propagating a carry
    /// into the byte before it, and stuffs a bit after a `0xFF` so no marker code appears in
    /// the data.
    #[expect(
        clippy::arithmetic_side_effects,
        clippy::cast_possible_truncation,
        reason = "the byte before is below 0xFF where the carry is added to it; the shifted \
                  values are the register's eight or seven output bits, which fit a byte"
    )]
    fn byte_out(&mut self) {
        // The vector always holds at least the byte before BPST.
        let last = self.bytes.len() - 1;
        if self.bytes[last] == 0xFF {
            self.bytes.push((self.code >> 20) as u8);
            self.code &= 0xF_FFFF;
            self.count = 7;
        } else if self.code < 0x800_0000 {
            self.bytes.push((self.code >> 19) as u8);
            self.code &= 0x7_FFFF;
            self.count = 8;
        } else {
            self.bytes[last] += 1;
            if self.bytes[last] == 0xFF {
                self.code &= 0x7FF_FFFF;
                self.bytes.push((self.code >> 20) as u8);
                self.code &= 0xF_FFFF;
                self.count = 7;
            } else {
                self.bytes.push((self.code >> 19) as u8);
                self.code &= 0x7_FFFF;
                self.count = 8;
            }
        }
    }

    /// `FLUSH` (C.2.9, Figure C.11) with `SETBITS` (Figure C.12), and the coded bytes.
    ///
    /// SETBITS sets as many of C's low bits as the interval allows, two bytes are taken out,
    /// and a final `0xFF` is discarded. The byte before `BPST` is dropped: C.2.8 has BP point
    /// at it, and the interval's upper bound is below the carry bit at the first `BYTEOUT`, so
    /// no carry ever reaches it.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "C + A is below 2^28 (ISO/IEC 15444-1 Table C.1), and CT is at most 12 with C \
                  below 2^20 after a byte is taken out"
    )]
    pub(crate) fn finish(mut self) -> Vec<u8> {
        let bound = self.code + self.interval;
        self.code |= 0xFFFF;
        if self.code >= bound {
            self.code -= 0x8000;
        }
        self.code <<= self.count;
        self.byte_out();
        self.code <<= self.count;
        self.byte_out();
        if self.bytes.last() == Some(&0xFF) {
            self.bytes.pop();
        }
        self.bytes.split_off(1)
    }
}
