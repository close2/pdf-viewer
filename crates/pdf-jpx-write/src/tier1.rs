//! Coefficient bit modelling, ISO/IEC 15444-1 Annex D: one code-block's coefficients coded bit
//! plane by bit plane in the three passes D.3 defines, through the MQ coder of Annex C.
//!
//! Only what this writer's codestream states is built: the LL band of zero decomposition levels
//! (ISO/IEC 15444-1 Table D.1's LL and LH column), code-block style 0 — no bypass, no context
//! reset, one terminated segment, no vertically causal contexts (A.6.1,
//! ISO/IEC 15444-1 Table A.19) — so every pass of a code-block is one codeword segment,
//! terminated once by C.2.9's `FLUSH`.

use crate::mq::{Encoder, RUN, UNIFORM};

/// A code-block's coded data and what a packet header says of it (B.10).
pub(crate) struct CodedBlock {
    /// The codeword segment.
    pub(crate) data: Vec<u8>,
    /// Coding passes in it: none where every coefficient is zero, otherwise 3 × planes − 2.
    pub(crate) passes: u32,
    /// Bit planes from the most significant set bit down, which is `Mb` less B.10.5's missing
    /// most significant bit planes.
    pub(crate) planes: u32,
}

/// A code-block's coefficients in sign-magnitude form (D.2), with the one-coefficient border
/// D.3's neighbourhoods read as insignificant.
struct Block {
    /// Columns.
    width: usize,
    /// Rows.
    height: usize,
    /// Columns of every state array: the block's and the two border columns.
    stride: usize,
    /// Each coefficient's magnitude, at its index in the bordered grid.
    magnitude: Vec<u32>,
    /// Whether the coefficient is negative.
    negative: Vec<bool>,
    /// D.2's significance state.
    significant: Vec<bool>,
    /// Coded in this bit plane's significance propagation pass (D.3.1), so skipped by the two
    /// passes after it.
    visited: Vec<bool>,
    /// Refined at least once (D.3.3), which ISO/IEC 15444-1 Table D.4's context tells apart.
    refined: Vec<bool>,
}

/// Codes one code-block of `width` × `height` coefficients, row by row (D.1).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a code-block is at most 64 × 64 (A.6.1), so the bordered indices and the pass \
              count, at most 3 × 32 − 2, are small"
)]
pub(crate) fn encode_block(coefficients: &[i32], width: usize, height: usize) -> CodedBlock {
    let mut block = Block::new(coefficients, width, height);
    let largest = block.magnitude.iter().copied().max().unwrap_or(0);
    let planes = u32::BITS - largest.leading_zeros();
    if planes == 0 {
        return CodedBlock {
            data: Vec::new(),
            passes: 0,
            planes: 0,
        };
    }
    let mut coder = Encoder::new();
    // D.3: the first plane with a set bit is coded by a cleanup pass alone, and every plane
    // below it by the three passes in order.
    block.cleanup(&mut coder, planes - 1);
    for plane in (0..planes - 1).rev() {
        block.significance_propagation(&mut coder, plane);
        block.magnitude_refinement(&mut coder, plane);
        block.cleanup(&mut coder, plane);
    }
    CodedBlock {
        data: coder.finish(),
        passes: 3 * planes - 2,
        planes,
    }
}

