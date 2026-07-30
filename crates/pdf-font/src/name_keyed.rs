//! What a *name-keyed* font program offers a simple font's character codes.
//!
//! ISO 32000-2 §9.6.5.2 describes one algorithm for two file formats, and §9.6.2.1's NOTE 1
//! is why it can: a CFF is "an alternative, more compact but functionally equivalent
//! representation of a Type 1 font program". Both key their glyph descriptions by *name*,
//! both carry a built-in encoding from codes to those names, and the clause's rules —
//! `/Differences` over a base encoding, the base being the program's own when the program is
//! embedded — are stated once for both.
//!
//! So this is the shape `cff.rs` and `type1.rs` both produce and
//! [`crate::simple_code_table`] consumes. Neither reader appears in it: what a Type 1
//! program's eexec encryption or a CFF's INDEX structures look like is their own business,
//! and the clause's business is only that a code has a name and a name has a glyph.

use std::collections::BTreeMap;

/// A name-keyed program's own statements about its glyphs and its codes.
#[derive(Debug)]
pub struct NameKeyed {
    /// Glyph index by glyph name, taken from the program's charset.
    pub by_name: BTreeMap<Box<str>, u16>,
    /// Glyph index by character code, taken from the encoding the program itself carries.
    ///
    /// §9.6.5.1's Table 112 makes this the *base* encoding whenever the font program is
    /// embedded and the `/Encoding` dictionary names no `/BaseEncoding`, so it is what a
    /// `/Differences` array describes differences from rather than only a fallback for a
    /// code nothing else reached.
    pub builtin: Box<[Option<u16>; 256]>,
    /// The glyph name the built-in encoding gives each character code.
    ///
    /// The same mapping as [`Self::builtin`], carried through the charset instead of
    /// stopping at the glyph index. Nothing about *drawing* needs it — `builtin` selects the
    /// glyph directly — but a code's glyph name is what a document with no `/ToUnicode`
    /// means by that code, so text extraction and [`crate::LoadedFont::code_for`] would
    /// otherwise lose every code the PDF encoding left to the program.
    pub builtin_names: Box<[Option<Box<str>>; 256]>,
}

impl NameKeyed {
    /// Builds the mapping from a program's charset and its built-in encoding.
    ///
    /// `by_glyph` is the charset: one name per glyph, in glyph order. Inverting it here
    /// rather than in each reader is what keeps the tie-break in one place — a name two
    /// glyphs share resolves to the *lower* glyph, matching the order a charset assigns
    /// them, and a `BTreeMap`'s iteration order is what makes that true rather than a
    /// comment claiming it.
    #[must_use]
    pub fn new(by_glyph: &BTreeMap<u16, Box<str>>, builtin: Box<[Option<u16>; 256]>) -> Self {
        let mut by_name = BTreeMap::new();
        for (glyph, name) in by_glyph {
            by_name.entry(name.clone()).or_insert(*glyph);
        }
        let builtin_names = Box::new(std::array::from_fn(|code| {
            builtin
                .get(code)
                .copied()
                .flatten()
                .and_then(|glyph| by_glyph.get(&glyph).cloned())
        }));
        Self {
            by_name,
            builtin,
            builtin_names,
        }
    }
}
