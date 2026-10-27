//! Packets, ISO/IEC 15444-1 Annex B.9 and B.10: a precinct's code-blocks announced in a packet
//! header — inclusion and missing bit planes by tag tree (B.10.2), passes by ISO/IEC 15444-1
//! Table B.4's codewords, lengths under `Lblock` (B.10.7) — and their codeword segments after it.
//!
//! One quality layer, so a code-block with any coded pass is included in layer 0 and its
//! inclusion tag tree is 0 at its leaf; a code-block with none is never included, which the
//! tree states by a leaf of 1, the layer past the last.

use crate::tier1::CodedBlock;

/// B.10.1's packet-header bit writer: bits most significant first, and after a `0xFF` byte the
/// next carries a stuffed zero bit in its most significant place.
#[derive(Default)]
struct Bits {
    /// The bytes completed.
    bytes: Vec<u8>,
    /// The byte being filled.
    current: u8,
    /// Bits in [`Self::current`].
    filled: u32,
}

impl Bits {
    /// Bits a byte holds after this one: seven after a `0xFF`, eight otherwise.
    fn room(&self) -> u32 {
        if self.bytes.last() == Some(&0xFF) {
            7
        } else {
            8
        }
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`filled` is below `room`, at most 8, before it is incremented"
    )]
    fn put(&mut self, bit: bool) {
        self.current = (self.current << 1) | u8::from(bit);
        self.filled += 1;
        if self.filled == self.room() {
            self.bytes.push(self.current);
            self.current = 0;
            self.filled = 0;
        }
    }

    /// `value`'s low `count` bits, most significant first.
    fn put_value(&mut self, value: u32, count: u32) {
        for bit in (0..count).rev() {
            self.put((value >> bit) & 1 == 1);
        }
    }

    /// The header's bytes: the last byte padded with zero bits, and a zero byte after a final
    /// `0xFF`, because B.10.1 forbids a packet header to end in one.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`filled` is below `room` here"
    )]
    fn finish(mut self) -> Vec<u8> {
        if self.filled > 0 {
            let room = self.room();
            self.bytes.push(self.current << (room - self.filled));
        }
        if self.bytes.last() == Some(&0xFF) {
            self.bytes.push(0);
        }
        self.bytes
    }
}

/// A tag tree of B.10.2 over a grid of code-blocks: each node the minimum of the four below it,
/// and what of it the decoder has been told.
struct TagTree {
    /// Each level's width, leaves first.
    widths: Vec<usize>,
    /// Each level's first node in [`Self::nodes`].
    starts: Vec<usize>,
    /// Every node: its value, the lower bound sent, and whether the value is known.
    nodes: Vec<(u32, u32, bool)>,
}

