//! The pair kerning an embedded font program states, read out of the program itself.
//!
//! XFA 3.3 chapter 27's `kerning-mode:pair` (*Kerning*, pages 1203 and 1204) asks for kerning by
//! the two adjacent glyphs and nothing else (the version 2.8 change list), and the numbers are
//! the face's: a rich text run set in a font the document embeds is kerned by the pairs that
//! program states, and by nothing else. The program states them in one of two places, and the
//! font format's own specification (ISO/IEC 14496-22, OpenType) says what each means:
//!
//! - **`GPOS`'s `kern` feature**, lookup type 2 (pair adjustment), in both of its subtable formats
//!   — format 1 by glyph, format 2 by class — and type 9's extension of it. Pairs are in logical
//!   order: for text read right to left the right-most glyph is the first of a pair.
//! - **The legacy `kern` table**, version 0, its formats 0 and 2, whose pairs are a left-hand and
//!   a right-hand glyph and whose values add up across subtables, an override subtable replacing
//!   what was accumulated.
//!
//! # Choices, each the font format's to leave or this reader's to make (ADR 1682)
//!
//! - **`GPOS` first.** A program stating a `kern` feature is kerned by it and its `kern` table is
//!   not read; one stating none is kerned by its `kern` table. OpenType makes `GPOS` the only
//!   table a CFF-flavoured program may kern with and states no order between the two otherwise.
//! - **The feature is the one the run's script selects** (ADR 1696), as OpenType's layout selects
//!   a feature set: the script table registered for the script the run's characters resolve to
//!   ([`scripts`]), the `DFLT` table where the program registers none for it or the run has no
//!   specific script, and in either the language system the text's natural language selects
//!   ([`Language`], ADR 1708), the default one where the table registers none for it or the text
//!   names no language. Its `kern` lookups apply in lookup-list order. Two glyphs of different
//!   scripts are not a pair: text is laid out one script at a time.
//! - **Only pair adjustment is read** (ADR 1696). A contextual lookup under the same feature —
//!   `ContextPos` or `ChainContextPos`, even one reaching a pair adjustment — positions a pair by
//!   the glyphs around it, and the version 2.8 change list defines pair kerning as kerning by the
//!   two adjacent glyphs alone; so it is not this property's to apply, and nothing the property
//!   asks for is left undone by leaving it.
//! - **Device tables and variation deltas are not applied**: they are adjustments at a pixel size
//!   or a design-space location, and an appearance stream is laid out in neither.
//! - **Vertical values are not applied and are said** ([`Adjusted::vertical`]): a pair stating a
//!   `YPlacement` or `YAdvance` would raise a glyph within a line, which a horizontal run of
//!   §9.4.3's `TJ` adjustments cannot do. A `kern` subtable holding minimum or cross-stream values
//!   is likewise not applied and is said ([`Pairs::unread`]).
//!
//! # What it costs
//!
//! Nothing at load: a program's pairs are read only for a run that asks for pair kerning, by the
//! caller that asks. The work is bounded by [`MAX_SUBTABLES`] — a program whose `kern` feature
//! reaches more pair subtables than that is refused rather than walked, because each glyph pair
//! visits every one of them in the worst case.

use std::sync::Arc;

use skrifa::raw::tables::gpos::{PairPos, PositionSubtables};
use skrifa::raw::tables::kern::{Kern, Subtable, SubtableKind};
use skrifa::raw::types::GlyphId;
use skrifa::raw::{FontRef, TableProvider as _};
use skrifa::{GlyphId16, Tag};

/// The most pair subtables one program may bring to a run: far above any face's kerning, which
/// states tens, and low enough that a run of [`crate::Code`]s at the field layout's own limit
/// stays a bounded walk.
pub const MAX_SUBTABLES: usize = 512;

/// Why a font gives a run no pairs to kern by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoPairs {
    /// The glyphs are a stand-in's — this machine's face or one this crate carries — so its
    /// pairs would be another typeface's numbers rather than the document's.
    NotTheDocuments,
    /// The program holds no `GPOS` `kern` feature and no `kern` table this reader applies: a
    /// bare CFF or Type 1 program, which carries neither, among them.
    NoPairData,
    /// The tables are there and could not be read.
    Unreadable,
    /// The `kern` feature reaches more than [`MAX_SUBTABLES`] pair subtables.
    OverBudget,
}

impl NoPairs {
    /// The reason, as a phrase for a report.
    #[must_use]
    pub fn phrase(self) -> &'static str {
        match self {
            Self::NotTheDocuments => {
                "a face this program chose, whose pairs are not the document's"
            }
            Self::NoPairData => "a face whose program states no pair kerning",
            Self::Unreadable => "a face whose kerning tables could not be read",
            Self::OverBudget => "a face whose kerning reaches more subtables than are read",
        }
    }
}

/// Where a program's pairs come from.
#[derive(Debug, Clone)]
enum Source {
    /// `GPOS`'s `kern` feature, as each script table's language systems reach it.
    Positioning(Scripted),
    /// The legacy `kern` table.
    Kerning,
}

/// `GPOS`'s script tables, each with the pair-adjustment lookups its language systems' `kern`
/// features reach, ascending — empty where one reaches none.
#[derive(Debug, Clone)]
struct Scripted {
    tables: Vec<ScriptTable>,
}

/// One script table's language systems, as far as pair kerning reads them.
#[derive(Debug, Clone)]
struct ScriptTable {
    tag: Tag,
    /// The default language system's lookups; empty where the table has none.
    default: Vec<u16>,
    /// Each language-specific system's tag and lookups, in the table's own order.
    languages: Vec<(Tag, Vec<u16>)>,
}

impl Scripted {
    /// The lookups OpenType's layout selects for a run of `script` in `language`.
    ///
    /// The script table is the one registered for the script, and `DFLT`'s where none is or the
    /// run has no specific script — "[a]n application should use a DFLT script table if there is
    /// not a script table associated with the specific script of the text being formatted, or if
    /// the text does not have a specific script" (ISO/IEC 14496-22, *OpenType Layout common table
    /// formats*, `ScriptList` table). Within it, the language system registered for the first of
    /// the language's tags the table registers, and the default one where it registers none of
    /// them: a run is laid out under one or the other and never both, so a language system's
    /// lookups replace the default's rather than adding to them (the `Script` table and its
    /// Example 3, ADR 1708). A program registering neither script table gives the run no lookups.
    fn lookups(&self, script: Script, language: Language) -> &[u16] {
        let find = |tag: Tag| self.tables.iter().find(|table| table.tag == tag);
        let Some(table) = script
            .0
            .into_iter()
            .flat_map(tags)
            .flatten()
            .find_map(find)
            .or_else(|| find(Tag::new(b"DFLT")))
        else {
            return &[];
        };
        language
            .tags()
            .find_map(|wanted| {
                table
                    .languages
                    .iter()
                    .find(|(registered, _)| registered.to_be_bytes() == wanted)
            })
            .map_or(table.default.as_slice(), |(_, lookups)| lookups.as_slice())
    }
}

/// The OpenType language systems a run's natural language selects, in the order they are tried.
///
/// The language is §14.9.2's — a BCP 47 language tag — and the tags are the ones OpenType's
/// registry (*Language system tags*, OpenType 1.9.1) gives the tag's language by its ISO 639
/// code: `TRK ` for `tr`, `URD ` for `ur`. Only the language subtag decides, with the
/// extended-language subtag under it tried first where there is one: RFC 5646 section 2.2.2
/// writes a language of a macrolanguage as the macrolanguage's subtag and its own (`ar-arz`), and
/// the registry often lists the macrolanguage alone (`ARA ` for `ara`). Script, region and
/// variant subtags select nothing here, because the registry's codes are languages' alone, so a
/// code it gives several tags (Chinese's five) tries them in the registry's order (ADR 1708). An
/// unknown language — none stated, an empty tag, a private-use or grandfathered tag, a code the
/// registry gives no tag — selects none, and the default language system is used, which is the
/// one OpenType has a script laid out under where there is no language-specific information.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Language {
    /// The extended-language subtag's tags.
    extended: &'static [[u8; 4]],
    /// The language subtag's tags.
    language: &'static [[u8; 4]],
}