impl Block {
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the block is at most 64 × 64, so every bordered index is small"
    )]
    fn new(coefficients: &[i32], width: usize, height: usize) -> Self {
        let stride = width + 2;
        let cells = stride * (height + 2);
        let mut magnitude = vec![0; cells];
        let mut negative = vec![false; cells];
        for row in 0..height {
            for col in 0..width {
                let value = coefficients[row * width + col];
                let at = (row + 1) * stride + col + 1;
                magnitude[at] = value.unsigned_abs();
                negative[at] = value < 0;
            }
        }
        Self {
            width,
            height,
            stride,
            magnitude,
            negative,
            significant: vec![false; cells],
            visited: vec![false; cells],
            refined: vec![false; cells],
        }
    }

    /// The bordered index of the coefficient at `row`, `col`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "row and col are inside a block of at most 64 × 64"
    )]
    fn at(&self, row: usize, col: usize) -> usize {
        (row + 1) * self.stride + col + 1
    }

    /// The significant neighbours of `at`: horizontal, vertical and diagonal (D.3.1, Figure
    /// D.2).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`at` is inside the border, so every neighbour index is in the array"
    )]
    fn neighbours(&self, at: usize) -> (u8, u8, u8) {
        let s = |index: usize| u8::from(self.significant[index]);
        let up = at - self.stride;
        let down = at + self.stride;
        (
            s(at - 1) + s(at + 1),
            s(up) + s(down),
            s(up - 1) + s(up + 1) + s(down - 1) + s(down + 1),
        )
    }

    /// ISO/IEC 15444-1 Table D.1's significance context for the LL band.
    fn significance_context(&self, at: usize) -> usize {
        match self.neighbours(at) {
            (2, _, _) => 8,
            (1, 1.., _) => 7,
            (1, 0, 1..) => 6,
            (1, 0, 0) => 5,
            (0, 2, _) => 4,
            (0, 1, _) => 3,
            (0, 0, 2..) => 2,
            (0, 0, 1) => 1,
            _ => 0,
        }
    }

    /// ISO/IEC 15444-1 Table D.3's sign context and its XOR bit, from ISO/IEC 15444-1 Table D.2's
    /// horizontal and vertical contributions.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "each contribution is -1, 0 or 1, so a sum of two is between -2 and 2"
    )]
    fn sign_context(&self, at: usize) -> (usize, u8) {
        let sign = |index: usize| -> i8 {
            if !self.significant[index] {
                0
            } else if self.negative[index] {
                -1
            } else {
                1
            }
        };
        let horizontal = (sign(at - 1) + sign(at + 1)).clamp(-1, 1);
        let vertical = (sign(at - self.stride) + sign(at + self.stride)).clamp(-1, 1);
        match (horizontal, vertical) {
            (1, 1) => (13, 0),
            (1, 0) => (12, 0),
            (1, -1) => (11, 0),
            (0, 1) => (10, 0),
            (0, -1) => (10, 1),
            (-1, 1) => (11, 1),
            (-1, 0) => (12, 1),
            (-1, -1) => (13, 1),
            _ => (9, 0),
        }
    }

    /// Bit `plane` of the coefficient at `at`.
    fn bit(&self, at: usize, plane: u32) -> u8 {
        u8::from((self.magnitude[at] >> plane) & 1 == 1)
    }

    /// Codes the sign of a coefficient that has just become significant (D.3.2).
    fn code_sign(&self, coder: &mut Encoder, at: usize) {
        let (context, xor) = self.sign_context(at);
        coder.encode(context, u8::from(self.negative[at]) ^ xor);
    }

    /// The scan of D.1: stripes of four rows, each column top to bottom, left to right.
    fn scan(&self) -> impl Iterator<Item = (usize, usize, usize)> + use<> {
        let (width, height) = (self.width, self.height);
        (0..height).step_by(4).flat_map(move |top| {
            (0..width).flat_map(move |col| {
                (top..height.min(top.saturating_add(4))).map(move |row| (top, row, col))
            })
        })
    }

    /// D.3.1: every insignificant coefficient with a significant neighbour has its bit coded.
    fn significance_propagation(&mut self, coder: &mut Encoder, plane: u32) {
        for (_, row, col) in self.scan() {
            let at = self.at(row, col);
            if self.significant[at] || self.significance_context(at) == 0 {
                continue;
            }
            let bit = self.bit(at, plane);
            coder.encode(self.significance_context(at), bit);
            self.visited[at] = true;
            if bit == 1 {
                self.code_sign(coder, at);
                self.significant[at] = true;
            }
        }
    }

    /// D.3.3: every coefficient significant before this bit plane has its bit coded in
    /// ISO/IEC 15444-1 Table D.4's context.
    fn magnitude_refinement(&mut self, coder: &mut Encoder, plane: u32) {
        for (_, row, col) in self.scan() {
            let at = self.at(row, col);
            if !self.significant[at] || self.visited[at] {
                continue;
            }
            let context = if self.refined[at] {
                16
            } else if self.neighbours(at) == (0, 0, 0) {
                14
            } else {
                15
            };
            coder.encode(context, self.bit(at, plane));
            self.refined[at] = true;
        }
    }

    /// D.3.4: every coefficient neither significant nor coded in this plane's first pass, with
    /// run-length coding over a full stripe column whose four coefficients all have the
    /// zero context; the plane's visits are then forgotten.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "stripe rows and columns are inside a block of at most 64 × 64, and a run's \
                  position is below four"
    )]
    fn cleanup(&mut self, coder: &mut Encoder, plane: u32) {
        for top in (0..self.height).step_by(4) {
            let bottom = self.height.min(top + 4);
            for col in 0..self.width {
                let mut row = top;
                let full = bottom - top == 4;
                if full
                    && (top..bottom).all(|r| {
                        let at = self.at(r, col);
                        !self.significant[at]
                            && !self.visited[at]
                            && self.significance_context(at) == 0
                    })
                {
                    let first = (top..bottom).find(|r| self.bit(self.at(*r, col), plane) == 1);
                    let Some(first) = first else {
                        coder.encode(RUN, 0);
                        continue;
                    };
                    coder.encode(RUN, 1);
                    let position = first - top;
                    coder.encode(UNIFORM, u8::from(position & 2 != 0));
                    coder.encode(UNIFORM, u8::from(position & 1 != 0));
                    let at = self.at(first, col);
                    self.code_sign(coder, at);
                    self.significant[at] = true;
                    row = first + 1;
                }
                for r in row..bottom {
                    let at = self.at(r, col);
                    if self.significant[at] || self.visited[at] {
                        continue;
                    }
                    let bit = self.bit(at, plane);
                    coder.encode(self.significance_context(at), bit);
                    if bit == 1 {
                        self.code_sign(coder, at);
                        self.significant[at] = true;
                    }
                }
            }
        }
        self.visited.fill(false);
    }
}
