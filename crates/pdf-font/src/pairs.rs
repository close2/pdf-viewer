//! The pair kerning an embedded font program states, read out of the program itself.
//!
//! XFA 3.3 chapter 27's `kerning-mode:pair` (*Kerning*, pages 1203 and 1204) asks for kerning
//! "based purely on the two adjacent glyphs" (the version 2.8 change list), and the numbers are
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
//! - **The feature is found by tag alone**, every script's together, as `crate::vertical` finds
//!   `vert`: a PDF names no script or language system for a field's text, and the lookups apply
//!   in lookup-list order as OpenType applies the lookups of one feature set.
//! - **Only pair adjustment is read.** A contextual lookup under the same feature positions
//!   glyphs by their context, which is not the "two adjacent glyphs" chapter 27 names.
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
    /// `GPOS`'s `kern` feature: the pair-adjustment lookups it reaches, in lookup-list order.
    Positioning(Vec<u16>),
    /// The legacy `kern` table.
    Kerning,
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
                Some(lookups) => (Source::Positioning(lookups), None),
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

    /// The adjustments of a run of glyphs in logical order; `right_to_left` says, per glyph,
    /// whether it reads right to left. A `None` glyph — a code reaching none of the program's —
    /// pairs with nothing, and two glyphs of different directions are not a pair.
    #[must_use]
    pub fn adjust(&self, glyphs: &[Option<u16>], right_to_left: &[bool]) -> Adjusted {
        let mut out = Adjusted {
            glyphs: vec![Adjustment::default(); glyphs.len()],
            vertical: false,
        };
        let Ok(font) = FontRef::new(&self.data) else {
            return out;
        };
        let pair = |at: usize| {
            let first = glyphs.get(at).copied().flatten()?;
            let second = glyphs.get(at.checked_add(1)?).copied().flatten()?;
            let direction = right_to_left.get(at).copied().unwrap_or(false);
            let other = right_to_left
                .get(at.checked_add(1)?)
                .copied()
                .unwrap_or(false);
            (direction == other).then_some((first, second, direction))
        };
        match &self.source {
            Source::Positioning(lookups) => {
                self.positioned(&font, lookups, glyphs.len(), &pair, &mut out);
            }
            Source::Kerning => self.kerned(&font, glyphs.len(), &pair, &mut out),
        }
        out
    }

    /// OpenType's pair adjustment, one lookup at a time over the whole run.
    ///
    /// Within a lookup the first subtable that matches a pair applies, and where its second
    /// value format is not empty the pair's second glyph is not the first of the next pair:
    /// "[i]f valueFormat2 is set to 0, then the second glyph of the pair is the 'next' glyph for
    /// which a lookup should be performed" (ISO/IEC 14496-22, `GPOS`, `PairPosFormat1`).
    fn positioned(
        &self,
        font: &FontRef,
        lookups: &[u16],
        count: usize,
        pair: &dyn Fn(usize) -> Option<(u16, u16, bool)>,
        out: &mut Adjusted,
    ) {
        let Ok(gpos) = font.gpos() else {
            return;
        };
        let Ok(list) = gpos.lookup_list() else {
            return;
        };
        for index in lookups {
            let Ok(lookup) = list.lookups().get(usize::from(*index)) else {
                continue;
            };
            let Ok(PositionSubtables::Pair(subtables)) = lookup.subtables() else {
                continue;
            };
            let subtables: Vec<PairPos> = subtables.iter().flatten().collect();
            let mut at = 0_usize;
            while at.saturating_add(1) < count {
                let Some((first, second, _)) = pair(at) else {
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

/// The pair-adjustment lookups `GPOS`'s `kern` feature reaches, ascending; `None` where the
/// program registers no such feature, or one reaching no pair adjustment.
fn kern_feature_lookups(font: &FontRef) -> Result<Option<Vec<u16>>, NoPairs> {
    let Ok(gpos) = font.gpos() else {
        return Ok(None);
    };
    let (Ok(features), Ok(list)) = (gpos.feature_list(), gpos.lookup_list()) else {
        return Err(NoPairs::Unreadable);
    };
    let mut indices = std::collections::BTreeSet::new();
    for record in features
        .feature_records()
        .iter()
        .filter(|record| record.feature_tag() == Tag::new(b"kern"))
    {
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
    let mut subtables = 0_usize;
    let mut pairs = Vec::new();
    for index in indices {
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
        pairs.push(index);
    }
    Ok((!pairs.is_empty()).then_some(pairs))
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

    use super::{Adjustment, NoPairs, Pairs};
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

    fn advances(pairs: &Pairs, glyphs: &[u16], right_to_left: bool) -> Vec<Adjustment> {
        let glyphs: Vec<Option<u16>> = glyphs.iter().copied().map(Some).collect();
        pairs
            .adjust(&glyphs, &vec![right_to_left; glyphs.len()])
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
        let adjusted = pairs.adjust(&[Some(a), Some(v)], &[false, false]);
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
}