impl Language {
    /// The language systems a BCP 47 language tag selects.
    #[must_use]
    pub fn of(tag: &str) -> Self {
        let mut subtags = tag.split('-');
        let primary = subtags.next().unwrap_or_default().to_ascii_lowercase();
        if !(2..=3).contains(&primary.len()) {
            return Self::default();
        }
        let extended = subtags
            .next()
            .filter(|next| next.len() == 3 && next.bytes().all(|byte| byte.is_ascii_alphabetic()))
            .map_or(&[][..], |extended| {
                registered(&extended.to_ascii_lowercase())
            });
        Self {
            extended,
            language: registered(&primary),
        }
    }

    /// The language system tags, in the order they are tried; none for an unknown language.
    pub fn tags(self) -> impl Iterator<Item = [u8; 4]> {
        self.extended.iter().chain(self.language).copied()
    }
}

/// The registry's tags for one ISO 639 code, two letters or three; none for a code it lists
/// under no tag.
fn registered(code: &str) -> &'static [[u8; 4]] {
    let three: Option<[u8; 3]> = match code.as_bytes() {
        &[first, second] => languages::TWO_LETTER
            .binary_search_by_key(&[first, second], |(two, _)| *two)
            .ok()
            .and_then(|at| languages::TWO_LETTER.get(at))
            .map(|(_, three)| *three),
        three => <[u8; 3]>::try_from(three).ok(),
    };
    three
        .filter(|code| code.iter().all(u8::is_ascii_lowercase))
        .and_then(|three| {
            languages::LANGUAGES
                .binary_search_by_key(&three, |(code, _, _)| *code)
                .ok()
        })
        .and_then(|at| languages::LANGUAGES.get(at))
        .and_then(|(_, first, count)| {
            let first = usize::from(*first);
            languages::LANGUAGE_TAGS.get(first..first.checked_add(usize::from(*count))?)
        })
        .unwrap_or_default()
}

/// The tables `build.rs` writes from `data/opentype/language-tags.txt` and
/// `data/iso639/ISO-639-2_utf-8.txt`.
mod languages {
    include!(concat!(env!("OUT_DIR"), "/languages.rs"));
}

/// The script a character is set in, as far as choosing a program's script table goes: the ISO
/// 15924 code of its Unicode `Script` property, a `Common` or `Inherited` character taking its
/// neighbours' ([`scripts`]); no code where the text around it has no specific script.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Script(Option<[u8; 4]>);

impl Script {
    /// The ISO 15924 code, `Latn` or `Arab`; `None` for text of no specific script.
    #[must_use]
    pub fn code(self) -> Option<[u8; 4]> {
        self.0
    }

    /// The tag a run of this script is laid out under, so that two scripts sharing one script
    /// table — `Hira` and `Kana` under `kana` — are one run.
    fn run(self) -> Option<Tag> {
        self.0.and_then(|code| tags(code)[0])
    }
}

/// The scripts of a run of characters, one per character.
///
/// Each character takes its Unicode `Script` property (`Scripts.txt`, compiled in by
/// `build.rs`). One whose property is `Common` or `Inherited` — a space, a digit, punctuation, a
/// combining mark — takes the script of the nearest specific character before it, or after it
/// where none precedes, which is the processing OpenType's `ScriptList` note describes:
/// "\[a\]pplications may process script-neutral characters together with immediately-preceding or
/// following script-specific characters". A run of neutral characters alone has no script, and
/// is laid out under `DFLT`.
#[must_use]
pub fn scripts(characters: &[char]) -> Vec<Script> {
    let own: Vec<Option<[u8; 4]>> = characters.iter().map(|c| script_of(*c)).collect();
    // The first specific script of the run is what its leading neutral characters take.
    let first = own.iter().find_map(|script| *script);
    let mut before = None;
    own.iter()
        .map(|script| {
            before = script.or(before);
            Script(before.or(first))
        })
        .collect()
}

/// A character's own `Script` property, as an ISO 15924 code; `None` for `Common`, `Inherited`
/// and an unassigned character.
fn script_of(character: char) -> Option<[u8; 4]> {
    let at = tables::SCRIPTS.partition_point(|(_, last, _)| *last < character);
    tables::SCRIPTS
        .get(at)
        .filter(|(first, _, _)| *first <= character)
        .map(|(_, _, code)| *code)
}

/// The script tags OpenType registers for an ISO 15924 code, in the order they are tried.
///
/// The registry (*Script tags*, OpenType 1.9.1) gives each script its tag, and for almost every
/// script `Scripts.txt` names its tag is the code in lower case. The rest are the registry's own
/// rows: Hiragana shares Katakana's `kana`; Lao, N'Ko, Vai and Yi are padded with spaces rather
/// than spelled as their ISO codes; and ten scripts carry a second tag, "v.2", registered for a
/// later layout implementation, which is tried before the first because a program carrying both
/// was built for both and the later is the one its maker revised. `data/opentype/script-tags.txt`
/// is the registry's list, and a test reads every code against it.
fn tags(code: [u8; 4]) -> [Option<Tag>; 2] {
    let one = |tag: &[u8; 4]| [Some(Tag::new(tag)), None];
    let two = |newer: &[u8; 4], older: &[u8; 4]| [Some(Tag::new(newer)), Some(Tag::new(older))];
    match &code {
        b"Hira" => one(b"kana"),
        b"Laoo" => one(b"lao "),
        b"Nkoo" => one(b"nko "),
        b"Vaii" => one(b"vai "),
        b"Yiii" => one(b"yi  "),
        b"Beng" => two(b"bng2", b"beng"),
        b"Deva" => two(b"dev2", b"deva"),
        b"Gujr" => two(b"gjr2", b"gujr"),
        b"Guru" => two(b"gur2", b"guru"),
        b"Knda" => two(b"knd2", b"knda"),
        b"Mlym" => two(b"mlm2", b"mlym"),
        b"Mymr" => two(b"mym2", b"mymr"),
        b"Orya" => two(b"ory2", b"orya"),
        b"Taml" => two(b"tml2", b"taml"),
        b"Telu" => two(b"tel2", b"telu"),
        _ => one(&code.map(|byte| byte.to_ascii_lowercase())),
    }
}

/// The table `build.rs` writes from `data/unicode/Scripts.txt`.
mod tables {
    include!(concat!(env!("OUT_DIR"), "/scripts.rs"));
}

/// One program's pair kerning, ready to be asked about runs of glyphs.
#[derive(Debug, Clone)]
pub struct Pairs {
    data: Arc<[u8]>,
    source: Source,
    units_per_em: f32,
    /// What the program states and this reader does not apply, said where the pairs are used.
    unread: Option<&'static str>,
}

/// One glyph's adjustment, in ems: its advance, which moves every glyph after it, and its
/// placement, which moves it alone — OpenType's `XAdvance` and `XPlacement`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Adjustment {
    /// Added to the glyph's advance: the room between it and the glyph after it in logical order.
    pub advance: f32,
    /// How far right the glyph is drawn of where it stands, the glyphs after it unmoved.
    pub placement: f32,
}

/// A run's adjustments, one per glyph asked about.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Adjusted {
    /// Each glyph's adjustment, in the order asked.
    pub glyphs: Vec<Adjustment>,
    /// Whether a pair the run met stated a vertical value, which is not applied.
    pub vertical: bool,
}

