//! One §7.10.5 program written twice: with its comments and without them.
//!
//! ISO 32000-2 §7.10.5.1 lists what a type 4 function's stream may contain, and "Comments" is
//! one of the five entries — so the same program, commented and bare, is one function with two
//! spellings and must paint one picture. Until the five-hundred-and-twenty-sixth session it did
//! not: the compiler split the whole stream on white space *before* looking for a PERCENT SIGN,
//! which destroys the line a comment ends at, and then skipped exactly one token after the sign.
//!
//! That leaves two failure shapes and the pair carries both on purpose:
//!
//! - **Loud.** Where the word after the sign is prose, the word after *that* is not an operator
//!   and the whole function is refused — which is how the defect was found, on the project
//!   owner's `doc/corpora-own/type4_pi.pdf` and its `% BBP Math for Pi …` first line.
//! - **Silent.** Where a comment quotes the code it documents — `% dup 3 mul`, which is how
//!   people comment arithmetic — every word after the first is a valid token, so the old rule
//!   compiled them into the program and the function computed something the file never said.
//!   Nothing reported, and a shading is exactly the place where a wrong number is still a
//!   plausible picture.
//!
//! Hand-built per trap 8, and **both arms are written out literally**: deriving the bare arm by
//! running this tree's own comment stripper over the commented one would compare the compiler
//! against itself and prove nothing.

use crate::assemble_pdf;

/// The fixture page's size in points, which at one pixel per point is the raster's too.
pub const TYPE4_PAGE: (u32, u32) = (100, 100);

/// Where the program's decision falls, in points from the page's left edge.
///
/// The function's whole content: the shading's domain is the unit square, so the program's
/// `0.5 ge` is this many points across a 100-point page. Left of it is black and right of it is
/// white — a picture a test can state exactly and an injected instruction cannot leave standing.
pub const TYPE4_SPLIT: u32 = 50;

/// Which arm of the pair to build — the fixture's one difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type4Comments {
    /// The program as somebody would write it by hand, with a comment above each step.
    Present,
    /// The same program with nothing but its tokens.
    Absent,
}

/// The commented arm, in the two shapes a comment fails the old rule in.
///
/// Lines 1 and 3 are prose, whose second word is not an operator; line 2 quotes a formula, whose
/// every word is one.
const COMMENTED: &str = "{\n\
     % pop the y, since only x decides\n\
     pop\n\
     % dup 3 mul\n\
     0.5 ge\n\
     % 1 is white and 0 is black\n\
     { 1 } { 0 } ifelse\n\
     }\n";

/// The bare arm: the same six tokens, typed out again rather than derived.
const BARE: &str = "{\n\
     pop\n\
     0.5 ge\n\
     { 1 } { 0 } ifelse\n\
     }\n";

/// Builds a one-page PDF whose page is painted by a §8.7.4.5.2 function-based shading.
///
/// The shading's `/Function` is the type 4 program above, mapping the unit square to one
/// `DeviceGray` component; `/Matrix` scales that square onto the page, so a pixel's colour is
/// the program's value at the point under it.
#[must_use]
pub fn type4_comment_pair(comments: Type4Comments) -> Vec<u8> {
    let program = match comments {
        Type4Comments::Present => COMMENTED,
        Type4Comments::Absent => BARE,
    };
    let (width, height) = TYPE4_PAGE;
    let content = b"/Sh0 sh".to_vec();

    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] \
             /Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>"
        )
        .into_bytes(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"\nendstream".to_vec(),
        ]
        .concat(),
        format!(
            "<< /ShadingType 1 /ColorSpace /DeviceGray /Domain [0 1 0 1] \
             /Matrix [{width} 0 0 {height} 0 0] /Function 6 0 R >>"
        )
        .into_bytes(),
        [
            format!(
                "<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1] /Length {} >>\nstream\n",
                program.len()
            )
            .into_bytes(),
            program.as_bytes().to_vec(),
            b"endstream".to_vec(),
        ]
        .concat(),
    ];

    assemble_pdf(&objects)
}