impl TagTree {
    /// A tree over `values`, `width` leaves a row.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "levels halve a grid of at most 512 × 512 code-blocks (a 32 768 precinct of \
                  64-sample code-blocks), so every index is small"
    )]
    fn new(values: &[u32], width: usize) -> Self {
        let mut widths = vec![width];
        let mut heights = vec![values.len() / width];
        let mut starts = vec![0];
        let mut nodes: Vec<(u32, u32, bool)> =
            values.iter().map(|value| (*value, 0, false)).collect();
        while widths[widths.len() - 1] > 1 || heights[heights.len() - 1] > 1 {
            let (below_width, below_height) =
                (widths[widths.len() - 1], heights[heights.len() - 1]);
            let below = starts[starts.len() - 1];
            let (level_width, level_height) = (below_width.div_ceil(2), below_height.div_ceil(2));
            starts.push(nodes.len());
            for row in 0..level_height {
                for col in 0..level_width {
                    let mut least = u32::MAX;
                    for (r, c) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                        let (r, c) = (2 * row + r, 2 * col + c);
                        if r < below_height && c < below_width {
                            least = least.min(nodes[below + r * below_width + c].0);
                        }
                    }
                    nodes.push((least, 0, false));
                }
            }
            widths.push(level_width);
            heights.push(level_height);
        }
        Self {
            widths,
            starts,
            nodes,
        }
    }

    /// Codes leaf `leaf` against `threshold` (B.10.2): from the root down, each node's value is
    /// sent as zeros up to it and a one, as far as the threshold asks and no further than an
    /// earlier call already went.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "indices halve within the tree; `low` is below `threshold` when incremented"
    )]
    fn encode(&mut self, bits: &mut Bits, leaf: usize, threshold: u32) {
        let mut path = Vec::with_capacity(self.widths.len());
        let (mut col, mut row) = (leaf % self.widths[0], leaf / self.widths[0]);
        for level in 0..self.widths.len() {
            path.push(self.starts[level] + row * self.widths[level] + col);
            col /= 2;
            row /= 2;
        }
        let mut low = 0;
        for &node in path.iter().rev() {
            let (value, sent, known) = self.nodes[node];
            low = low.max(sent);
            let mut known = known;
            while low < threshold {
                if low >= value {
                    if !known {
                        bits.put(true);
                        known = true;
                    }
                    break;
                }
                bits.put(false);
                low += 1;
            }
            self.nodes[node] = (value, low, known);
        }
    }
}

/// ISO/IEC 15444-1 Table B.4's codeword for `passes` coding passes, 1 to 164.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "each branch subtracts the least count it holds"
)]
fn put_passes(bits: &mut Bits, passes: u32) {
    match passes {
        1 => bits.put(false),
        2 => bits.put_value(0b10, 2),
        3..=5 => bits.put_value(0b1100 | (passes - 3), 4),
        6..=36 => bits.put_value((0b1111 << 5) | (passes - 6), 9),
        _ => bits.put_value((0b1_1111_1111 << 7) | (passes - 37), 16),
    }
}

/// One packet: the header announcing `blocks`, a `columns`-wide grid of one precinct's
/// code-blocks, and their data (B.9). `most` is the band's `Mb` (E.1.1.1), from which each
/// included block's missing bit planes are counted.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a code-block's planes are at most `most`, and its length and `Lblock` are \
              bounded by a 64 × 64 block's data"
)]
pub(crate) fn packet(blocks: &[CodedBlock], columns: usize, most: u32) -> Vec<u8> {
    let mut bits = Bits::default();
    if blocks.iter().all(|block| block.passes == 0) {
        // B.10.3: a zero bit is a packet with no code-block in it.
        bits.put(false);
        return bits.finish();
    }
    bits.put(true);
    let inclusion: Vec<u32> = blocks
        .iter()
        .map(|block| u32::from(block.passes == 0))
        .collect();
    let missing: Vec<u32> = blocks
        .iter()
        .map(|block| most.saturating_sub(block.planes))
        .collect();
    let mut inclusion = TagTree::new(&inclusion, columns);
    let mut missing_tree = TagTree::new(&missing, columns);
    for (index, block) in blocks.iter().enumerate() {
        inclusion.encode(&mut bits, index, 1);
        if block.passes == 0 {
            continue;
        }
        missing_tree.encode(&mut bits, index, missing[index] + 1);
        put_passes(&mut bits, block.passes);
        // B.10.7.1: the length takes Lblock + ⌊log2 passes⌋ bits, and Lblock, 3 at first, is
        // raised by the count of one bits before a zero.
        let length = u32::try_from(block.data.len()).unwrap_or(u32::MAX);
        let needed = u32::BITS - length.leading_zeros();
        let given = 3 + (u32::BITS - 1 - block.passes.leading_zeros());
        for _ in given..needed {
            bits.put(true);
        }
        bits.put(false);
        bits.put_value(length, given.max(needed));
    }
    let mut out = bits.finish();
    for block in blocks {
        out.extend_from_slice(&block.data);
    }
    out
}