impl Pairs {
    /// Reads an sfnt program's pairs.
    ///
    /// # Errors
    ///
    /// [`NoPairs`] where the program states none this reader applies, cannot be read, or reaches
    /// more subtables than [`MAX_SUBTABLES`].
    pub(crate) fn read(data: Arc<[u8]>, units_per_em: f32) -> Result<Self, NoPairs> {
        if units_per_em <= 0.0 {
            return Err(NoPairs::Unreadable);
        }
        let (source, unread) = {
            let font = FontRef::new(&data).map_err(|_| NoPairs::Unreadable)?;
            match kern_feature_lookups(&font)? {
                Some(scripted) => (Source::Positioning(scripted), None),
                None => (Source::Kerning, kern_table_shape(&font)?),
            }
        };
        Ok(Self {
            data,
            source,
            units_per_em,
            unread,
        })
    }

    /// What this program states about kerning and this reader does not apply, as a phrase.
    #[must_use]
    pub fn unread(&self) -> Option<&'static str> {
        self.unread
    }

    /// The adjustments of a run of glyphs in logical order; `scripts` gives, per glyph, the
    /// script its character resolves to ([`scripts`]), `language` the run's natural language
    /// ([`Language`]), and `right_to_left` whether each glyph reads right to left. A `None`
    /// glyph — a code reaching none of the program's — pairs with nothing, and two glyphs of
    /// different directions or of different scripts are not a pair.
    #[must_use]
    pub fn adjust(
        &self,
        glyphs: &[Option<u16>],
        scripts: &[Script],
        language: Language,
        right_to_left: &[bool],
    ) -> Adjusted {
        let mut out = Adjusted {
            glyphs: vec![Adjustment::default(); glyphs.len()],
            vertical: false,
        };
        let Ok(font) = FontRef::new(&self.data) else {
            return out;
        };
        let script = |at: usize| scripts.get(at).copied().unwrap_or_default();
        let pair = |at: usize| {
            let next = at.checked_add(1)?;
            let first = glyphs.get(at).copied().flatten()?;
            let second = glyphs.get(next).copied().flatten()?;
            let direction = right_to_left.get(at).copied().unwrap_or(false);
            let other = right_to_left.get(next).copied().unwrap_or(false);
            (direction == other && script(at).run() == script(next).run())
                .then_some((first, second, direction))
        };
        match &self.source {
            Source::Positioning(scripted) => {
                let chosen: Vec<&[u16]> = (0..glyphs.len())
                    .map(|at| scripted.lookups(script(at), language))
                    .collect();
                self.positioned(&font, &chosen, &pair, &mut out);
            }
            Source::Kerning => self.kerned(&font, glyphs.len(), &pair, &mut out),
        }
        out
    }

    /// OpenType's pair adjustment, one lookup at a time over the whole run; `chosen` is, per
    /// glyph, the lookups its script selects, so a lookup passes over a pair its script's table
    /// does not reach.
    ///
    /// Within a lookup the first subtable that matches a pair applies, and where its second
    /// value format is not empty the pair's second glyph is not the first of the next pair:
    /// "[i]f valueFormat2 is set to 0, then the second glyph of the pair is the 'next' glyph for
    /// which a lookup should be performed" (ISO/IEC 14496-22, `GPOS`, `PairPosFormat1`).
    fn positioned(
        &self,
        font: &FontRef,
        chosen: &[&[u16]],
        pair: &dyn Fn(usize) -> Option<(u16, u16, bool)>,
        out: &mut Adjusted,
    ) {
        let Ok(gpos) = font.gpos() else {
            return;
        };
        let Ok(list) = gpos.lookup_list() else {
            return;
        };
        let count = chosen.len();
        let mut every: Vec<u16> = chosen
            .iter()
            .flat_map(|lookups| lookups.iter().copied())
            .collect();
        every.sort_unstable();
        every.dedup();
        for index in every {
            let Ok(lookup) = list.lookups().get(usize::from(index)) else {
                continue;
            };
            let Ok(PositionSubtables::Pair(subtables)) = lookup.subtables() else {
                continue;
            };
            let subtables: Vec<PairPos> = subtables.iter().flatten().collect();
            let selects = |at: usize| {
                chosen
                    .get(at)
                    .is_some_and(|lookups| lookups.binary_search(&index).is_ok())
            };
            let mut at = 0_usize;
            while at.saturating_add(1) < count {
                let Some((first, second, _)) = pair(at).filter(|_| selects(at)) else {
                    at = at.saturating_add(1);
                    continue;
                };
                let Some(matched) = pair_values(&subtables, first, second) else {
                    at = at.saturating_add(1);
                    continue;
                };
                for (offset, value) in [(0_usize, matched.first), (1, matched.second)] {
                    if let Some(glyph) = out.glyphs.get_mut(at.saturating_add(offset)) {
                        glyph.advance += self.in_ems(value.x_advance);
                        glyph.placement += self.in_ems(value.x_placement);
                    }
                    out.vertical |= value.vertical;
                }
                at = at.saturating_add(if matched.consumes_second { 2 } else { 1 });
            }
        }
    }

    /// The `kern` table's pairs: a left-hand and a right-hand glyph, the value the room between
    /// them, summed across subtables and replaced by an override subtable's.
    ///
    /// The room between a pair is the first glyph's advance in logical order, whichever way the
    /// pair reads; read right to left, the left-hand glyph is the second.
    fn kerned(
        &self,
        font: &FontRef,
        count: usize,
        pair: &dyn Fn(usize) -> Option<(u16, u16, bool)>,
        out: &mut Adjusted,
    ) {
        let Ok(table @ Kern::Ot(_)) = font.kern() else {
            return;
        };
        let subtables: Vec<(SubtableKind, bool)> = table
            .subtables()
            .flatten()
            .filter_map(|subtable| {
                let Subtable::Ot(header) = &subtable else {
                    return None;
                };
                let coverage = header.coverage();
                // Bit 0 horizontal, bit 1 minimum, bit 2 cross-stream, bit 3 override.
                let applied = coverage & 0b0111 == 0b0001;
                let kind = subtable.kind().ok()?;
                let readable = matches!(kind, SubtableKind::Format0(_) | SubtableKind::Format2(_));
                (applied && readable).then_some((kind, coverage & 0b1000 != 0))
            })
            .collect();
        for at in 0..count {
            let Some((first, second, right_to_left)) = pair(at) else {
                continue;
            };
            let (left, right) = if right_to_left {
                (second, first)
            } else {
                (first, second)
            };
            let (left, right) = (GlyphId::new(left.into()), GlyphId::new(right.into()));
            let mut value = 0_i32;
            for (kind, overrides) in &subtables {
                let stated = match kind {
                    SubtableKind::Format0(table) => table.kerning(left, right),
                    SubtableKind::Format2(table) => table.kerning(left, right),
                    SubtableKind::Format1(_) | SubtableKind::Format3(_) => None,
                };
                if let Some(stated) = stated {
                    value = if *overrides {
                        stated
                    } else {
                        value.saturating_add(stated)
                    };
                }
            }
            if let Some(glyph) = out.glyphs.get_mut(at) {
                // A `kern` value is an FWORD, so the sum of two is far inside `f32`'s exact range.
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a sum of 16-bit values is exact in f32 up to 2^24"
                )]
                let units = value as f32;
                glyph.advance += units / self.units_per_em;
            }
        }
    }

    fn in_ems(&self, units: i16) -> f32 {
        f32::from(units) / self.units_per_em
    }
}

/// One pair's two value records, as far as this reader applies them.
#[derive(Debug, Clone, Copy, Default)]
struct Value {
    x_advance: i16,
    x_placement: i16,
    vertical: bool,
}

