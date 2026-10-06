//! The glyph names an sfnt's `post` table states, read once per face.
//!
//! ISO 32000-2 §9.6.5.4 sends a glyph name no other route maps to the font program's `post`
//! table, and §9.10.2 reads the same names back as text. Both ask the table the same two
//! questions — which name a glyph has, and which glyph has a name — and the table answers only
//! the first directly.
//!
//! The OpenType specification's `post` table, in its section on version 2.0, lays the names out
//! as two arrays: `glyphNameIndex`, one 16-bit entry per glyph, and after it a run of
//! Pascal strings. An index below 258 names one of the standard Macintosh glyph names, which the
//! specification lists in its version 1.0 section and which `read-fonts` carries as a static table;
//! an index of 258 or more names the string at that position less 258 in the run. Version 1.0 is
//! that standard list with no index at all, glyph for glyph, and version 3.0 states no names.
//!
//! **Why this is a table rather than a call per glyph.** The run of strings has no offsets, so
//! the position of the n-th string is found by stepping over the n before it, and `read-fonts`'
//! `Post::glyph_name` does exactly that on every call. A caller asking for every glyph's name, or
//! searching every glyph for one name, therefore pays the square of the glyph count. A fuzzed
//! document with a 5125-glyph table cost 14.3 G instructions per page that way (ADR 1584). Here
//! the strings are stepped over once, the index is resolved once into one name per glyph, and the
//! inverse is one map built from that, each the first time something asks for it.

use std::cell::OnceCell;
use std::collections::HashMap;

use skrifa::FontRef;
use skrifa::raw::TableProvider;
use skrifa::raw::tables::post::{DEFAULT_GLYPH_NAMES, Post};
use skrifa::raw::types::Version16Dot16;

/// One face's `post` glyph names, both ways, each built on first use.
///
/// Constructing it reads the table's header and nothing else, so a font whose every name some
/// other route answers never walks its strings.
pub(crate) struct PostNames<'a> {
    /// The table, or `None` when the face has none or it cannot be read.
    post: Option<Post<'a>>,
    /// The name each glyph has, indexed by glyph; `None` where the table names it nothing.
    by_glyph: OnceCell<Box<[Option<&'a str>]>>,
    /// The glyph each name belongs to.
    ///
    /// A hash map under the standard library's per-process random keys. The names are the
    /// document's, so a fixed hash is one a file can fill with colliding names; and a tree cost
    /// about 890 instructions a name to build against about 280 for this, which on a subset of two
    /// thousand glyphs asked a few names is more than the search it replaces (ADR 1584).
    by_name: OnceCell<HashMap<&'a str, u16>>,
}

impl<'a> PostNames<'a> {
    /// Takes the face's `post` table, reading only its header.
    pub(crate) fn new(font: &FontRef<'a>) -> Self {
        Self {
            post: font.post().ok(),
            by_glyph: OnceCell::new(),
            by_name: OnceCell::new(),
        }
    }

    /// The name the table gives `glyph`, if it gives one.
    pub(crate) fn name(&self, glyph: u16) -> Option<&'a str> {
        self.by_glyph().get(usize::from(glyph)).copied().flatten()
    }

    /// The glyph the table gives `name` to.
    ///
    /// Where two glyphs carry the same name the lower one is answered, which is the order the
    /// table lists them in and what a search from glyph zero would find.
    pub(crate) fn glyph(&self, name: &str) -> Option<u16> {
        self.by_name
            .get_or_init(|| {
                let by_glyph = self.by_glyph();
                let mut by_name = HashMap::with_capacity(by_glyph.len());
                for (glyph, slot) in (0..=u16::MAX).zip(by_glyph) {
                    if let Some(name) = slot {
                        by_name.entry(*name).or_insert(glyph);
                    }
                }
                by_name
            })
            .get(name)
            .copied()
    }

    /// Every glyph's name, resolved from the index in one pass over the strings.
    fn by_glyph(&self) -> &[Option<&'a str>] {
        self.by_glyph.get_or_init(|| match &self.post {
            Some(post) => resolve(post),
            None => Box::default(),
        })
    }
}

/// The name of each glyph a `post` table covers, in glyph order.
///
/// Versions 1.0 and 2.0 are the two that state names by glyph; any other version answers none,
/// which is what `read-fonts` itself answers for them. A string that is not ASCII names nothing,
/// and so does an index past the end of the strings — both are a malformed table, and the glyph
/// is then one the table does not name rather than one it misnames.
fn resolve<'a>(post: &Post<'a>) -> Box<[Option<&'a str>]> {
    match post.version() {
        Version16Dot16::VERSION_1_0 => DEFAULT_GLYPH_NAMES.iter().copied().map(Some).collect(),
        Version16Dot16::VERSION_2_0 => {
            let Some(indices) = post.glyph_name_index() else {
                return Box::default();
            };
            let strings: Vec<Option<&'a str>> = post
                .string_data()
                .map(|strings| {
                    strings
                        .iter()
                        .map(|string| string.ok().map(|string| string.as_str()))
                        .collect()
                })
                .unwrap_or_default();
            indices
                .iter()
                .map(|index| {
                    let index = usize::from(index.get());
                    match index.checked_sub(DEFAULT_GLYPH_NAMES.len()) {
                        None => DEFAULT_GLYPH_NAMES.get(index).copied(),
                        Some(position) => strings.get(position).copied().flatten(),
                    }
                })
                .collect()
        }
        _ => Box::default(),
    }
}