impl Value {
    fn of(record: &skrifa::raw::tables::gpos::ValueRecord) -> Self {
        Self {
            x_advance: record.x_advance().unwrap_or(0),
            x_placement: record.x_placement().unwrap_or(0),
            vertical: record.y_advance().is_some_and(|value| value != 0)
                || record.y_placement().is_some_and(|value| value != 0),
        }
    }
}

/// What one subtable said about one pair.
struct Matched {
    first: Value,
    second: Value,
    consumes_second: bool,
}

/// The first of a lookup's subtables that states the pair.
///
/// A format 1 subtable states the pairs it lists; a format 2 one states every pair whose first
/// glyph its coverage holds, class 0 included, so it matches wherever that glyph is covered.
fn pair_values(subtables: &[PairPos], first: u16, second: u16) -> Option<Matched> {
    let (first, second) = (GlyphId16::new(first), GlyphId16::new(second));
    for subtable in subtables {
        match subtable {
            PairPos::Format1(table) => {
                let Some(covered) = table.coverage().ok().and_then(|c| c.get(first)) else {
                    continue;
                };
                let Ok(set) = table.pair_sets().get(usize::from(covered)) else {
                    continue;
                };
                let records = set.pair_value_records();
                // The records are sorted by the second glyph, so the search is binary.
                let (mut low, mut high) = (0_usize, records.len());
                let mut found = None;
                while low < high {
                    let middle = usize::midpoint(low, high);
                    let Ok(record) = records.get(middle) else {
                        break;
                    };
                    match record.second_glyph().cmp(&second) {
                        std::cmp::Ordering::Less => low = middle.saturating_add(1),
                        std::cmp::Ordering::Greater => high = middle,
                        std::cmp::Ordering::Equal => {
                            found = Some(record);
                            break;
                        }
                    }
                }
                let Some(record) = found else {
                    continue;
                };
                return Some(Matched {
                    first: Value::of(record.value_record1()),
                    second: Value::of(record.value_record2()),
                    consumes_second: table.value_format2().bits() != 0,
                });
            }
            PairPos::Format2(table) => {
                if table.coverage().ok().and_then(|c| c.get(first)).is_none() {
                    continue;
                }
                let (Ok(one), Ok(two)) = (table.class_def1(), table.class_def2()) else {
                    continue;
                };
                let (class1, class2) = (one.get(first), two.get(second));
                if class1 >= table.class1_count() || class2 >= table.class2_count() {
                    continue;
                }
                let Ok(row) = table.class1_records().get(usize::from(class1)) else {
                    continue;
                };
                let Ok(record) = row.class2_records().get(usize::from(class2)) else {
                    continue;
                };
                return Some(Matched {
                    first: Value::of(record.value_record1()),
                    second: Value::of(record.value_record2()),
                    consumes_second: table.value_format2().bits() != 0,
                });
            }
        }
    }
    None
}

/// The lookup indices one language system's `kern` features reach.
type Reached = std::collections::BTreeSet<u16>;

/// One script table's language systems and every lookup each reaches, before the lookups that
/// are not pair adjustment are set aside.
struct Reaching {
    tag: Tag,
    default: Reached,
    languages: Vec<(Tag, Reached)>,
}

/// The lookups a language system's `kern` features reach, the required feature among them.
fn kern_lookups_of(
    system: &skrifa::raw::tables::layout::LangSys,
    features: &skrifa::raw::tables::layout::FeatureList,
) -> Reached {
    let kern = Tag::new(b"kern");
    let required = system.required_feature_index();
    let named = system
        .feature_indices()
        .iter()
        .map(skrifa::raw::types::BigEndian::get)
        .chain((required != 0xFFFF).then_some(required));
    let mut indices = Reached::new();
    for feature_index in named {
        let Some(record) = features.feature_records().get(usize::from(feature_index)) else {
            continue;
        };
        if record.feature_tag() != kern {
            continue;
        }
        let Ok(feature) = record.feature(features.offset_data()) else {
            continue;
        };
        indices.extend(
            feature
                .lookup_list_indices()
                .iter()
                .map(skrifa::raw::types::BigEndian::get),
        );
    }
    indices
}

/// `GPOS`'s script tables with the pair-adjustment lookups each of their language systems
/// reaches through its `kern` feature; `None` where no language system of any script table
/// reaches one.
///
/// A language system names its features by index, the required feature among them, and OpenType
/// applies those and no others; a `kern` feature record no selected language system names is
/// not applied. The budget is over every pair subtable any language system reaches.
fn kern_feature_lookups(font: &FontRef) -> Result<Option<Scripted>, NoPairs> {
    let Ok(gpos) = font.gpos() else {
        return Ok(None);
    };
    let (Ok(scripts), Ok(features), Ok(list)) =
        (gpos.script_list(), gpos.feature_list(), gpos.lookup_list())
    else {
        return Err(NoPairs::Unreadable);
    };
    let mut reached: Vec<Reaching> = Vec::new();
    for record in scripts.script_records() {
        let table = record.script(scripts.offset_data()).ok();
        let default = table
            .as_ref()
            .and_then(skrifa::raw::tables::layout::Script::default_lang_sys)
            .and_then(Result::ok)
            .map(|system| kern_lookups_of(&system, &features))
            .unwrap_or_default();
        let languages = table
            .as_ref()
            .map(|table| {
                table
                    .lang_sys_records()
                    .iter()
                    .filter_map(|language| {
                        let system = language.lang_sys(table.offset_data()).ok()?;
                        Some((language.lang_sys_tag(), kern_lookups_of(&system, &features)))
                    })
                    .collect()
            })
            .unwrap_or_default();
        reached.push(Reaching {
            tag: record.script_tag(),
            default,
            languages,
        });
    }
    let every: Reached = reached
        .iter()
        .flat_map(|table| {
            table
                .default
                .iter()
                .chain(table.languages.iter().flat_map(|(_, indices)| indices))
                .copied()
        })
        .collect();
    let mut subtables = 0_usize;
    let mut pairs = std::collections::BTreeSet::new();
    for index in every {
        let Ok(lookup) = list.lookups().get(usize::from(index)) else {
            continue;
        };
        let Ok(PositionSubtables::Pair(found)) = lookup.subtables() else {
            continue;
        };
        subtables = subtables.saturating_add(found.len());
        if subtables > MAX_SUBTABLES {
            return Err(NoPairs::OverBudget);
        }
        pairs.insert(index);
    }
    if pairs.is_empty() {
        return Ok(None);
    }
    let kept = |indices: Reached| -> Vec<u16> {
        indices
            .into_iter()
            .filter(|index| pairs.contains(index))
            .collect()
    };
    let tables = reached
        .into_iter()
        .map(|table| ScriptTable {
            tag: table.tag,
            default: kept(table.default),
            languages: table
                .languages
                .into_iter()
                .map(|(language, indices)| (language, kept(indices)))
                .collect(),
        })
        .collect();
    Ok(Some(Scripted { tables }))
}

/// Whether the `kern` table holds a subtable this reader applies, and what it holds that it
/// does not.
fn kern_table_shape(font: &FontRef) -> Result<Option<&'static str>, NoPairs> {
    let Ok(table) = font.kern() else {
        return Err(if font.table_data(Tag::new(b"kern")).is_some() {
            NoPairs::Unreadable
        } else {
            NoPairs::NoPairData
        });
    };
    if matches!(table, Kern::Aat(_)) {
        // Apple's version 1 table, whose extensions OpenType says it does not support.
        return Err(NoPairs::NoPairData);
    }
    let mut applied = 0_usize;
    let mut unread = None;
    for subtable in table.subtables() {
        let Ok(Subtable::Ot(header)) = subtable else {
            return Err(NoPairs::Unreadable);
        };
        let coverage = header.coverage();
        let format = coverage >> 8;
        if coverage & 0b0001 == 0 {
            // Vertical kerning, which a horizontal run does not ask about.
            continue;
        }
        if coverage & 0b0110 != 0 {
            unread = Some("kern subtables of minimum or cross-stream values");
            continue;
        }
        if format == 0 || format == 2 {
            applied = applied.saturating_add(1);
            if applied > MAX_SUBTABLES {
                return Err(NoPairs::OverBudget);
            }
        }
    }
    if applied == 0 {
        return Err(NoPairs::NoPairData);
    }
    Ok(unread)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::type_complexity,
    reason = "a fixture's table offsets are small sums of known lengths, and a kern subtable is \
              written as the tuple its format states"
)]
mod tests {
    //! Each fixture is Liberation Sans with its own kerning taken out and one known pair written
    //! in, so every expected value below is the table this test wrote, divided by the face's
    //! 2048 units per em (trap 8).

    use super::{Adjustment, Language, NoPairs, Pairs, Script, scripts};
    use skrifa::MetadataProvider as _;
    use std::sync::Arc;

    const LIBERATION: &[u8] =
        include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");
    const EM: f32 = 2048.0;

    fn words(values: &[u16]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_be_bytes())
            .collect()
    }

    fn glyph(character: char) -> u16 {
        let font = skrifa::FontRef::new(LIBERATION).expect("Liberation Sans is an sfnt");
        let glyph = font
            .charmap()
            .map(character)
            .expect("Liberation Sans draws it");
        u16::try_from(glyph.to_u32()).expect("a 16-bit glyph index")
    }

    /// A version 0 `kern` table of the given subtables, each `(coverage, pairs)` in format 0.
    fn kern_table(subtables: &[(u16, Vec<(u16, u16, i16)>)]) -> Vec<u8> {
        let count = u16::try_from(subtables.len()).expect("few");
        let mut out = words(&[0, count]);
        for (coverage, pairs) in subtables {
            let mut pairs = pairs.clone();
            pairs.sort_unstable_by_key(|(left, right, _)| {
                (u32::from(*left) << 16) | u32::from(*right)
            });
            let n = u16::try_from(pairs.len()).expect("few");
            let length = 6 + 8 + 6 * n;
            out.extend(words(&[0, length, *coverage, n, 6, 0, 0]));
            for (left, right, value) in pairs {
                out.extend(words(&[left, right, value.cast_unsigned()]));
            }
        }
        out
    }

    /// A `GPOS` table whose `kern` feature reaches one lookup per subtable given, in order.
    fn gpos_table(subtables: &[Vec<u8>]) -> Vec<u8> {
        let count = u16::try_from(subtables.len()).expect("few");
        // The script list: DFLT, its default language system naming feature 0.
        let script_list: Vec<u8> = [
            words(&[1]),
            b"DFLT".to_vec(),
            words(&[8, 4, 0, 0, 0xFFFF, 1, 0]),
        ]
        .concat();
        let feature_list: Vec<u8> = [
            words(&[1]),
            b"kern".to_vec(),
            words(&[8, 0, count]),
            words(&(0..count).collect::<Vec<_>>()),
        ]
        .concat();
        let mut lookup_list = words(&[count]);
        let mut at = 2 + 2 * count;
        let mut bodies = Vec::new();
        for subtable in subtables {
            lookup_list.extend(words(&[at]));
            let body = [words(&[2, 0, 1, 8]), subtable.clone()].concat();
            at += u16::try_from(body.len()).expect("small");
            bodies.extend(body);
        }
        lookup_list.extend(bodies);
        let scripts = 10_u16;
        let features = scripts + u16::try_from(script_list.len()).expect("small");
        let lookups = features + u16::try_from(feature_list.len()).expect("small");
        [
            words(&[1, 0, scripts, features, lookups]),
            script_list,
            feature_list,
            lookup_list,
        ]
        .concat()
    }

    /// A format 1 coverage table of sorted glyphs.
    fn coverage(glyphs: &[u16]) -> Vec<u8> {
        [
            words(&[1, u16::try_from(glyphs.len()).expect("few")]),
            words(glyphs),
        ]
        .concat()
    }

    /// `PairPosFormat1` with one pair, each value record given in its format's fields.
    fn pair_format1(pair: (u16, u16), formats: (u16, u16), values: (&[u16], &[u16])) -> Vec<u8> {
        let set = [words(&[1, pair.1]), words(values.0), words(values.1)].concat();
        let head = 12_u16;
        let set_at = head;
        let coverage_at = set_at + u16::try_from(set.len()).expect("small");
        [
            words(&[1, coverage_at, formats.0, formats.1, 1, set_at]),
            set,
            coverage(&[pair.0]),
        ]
        .concat()
    }

    /// `PairPosFormat2` with two classes a side, class 1 against class 1 kerned by `value`.
    fn pair_format2(first: u16, second: u16, value: i16) -> Vec<u8> {
        // Records: (0,0) (0,1) (1,0) (1,1), value format 1 = XAdvance only.
        let records = words(&[0, 0, 0, value.cast_unsigned()]);
        let head = 16_u16;
        let coverage_at = head + u16::try_from(records.len()).expect("small");
        let class1_at = coverage_at + 6;
        let class2_at = class1_at + 8;
        [
            words(&[2, coverage_at, 0x0004, 0, class1_at, class2_at, 2, 2]),
            records,
            coverage(&[first]),
            words(&[1, first, 1, 1]),
            words(&[1, second, 1, 1]),
        ]
        .concat()
    }

    fn face(tables: &[([u8; 4], Vec<u8>)]) -> Arc<[u8]> {
        let bare = crate::embedding::without_tables(LIBERATION, &[*b"GPOS", *b"kern"])
            .expect("Liberation Sans is an sfnt");
        crate::embedding::with_tables(&bare, tables)
            .expect("the tables go in")
            .into()
    }

    /// The adjustments of `glyphs`, every one taken as Latin.
    fn advances(pairs: &Pairs, glyphs: &[u16], right_to_left: bool) -> Vec<Adjustment> {
        written(pairs, glyphs, &vec!['A'; glyphs.len()], right_to_left)
    }

    /// The adjustments of `glyphs`, each glyph of the script its character in `text` resolves to.
    fn written(
        pairs: &Pairs,
        glyphs: &[u16],
        text: &[char],
        right_to_left: bool,
    ) -> Vec<Adjustment> {
        let glyphs: Vec<Option<u16>> = glyphs.iter().copied().map(Some).collect();
        pairs
            .adjust(
                &glyphs,
                &scripts(text),
                Language::default(),
                &vec![right_to_left; glyphs.len()],
            )
            .glyphs
    }

    const HORIZONTAL: u16 = 0x0001;

    #[test]
    fn a_kern_table_pair_widens_the_room_after_its_left_glyph() {
        let (a, v) = (glyph('A'), glyph('V'));
        let program = face(&[(*b"kern", kern_table(&[(HORIZONTAL, vec![(a, v, -300)])]))]);
        let pairs = Pairs::read(program, EM).expect("the pair is read");
        let adjusted = advances(&pairs, &[a, v, a], false);
        assert!(
            (adjusted[0].advance - (-300.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
        assert_eq!(
            adjusted[1],
            Adjustment::default(),
            "V A is no pair the table states"
        );
        // Read right to left, the right-hand glyph is the first: V then A is the pair A V.
        let adjusted = advances(&pairs, &[v, a], true);
        assert!(
            (adjusted[0].advance - (-300.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
    }

    #[test]
    fn kern_subtables_add_up_and_an_override_replaces_the_sum() {
        let (a, v) = (glyph('A'), glyph('V'));
        let added = face(&[(
            *b"kern",
            kern_table(&[
                (HORIZONTAL, vec![(a, v, -300)]),
                (HORIZONTAL, vec![(a, v, -100)]),
            ]),
        )]);
        let adjusted = advances(&Pairs::read(added, EM).expect("read"), &[a, v], false);
        assert!(
            (adjusted[0].advance - (-400.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
        let overridden = face(&[(
            *b"kern",
            kern_table(&[
                (HORIZONTAL, vec![(a, v, -300)]),
                (HORIZONTAL | 0x0008, vec![(a, v, -100)]),
            ]),
        )]);
        let adjusted = advances(&Pairs::read(overridden, EM).expect("read"), &[a, v], false);
        assert!(
            (adjusted[0].advance - (-100.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
    }

    #[test]
    fn a_minimum_subtable_is_not_applied_and_is_said() {
        let (a, v) = (glyph('A'), glyph('V'));
        let program = face(&[(
            *b"kern",
            kern_table(&[
                (HORIZONTAL, vec![(a, v, -300)]),
                (HORIZONTAL | 0x0002, vec![(a, v, -10)]),
            ]),
        )]);
        let pairs = Pairs::read(program, EM).expect("read");
        assert!(pairs.unread().is_some());
        let adjusted = advances(&pairs, &[a, v], false);
        assert!(
            (adjusted[0].advance - (-300.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
    }

    #[test]
    fn gpos_is_read_before_the_kern_table() {
        let (a, v) = (glyph('A'), glyph('V'));
        let program = face(&[
            (*b"kern", kern_table(&[(HORIZONTAL, vec![(a, v, -300)])])),
            (
                *b"GPOS",
                gpos_table(&[pair_format1(
                    (a, v),
                    (0x0004, 0),
                    (&[(-200_i16).cast_unsigned()], &[]),
                )]),
            ),
        ]);
        let adjusted = advances(&Pairs::read(program, EM).expect("read"), &[a, v, a], false);
        assert!(
            (adjusted[0].advance - (-200.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
        assert_eq!(adjusted[1], Adjustment::default());
    }

    #[test]
    fn a_class_pair_kerns_every_glyph_of_its_classes() {
        let (t, o, a) = (glyph('T'), glyph('o'), glyph('A'));
        let program = face(&[(*b"GPOS", gpos_table(&[pair_format2(t, o, -150)]))]);
        let pairs = Pairs::read(program, EM).expect("read");
        let adjusted = advances(&pairs, &[t, o, a, o], false);
        assert!(
            (adjusted[0].advance - (-150.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
        assert_eq!(
            adjusted[2],
            Adjustment::default(),
            "A is in no class of the pair"
        );
    }

    #[test]
    fn a_second_value_record_places_the_second_glyph_and_takes_it_out_of_the_next_pair() {
        let (a, v) = (glyph('A'), glyph('V'));
        // A V: first's XAdvance -200, second's XPlacement +50; V A: -100, in a later lookup.
        let program = face(&[(
            *b"GPOS",
            gpos_table(&[
                pair_format1(
                    (a, v),
                    (0x0004, 0x0001),
                    (&[(-200_i16).cast_unsigned()], &[50]),
                ),
                pair_format1((v, a), (0x0004, 0), (&[(-100_i16).cast_unsigned()], &[])),
            ]),
        )]);
        let adjusted = advances(&Pairs::read(program, EM).expect("read"), &[a, v, a], false);
        assert!(
            (adjusted[0].advance - (-200.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
        assert!(
            (adjusted[1].placement - 50.0 / EM).abs() < 1e-6,
            "{adjusted:?}"
        );
        // The second lookup is its own pass, so V A is a pair there.
        assert!(
            (adjusted[1].advance - (-100.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
    }

    #[test]
    fn a_pair_consumed_in_one_lookup_is_not_the_first_of_the_next_there() {
        let (a, v) = (glyph('A'), glyph('V'));
        let first = pair_format1(
            (a, v),
            (0x0004, 0x0001),
            (&[(-200_i16).cast_unsigned()], &[50]),
        );
        let second = pair_format1((v, a), (0x0004, 0), (&[(-100_i16).cast_unsigned()], &[]));
        // Both subtables in one lookup: after A V, the next pair starts at the second A.
        let joined = gpos_one_lookup(&[first, second]);
        let adjusted = advances(
            &Pairs::read(face(&[(*b"GPOS", joined)]), EM).expect("read"),
            &[a, v, a],
            false,
        );
        assert!(
            (adjusted[0].advance - (-200.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
        assert!(adjusted[1].advance.abs() < 1e-9, "{adjusted:?}");
    }

    /// A `GPOS` table whose `kern` feature reaches one lookup holding every subtable given.
    fn gpos_one_lookup(subtables: &[Vec<u8>]) -> Vec<u8> {
        let count = u16::try_from(subtables.len()).expect("few");
        let script_list: Vec<u8> = [
            words(&[1]),
            b"DFLT".to_vec(),
            words(&[8, 4, 0, 0, 0xFFFF, 1, 0]),
        ]
        .concat();
        let feature_list: Vec<u8> = [words(&[1]), b"kern".to_vec(), words(&[8, 0, 1, 0])].concat();
        let mut lookup = words(&[2, 0, count]);
        let mut at = 6 + 2 * count;
        let mut bodies = Vec::new();
        for subtable in subtables {
            lookup.extend(words(&[at]));
            at += u16::try_from(subtable.len()).expect("small");
            bodies.extend(subtable.clone());
        }
        lookup.extend(bodies);
        let lookup_list = [words(&[1, 4]), lookup].concat();
        let scripts = 10_u16;
        let features = scripts + u16::try_from(script_list.len()).expect("small");
        let lookups = features + u16::try_from(feature_list.len()).expect("small");
        [
            words(&[1, 0, scripts, features, lookups]),
            script_list,
            feature_list,
            lookup_list,
        ]
        .concat()
    }

    #[test]
    fn a_vertical_value_is_said_rather_than_applied() {
        let (a, v) = (glyph('A'), glyph('V'));
        let program = face(&[(
            *b"GPOS",
            gpos_table(&[pair_format1(
                (a, v),
                (0x0006, 0),
                (&[30, (-200_i16).cast_unsigned()], &[]),
            )]),
        )]);
        let pairs = Pairs::read(program, EM).expect("read");
        let adjusted = pairs.adjust(
            &[Some(a), Some(v)],
            &[Script::default(); 2],
            Language::default(),
            &[false, false],
        );
        assert!(adjusted.vertical);
        assert!(
            (adjusted.glyphs[0].advance - (-200.0 / EM)).abs() < 1e-6,
            "{adjusted:?}"
        );
    }

    #[test]
    fn a_program_without_either_table_has_no_pairs() {
        assert_eq!(
            Pairs::read(face(&[]), EM).map(|_| ()),
            Err(NoPairs::NoPairData)
        );
        assert!(
            Pairs::read(LIBERATION.into(), EM).is_ok(),
            "Liberation Sans kerns by GPOS"
        );
    }

    /// A `GPOS` table of the given script tables, each one's default language system naming the
    /// one `kern` feature given by index; each feature names its lookups, and each lookup is
    /// `(type, subtable)`. The script tags are given sorted, as `ScriptList` requires.
    fn gpos_built(
        scripts: &[(&[u8; 4], u16)],
        features: &[&[u16]],
        lookups: &[(u16, Vec<u8>)],
    ) -> Vec<u8> {
        let n = u16::try_from(scripts.len()).expect("few");
        let mut script_list = words(&[n]);
        for (at, (tag, _)) in (0_u16..).zip(scripts) {
            script_list.extend(tag.as_slice());
            script_list.extend(words(&[2 + 6 * n + 12 * at]));
        }
        for (_, feature) in scripts {
            // defaultLangSysOffset 4, no language-specific systems; the LangSys names one feature.
            script_list.extend(words(&[4, 0, 0, 0xFFFF, 1, *feature]));
        }
        gpos_of(script_list, features, lookups)
    }

    /// A `GPOS` table of one script table holding a default language system, where `default`
    /// names a feature, and one language system `language` naming the feature given by index.
    fn gpos_language(
        script: [u8; 4],
        default: Option<u16>,
        language: (&[u8; 4], u16),
        features: &[&[u16]],
        lookups: &[(u16, Vec<u8>)],
    ) -> Vec<u8> {
        let mut script_list = words(&[1]);
        script_list.extend(script);
        script_list.extend(words(&[8]));
        // The Script table: its default LangSys at 10 (or none), one LangSysRecord to the
        // language's LangSys at 18; each LangSys names one feature.
        script_list.extend(words(&[if default.is_some() { 10 } else { 0 }, 1]));
        script_list.extend(language.0.as_slice());
        script_list.extend(words(&[18]));
        script_list.extend(words(&[0, 0xFFFF, 1, default.unwrap_or(0)]));
        script_list.extend(words(&[0, 0xFFFF, 1, language.1]));
        gpos_of(script_list, features, lookups)
    }

    /// A `GPOS` table of the given script list, a `kern` feature per entry of `features` naming
    /// its lookups, and the lookups, each `(type, subtable)`.
    fn gpos_of(script_list: Vec<u8>, features: &[&[u16]], lookups: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let f = u16::try_from(features.len()).expect("few");
        let mut feature_list = words(&[f]);
        let mut at = 2 + 6 * f;
        for named in features {
            feature_list.extend(b"kern");
            feature_list.extend(words(&[at]));
            at += 4 + 2 * u16::try_from(named.len()).expect("few");
        }
        for named in features {
            feature_list.extend(words(&[0, u16::try_from(named.len()).expect("few")]));
            feature_list.extend(words(named));
        }
        let l = u16::try_from(lookups.len()).expect("few");
        let mut lookup_list = words(&[l]);
        let mut at = 2 + 2 * l;
        let mut bodies = Vec::new();
        for (kind, subtable) in lookups {
            lookup_list.extend(words(&[at]));
            let body = [words(&[*kind, 0, 1, 8]), subtable.clone()].concat();
            at += u16::try_from(body.len()).expect("small");
            bodies.extend(body);
        }
        lookup_list.extend(bodies);
        let scripts_at = 10_u16;
        let features_at = scripts_at + u16::try_from(script_list.len()).expect("small");
        let lookups_at = features_at + u16::try_from(feature_list.len()).expect("small");
        [
            words(&[1, 0, scripts_at, features_at, lookups_at]),
            script_list,
            feature_list,
            lookup_list,
        ]
        .concat()
    }

    /// `PairPosFormat1` kerning one pair by an `XAdvance` on its first glyph.
    fn kerns(pair: (u16, u16), value: i16) -> (u16, Vec<u8>) {
        (
            2,
            pair_format1(pair, (0x0004, 0), (&[value.cast_unsigned()], &[])),
        )
    }

    fn close(adjustment: Adjustment, units: f32) -> bool {
        (adjustment.advance - units / EM).abs() < 1e-6
    }

    #[test]
    fn a_neutral_character_takes_the_script_beside_it() {
        let code = |text: &str| -> Vec<Option<[u8; 4]>> {
            scripts(&text.chars().collect::<Vec<_>>())
                .into_iter()
                .map(Script::code)
                .collect()
        };
        let (latin, greek) = (Some(*b"Latn"), Some(*b"Grek"));
        // A leading bracket takes the script after it; the digit, space and stop the one before.
        assert_eq!(code("(A1 Ω."), [latin, latin, latin, latin, greek, greek]);
        // A combining mark is `Inherited`, and takes its base's.
        assert_eq!(code("e\u{301}"), [latin, latin]);
        assert_eq!(code("12."), [None, None, None], "no specific script at all");
        // Hiragana and Katakana are two scripts and one script table.
        let kana = scripts(&['あ', 'ア']);
        assert_ne!(kana[0], kana[1]);
        assert_eq!(kana[0].run(), kana[1].run());
    }

    #[test]
    fn every_script_reaches_a_registered_tag() {
        let registry: std::collections::BTreeSet<[u8; 4]> =
            include_str!("../../../data/opentype/script-tags.txt")
                .lines()
                .filter(|line| !line.starts_with('#'))
                .map(|line| {
                    let tag = line.split('\t').next().expect("a tag column");
                    let tag = tag.trim_matches('\'').as_bytes();
                    <[u8; 4]>::try_from(tag).expect("a four-byte tag")
                })
                .collect();
        let mut reached = std::collections::BTreeSet::new();
        for (_, _, code) in &super::tables::SCRIPTS {
            for tag in super::tags(*code).into_iter().flatten() {
                assert!(
                    registry.contains(&tag.to_be_bytes()),
                    "{} maps to {tag}, which the registry does not hold",
                    String::from_utf8_lossy(code)
                );
                reached.insert(tag.to_be_bytes());
            }
        }
        // What no `Script` value reaches is what is no script of text: the default table, math
        // layout, the two music notations `Scripts.txt` gives as `Common`, and the Jamo tag the
        // registry says not to use.
        let unreached: Vec<&[u8; 4]> = registry.difference(&reached).collect();
        assert_eq!(
            unreached,
            [b"DFLT", b"byzm", b"jamo", b"math", b"musc"],
            "{unreached:?}"
        );
    }

    #[test]
    fn a_run_takes_the_kern_lookups_of_its_own_script() {
        let (a, v) = (glyph('A'), glyph('V'));
        // DFLT kerns A V by -100 and latn by -300; a reading of every script's lookups together
        // applies both, -400.
        let program = face(&[(
            *b"GPOS",
            gpos_built(
                &[(b"DFLT", 0), (b"latn", 1)],
                &[&[0], &[1]],
                &[kerns((a, v), -100), kerns((a, v), -300)],
            ),
        )]);
        let pairs = Pairs::read(program, EM).expect("read");
        let latin = written(&pairs, &[a, v], &['A', 'V'], false);
        assert!(close(latin[0], -300.0), "{latin:?}");
        // Cyrillic has no table of its own here, so it is laid out under DFLT.
        let cyrillic = written(&pairs, &[a, v], &['А', 'В'], false);
        assert!(close(cyrillic[0], -100.0), "{cyrillic:?}");
        // So is text of no specific script.
        let neutral = written(&pairs, &[a, v], &['1', '2'], false);
        assert!(close(neutral[0], -100.0), "{neutral:?}");
    }

    #[test]
    fn a_script_with_a_table_of_its_own_does_not_borrow_dflts() {
        let (a, v, t, o) = (glyph('A'), glyph('V'), glyph('T'), glyph('o'));
        // latn's table kerns only T o, so A V in Latin is not kerned although DFLT states it.
        let program = face(&[(
            *b"GPOS",
            gpos_built(
                &[(b"DFLT", 0), (b"latn", 1)],
                &[&[0], &[1]],
                &[kerns((a, v), -100), kerns((t, o), -200)],
            ),
        )]);
        let pairs = Pairs::read(program, EM).expect("read");
        let latin = written(&pairs, &[a, v, t, o], &['A', 'V', 'T', 'o'], false);
        assert!(latin[0].advance.abs() < 1e-9, "{latin:?}");
        assert!(close(latin[2], -200.0), "{latin:?}");
    }

    #[test]
    fn two_glyphs_of_two_scripts_are_no_pair() {
        let (a, v) = (glyph('A'), glyph('V'));
        let program = face(&[(
            *b"GPOS",
            gpos_built(&[(b"DFLT", 0)], &[&[0]], &[kerns((a, v), -100)]),
        )]);
        let pairs = Pairs::read(program, EM).expect("read");
        assert!(close(
            written(&pairs, &[a, v], &['A', 'V'], false)[0],
            -100.0
        ));
        let across = written(&pairs, &[a, v], &['A', 'Ω'], false);
        assert!(across[0].advance.abs() < 1e-9, "{across:?}");
    }

    #[test]
    fn a_language_tag_selects_the_registry_s_language_systems() {
        let tags = |tag: &str| Language::of(tag).tags().collect::<Vec<_>>();
        assert_eq!(
            tags("tr"),
            [*b"TRK "],
            "ISO 639-1, through the three-letter code"
        );
        assert_eq!(
            tags("TR-tr"),
            [*b"TRK "],
            "case is not significant (§14.9.2.2)"
        );
        assert_eq!(
            tags("ur-Arab-PK"),
            [*b"URD "],
            "script and region select nothing"
        );
        assert_eq!(tags("deu"), [*b"DEU "], "a three-letter code as it stands");
        assert_eq!(tags("de-1996"), [*b"DEU "]);
        // An extended-language subtag is tried before its macrolanguage, which the registry
        // often lists alone: Egyptian Arabic has no tag, Arabic has `ARA `.
        assert_eq!(tags("ar-arz"), [*b"ARA "]);
        assert_eq!(tags("arz"), [] as [[u8; 4]; 0]);
        // The registry's deprecated `DHV ` comes after `DIV `.
        assert_eq!(tags("dv"), [*b"DIV ", *b"DHV "]);
        // Chinese has five tags, tried in the registry's order.
        assert_eq!(tags("zh-Hant").len(), 5);
        for unknown in ["", "x-private", "i-klingon", "zxx", "qaa", "language", "e"] {
            assert!(tags(unknown).is_empty(), "{unknown:?}");
        }
    }

    #[test]
    fn every_registered_code_is_reached_by_its_own_tag() {
        // Every row of the registry naming a code is selected by that code: the table the
        // build compiled is the registry, read in full.
        let registry = include_str!("../../../data/opentype/language-tags.txt");
        let mut rows = 0;
        for line in registry.lines().filter(|line| !line.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            let tag: [u8; 4] = fields[0]
                .trim_matches('\'')
                .as_bytes()
                .try_into()
                .expect("a four-byte tag");
            for code in fields[1].split(',').filter(|code| !code.is_empty()) {
                assert!(
                    Language::of(code).tags().any(|reached| reached == tag),
                    "{code} does not reach {}",
                    String::from_utf8_lossy(&tag)
                );
                rows += 1;
            }
        }
        assert!(rows > 600, "{rows} codes");
    }

    #[test]
    fn a_language_system_replaces_the_default_one() {
        let (a, v, t, o) = (glyph('A'), glyph('V'), glyph('T'), glyph('o'));
        // latn's default system kerns A V by -100; its TRK system kerns T o by -200 alone.
        let program = face(&[(
            *b"GPOS",
            gpos_language(
                *b"latn",
                Some(0),
                (b"TRK ", 1),
                &[&[0], &[1]],
                &[kerns((a, v), -100), kerns((t, o), -200)],
            ),
        )]);
        let pairs = Pairs::read(program, EM).expect("read");
        let glyphs = [Some(a), Some(v), Some(t), Some(o)];
        let latin = scripts(&['A', 'V', 'T', 'o']);
        let laid =
            |language: &str| pairs.adjust(&glyphs, &latin, Language::of(language), &[false; 4]);
        let turkish = laid("tr").glyphs;
        assert!(turkish[0].advance.abs() < 1e-9, "{turkish:?}");
        assert!(close(turkish[2], -200.0), "{turkish:?}");
        // A language the table registers no system for, and no language, take the default.
        for other in ["de", ""] {
            let default = laid(other).glyphs;
            assert!(close(default[0], -100.0), "{other:?}: {default:?}");
            assert!(default[2].advance.abs() < 1e-9, "{other:?}: {default:?}");
        }
    }

    #[test]
    fn a_program_kerning_only_in_a_language_system_kerns_that_language() {
        let (a, v) = (glyph('A'), glyph('V'));
        // No default system at all: only Urdu text is kerned, and the program still reads as
        // `GPOS` rather than falling to a `kern` table.
        let program = face(&[(
            *b"GPOS",
            gpos_language(
                *b"arab",
                None,
                (b"URD ", 0),
                &[&[0]],
                &[kerns((a, v), -150)],
            ),
        )]);
        let pairs = Pairs::read(program, EM).expect("read");
        let arabic = scripts(&['ب', 'ت']);
        let glyphs = [Some(a), Some(v)];
        let urdu = pairs.adjust(&glyphs, &arabic, Language::of("ur"), &[true; 2]);
        assert!(close(urdu.glyphs[0], -150.0), "{urdu:?}");
        let arabic_only = pairs.adjust(&glyphs, &arabic, Language::of("ar"), &[true; 2]);
        assert!(
            arabic_only.glyphs[0].advance.abs() < 1e-9,
            "{arabic_only:?}"
        );
    }

    #[test]
    fn a_second_version_tag_is_chosen_before_the_first() {
        let (a, v) = (glyph('A'), glyph('V'));
        let both = face(&[(
            *b"GPOS",
            gpos_built(
                &[(b"dev2", 0), (b"deva", 1)],
                &[&[0], &[1]],
                &[kerns((a, v), -300), kerns((a, v), -100)],
            ),
        )]);
        let devanagari = ['क', 'ख'];
        let pairs = Pairs::read(both, EM).expect("read");
        assert!(close(
            written(&pairs, &[a, v], &devanagari, false)[0],
            -300.0
        ));
        let older = face(&[(
            *b"GPOS",
            gpos_built(&[(b"deva", 0)], &[&[0]], &[kerns((a, v), -100)]),
        )]);
        let pairs = Pairs::read(older, EM).expect("read");
        assert!(close(
            written(&pairs, &[a, v], &devanagari, false)[0],
            -100.0
        ));
    }

    /// `ChainContextPos` format 3: after `backtrack`, the `input` pair takes lookup `nested` at
    /// its first glyph.
    fn chained(backtrack: u16, input: (u16, u16), nested: u16) -> (u16, Vec<u8>) {
        let table = [
            words(&[3, 1, 20, 2, 26, 32, 0, 1, 0, nested]),
            coverage(&[backtrack]),
            coverage(&[input.0]),
            coverage(&[input.1]),
        ]
        .concat();
        (8, table)
    }

    #[test]
    fn a_pair_positioned_by_its_context_is_not_pair_kerning() {
        let (t, a, v) = (glyph('T'), glyph('A'), glyph('V'));
        // The kern feature names lookup 0, A V after T by way of lookup 1 at -250, and lookup 2,
        // V A at -100. Lookup 1 is reached only through the context.
        let program = face(&[(
            *b"GPOS",
            gpos_built(
                &[(b"DFLT", 0)],
                &[&[0, 2]],
                &[
                    chained(t, (a, v), 1),
                    kerns((a, v), -250),
                    kerns((v, a), -100),
                ],
            ),
        )]);
        let pairs = Pairs::read(program, EM).expect("the pair lookup is read");
        let adjusted = written(&pairs, &[t, a, v, a], &['T', 'A', 'V', 'A'], false);
        // OpenType's whole kern feature would move A by -250 here; pair kerning, by the two
        // adjacent glyphs alone, does not, and V A is kerned as the program states it.
        assert!(adjusted[1].advance.abs() < 1e-9, "{adjusted:?}");
        assert!(close(adjusted[2], -100.0), "{adjusted:?}");
        // A feature reaching contextual lookups alone states no pair kerning at all.
        let contextual_only = face(&[(
            *b"GPOS",
            gpos_built(
                &[(b"DFLT", 0)],
                &[&[0]],
                &[chained(t, (a, v), 1), kerns((a, v), -250)],
            ),
        )]);
        assert_eq!(
            Pairs::read(contextual_only, EM).map(|_| ()),
            Err(NoPairs::NoPairData)
        );
    }
}
