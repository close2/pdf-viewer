//! ISO 19005-4 section 6.2.10.5's Type 3 width: a glyph procedure's `d0` or `d1` horizontal width
//! written as the font's `/Widths` states it, under the loss `type3-width-as-drawn` (`doc/adr/2907`,
//! `doc/questions/A865`).
//!
//! # What the clauses say
//!
//! ISO 32000-2 §9.6.4's Table 111 states the operand twice over, once for each operator:
//!
//! > w x denotes the horizontal displacement in the glyph coordinate system; it shall be
//! > consistent with the corresponding width in the font's Widths array.
//!
//! and Table 110 makes `/Widths` required and puts it in glyph space, read through the
//! `/FontMatrix`, the same space the operand is in. Section 6.2.10.5 of part 4 holds a file to the
//! table's sentence to within a thousandth of a unit of text space.
//!
//! # Why the edit moves no mark
//!
//! §9.2.4 makes the font dictionary's width what advances the text position, and this reader
//! reads `d0`'s and `d1`'s width nowhere (`pdf_model`'s `run_type3_op`): the page is laid out by
//! `/Widths` before the edit and after it. So the operand written is the number the window already
//! draws by, written where Table 111 asks for it — ADR 2638's construction — and nothing else in
//! the procedure changes: not `wy`, not `d1`'s bounding box, not one byte after the operand. It is
//! still the loss `type3-width-as-drawn`, because the operand the producer wrote is gone from the
//! procedure, and `xmpMM:History` keeps it.
//!
//! # A procedure reached at two widths
//!
//! One `/CharProcs` stream can be named by two fonts whose `/Widths` give it two widths, or by two
//! codes of one font at two widths, and no one operand holds both. The procedure is then copied,
//! one copy for each further width: the codes the procedure already agrees with keep it unedited, a
//! font whose codes all need one copy is given a `/CharProcs` naming it under the same glyph name,
//! and a code of a font whose other codes keep the procedure is given a name of its own for the
//! copy, its `/Differences` entry written with it. The name is the old one with a suffix after a
//! period, which the Adobe Glyph List's rule reads as the same character, so a code read by its
//! name reads as it did.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Arc;

use pdf_archive::survey::Survey;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::{Document, Lexer, Token};

use super::decision::Because;
use super::prepare::Spare;

/// The requirement this answers.
pub(super) const SITE: &str = "fonts/type3-glyph-procedures-state-their-width";

/// ISO 19005-4 section 6.2.10.5's tolerance, a thousandth of a unit of text space, applied as the
/// validator applies it (`pdf_archive`'s `type3_glyph_procedures_state_their_width`).
const CONSISTENT: f64 = 0.001;

/// How many tokens of a procedure are read looking for its first operator, the validator's bound:
/// §9.6.4 makes `d0` or `d1` the first operator, and a hostile stream is not read to its end.
const PROCEDURE_TOKENS: usize = 64;

/// The most procedures one conversion restates: `CLAUDE.md` principle 3's explicit budget, at
/// one decoded stream kept per procedure.
const MOST_PROCEDURES: usize = 1 << 14;

/// The most widths `xmpMM:History` names one by one; the rest are counted.
const MOST_NAMED: usize = 16;

/// Why a mismatched procedure this conversion cannot read keeps the refusal.
const PROCEDURE_UNREAD: &str = "a Type 3 glyph procedure whose width disagrees with its font's \
     Widths does not decode, or does not open with d0 or d1 and a number, so its operand cannot \
     be found to be written";

/// Why a font written directly in a resource dictionary keeps the refusal where it needs a copy.
const DIRECT_FONT: &str = "a Type 3 font written directly in a resource dictionary reaches a \
     glyph procedure another font reaches at another width, and only an indirect font can be \
     given a CharProcs of its own";

/// Why a procedure stream this conversion cannot write back keeps the refusal.
const WILL_NOT_RE_ENCODE: &str = "a Type 3 glyph procedure whose width is to be written will \
     not re-encode, or its DecodeParms names another object that discarding the parameters would \
     orphan";

/// Why a document past [`MOST_PROCEDURES`] keeps the refusal.
const TOO_MANY: &str = "more than 16384 Type 3 glyph procedures disagree with their fonts' \
     Widths, more than this conversion restates";

/// Why a procedure another stream repair also writes keeps the refusal.
const ALSO_ANOTHER_REPAIR: &str = "a Type 3 glyph procedure whose width is to be written is also \
     written by another repair of its content, and the two are not composed";

/// Why no free object number was left for a copy.
const NO_SPARE_OBJECT: &str = "no free object number is left for a copy of a Type 3 glyph \
     procedure two fonts reach at two widths";

/// One width a conversion wrote into a Type 3 glyph procedure, under the loss
/// `type3-width-as-drawn` (`doc/adr/2907`).
#[derive(Debug, Clone, PartialEq)]
pub struct RestatedType3Width {
    /// The font whose `/Widths` states the width, where the font is an indirect object.
    pub font: Option<ObjectId>,
    /// The glyph name, the procedure's key in the font's `/CharProcs`.
    pub glyph: String,
    /// The procedure the producer wrote, in the source's numbering.
    pub procedure: ObjectId,
    /// Where the width was written: the procedure itself, or the copy made for this font.
    pub written_at: ObjectId,
    /// `d0` or `d1`.
    pub operator: String,
    /// The operand as the producer wrote it.
    pub was: String,
    /// The width written, the font's `/Widths` entry.
    pub width: f64,
}

/// What this conversion writes for the row.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// The procedures written in place, by object.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// The copies, at spare object numbers.
    pub(super) written: BTreeMap<ObjectId, Object>,
    /// Each font given a `/CharProcs` naming a copy, with the entries it is given: that
    /// `/CharProcs`, and an `/Encoding` where a code is renamed for its copy.
    pub(super) char_procs: BTreeMap<ObjectId, Dictionary>,
    /// Every width written, in procedure order.
    pub(super) widths: Vec<RestatedType3Width>,
}

/// One font's claim on one procedure.
#[derive(Debug, Clone)]
struct Claim {
    font: Option<ObjectId>,
    code: i64,
    glyph: String,
    width: f64,
    scale: f64,
}

/// A procedure's first operator, its first operand's place and the number that operand states.
struct Operand {
    operator: &'static str,
    start: usize,
    end: usize,
    value: f64,
}

/// What every rendered Type 3 font needs written for its procedures to agree with its `/Widths`.
///
/// Every procedure a rendered font's encoding reaches inside `/FirstChar`..`/LastChar` is asked,
/// whether the validator named its font or not, because a procedure written in place is read by
/// every font that names it.
pub(super) fn restate(
    document: &Document,
    survey: Option<&Survey>,
    spare: &mut Spare,
) -> Result<Restated, Because> {
    let claims = claims(document, survey);
    let mut out = Restated::default();
    let mut edits: BTreeMap<ObjectId, FontEdit> = BTreeMap::new();
    for (procedure, claims) in &claims {
        let Some((stream, data, operand)) = read_procedure(document, *procedure) else {
            if claims.iter().any(|claim| disagrees(claim, None)) {
                return Err(Because::NotBuiltYet(PROCEDURE_UNREAD));
            }
            continue;
        };
        if claims.iter().all(|claim| agree(claim, operand.value)) {
            continue;
        }
        if out.at.len().saturating_add(out.written.len()) >= MOST_PROCEDURES {
            return Err(Because::NotBuiltYet(TOO_MANY));
        }
        // The widths the codes need, the one the procedure already states first where any code
        // needs it: that group keeps the procedure as the producer wrote it.
        let mut groups: Vec<(f64, Vec<&Claim>)> = Vec::new();
        for claim in claims {
            match groups.iter_mut().find(|(width, _)| agree(claim, *width)) {
                Some((_, members)) => members.push(claim),
                None => groups.push((claim.width, vec![claim])),
            }
        }
        groups.sort_by_key(|(_, members)| !members.iter().all(|claim| agree(claim, operand.value)));
        let mut placed: Vec<(ObjectId, Vec<&Claim>)> = Vec::new();
        for (index, (width, members)) in groups.into_iter().enumerate() {
            let at = if index == 0 {
                if members
                    .iter()
                    .any(|claim| disagrees(claim, Some(operand.value)))
                {
                    out.at
                        .insert(*procedure, with_width(&stream, &data, &operand, width)?);
                    record(
                        &mut out.widths,
                        &members,
                        *procedure,
                        *procedure,
                        &operand,
                        &data,
                        width,
                    );
                }
                *procedure
            } else {
                let at = spare
                    .take(document)
                    .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
                out.written
                    .insert(at, with_width(&stream, &data, &operand, width)?);
                record(
                    &mut out.widths,
                    &members,
                    *procedure,
                    at,
                    &operand,
                    &data,
                    width,
                );
                at
            };
            placed.push((at, members));
        }
        // Each font's glyph name keeps the procedure it names where any of its codes is in the
        // first group, and otherwise names the copy of the first group it is in; a code of the same
        // name in a further group is given a name of its own for that copy.
        let mut named: BTreeMap<(Option<ObjectId>, &str), ObjectId> = BTreeMap::new();
        for (at, members) in &placed {
            for claim in members {
                let key = (claim.font, claim.glyph.as_str());
                let kept = named.entry(key).or_insert(*at);
                if *kept == *at && *at == *procedure {
                    continue;
                }
                let font = claim.font.ok_or(Because::NotBuiltYet(DIRECT_FONT))?;
                let edit = match edits.entry(font) {
                    std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(FontEdit::of(document, font)?)
                    }
                };
                if *kept == *at {
                    edit.char_procs
                        .insert(Name::new(claim.glyph.as_bytes()), Object::Reference(*at));
                } else {
                    let renamed = edit.name_for(&claim.glyph, *at);
                    edit.codes.insert(claim.code, renamed);
                }
            }
        }
    }
    for (font, edit) in edits {
        out.char_procs.insert(font, edit.finish(document, font));
    }
    Ok(out)
}

/// What one font is given: its `/CharProcs`, and its `/Encoding` where a code is renamed.
struct FontEdit {
    char_procs: Dictionary,
    codes: BTreeMap<i64, String>,
}

impl FontEdit {
    /// The font's own `/CharProcs`, to be edited.
    fn of(document: &Document, font: ObjectId) -> Result<Self, Because> {
        let object = document.get(font);
        let dict = object.as_dict().ok_or(Because::NotBuiltYet(DIRECT_FONT))?;
        let procedures = document.get_key(dict, "CharProcs");
        let procedures = procedures
            .as_dict()
            .ok_or(Because::NotBuiltYet(PROCEDURE_UNREAD))?;
        Ok(Self {
            char_procs: procedures.clone(),
            codes: BTreeMap::new(),
        })
    }

    /// A glyph name this font does not use yet for the copy `at` of `glyph`'s procedure: the name
    /// with a suffix after a period, which the Adobe Glyph List's rule reads as the same character
    /// (`pdf_font`'s `encoding` strips it), so a code read by its name reads as it did.
    fn name_for(&mut self, glyph: &str, at: ObjectId) -> String {
        let mut suffix = 1_u32;
        loop {
            let candidate = format!("{glyph}.{suffix}");
            match self.char_procs.get(candidate.as_str()) {
                Some(Object::Reference(existing)) if *existing == at => return candidate,
                Some(_) => suffix = suffix.saturating_add(1),
                None => {
                    self.char_procs
                        .insert(Name::new(candidate.as_bytes()), Object::Reference(at));
                    return candidate;
                }
            }
        }
    }

    /// The entries the font is given: `/CharProcs`, and `/Encoding` with every code §9.6.5.1's
    /// `/Differences` assigned written out again, the renamed codes under their new names.
    fn finish(self, document: &Document, font: ObjectId) -> Dictionary {
        let mut entries = Dictionary::new();
        entries.insert(
            Name::new(&b"CharProcs"[..]),
            Object::Dictionary(self.char_procs),
        );
        if self.codes.is_empty() {
            return entries;
        }
        let object = document.get(font);
        let Some(dict) = object.as_dict() else {
            return entries;
        };
        let mut encoding = document
            .get_key(dict, "Encoding")
            .as_dict()
            .cloned()
            .unwrap_or_default();
        let mut differences = Vec::new();
        for (code, name) in pdf_archive::type3_encoding(document, dict) {
            let name = self.codes.get(&code).cloned().unwrap_or(name);
            differences.push(Object::Integer(code));
            differences.push(Object::Name(Name::new(name.as_bytes())));
        }
        encoding.insert(Name::new(&b"Differences"[..]), Object::Array(differences));
        entries.insert(Name::new(&b"Encoding"[..]), Object::Dictionary(encoding));
        entries
    }
}

/// Every procedure a rendered Type 3 font reaches, with each font's width for it.
fn claims(document: &Document, survey: Option<&Survey>) -> BTreeMap<ObjectId, Vec<Claim>> {
    let mut claims: BTreeMap<ObjectId, Vec<Claim>> = BTreeMap::new();
    let mut seen: BTreeSet<Option<ObjectId>> = BTreeSet::new();
    for used in survey.into_iter().flat_map(Survey::fonts) {
        let is_type3 = document
            .get_key(&used.dict, "Subtype")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Type3");
        if !used.rendered || !is_type3 {
            continue;
        }
        // A direct font is a font of its own wherever it is written, and the survey selects it
        // once per resource dictionary.
        if used.id.is_some() && !seen.insert(used.id) {
            continue;
        }
        let matrix = document.get_key(&used.dict, "FontMatrix");
        let scale = matrix
            .as_array()
            .and_then(|items| items.first())
            .and_then(|first| document.resolve(first).as_number());
        let first = document.get_key(&used.dict, "FirstChar").as_integer();
        let widths = document.get_key(&used.dict, "Widths");
        let procedures = document.get_key(&used.dict, "CharProcs");
        let (Some(scale), Some(first), Some(widths), Some(procedures)) =
            (scale, first, widths.as_array(), procedures.as_dict())
        else {
            continue;
        };
        for (code, glyph) in pdf_archive::type3_encoding(document, &used.dict) {
            let Some(width) = usize::try_from(code.saturating_sub(first))
                .ok()
                .and_then(|at| widths.get(at))
                .and_then(|width| document.resolve(width).as_number())
            else {
                continue;
            };
            let Some(Object::Reference(procedure)) = procedures.get(glyph.as_str()) else {
                continue;
            };
            claims.entry(*procedure).or_default().push(Claim {
                font: used.id,
                code,
                glyph,
                width,
                scale,
            });
        }
    }
    claims
}

/// Whether `claim`'s width disagrees with the operand `drawn`, or with an operand that cannot be
/// read, by more than the clause's tolerance.
fn disagrees(claim: &Claim, drawn: Option<f64>) -> bool {
    drawn.is_none_or(|drawn| ((claim.width - drawn) * claim.scale).abs() > CONSISTENT)
}

/// Whether `width` holds for `claim` to within the clause's tolerance.
fn agree(claim: &Claim, width: f64) -> bool {
    !disagrees(claim, Some(width))
}

/// A procedure's stream, its decoded bytes and its first operand.
fn read_procedure(document: &Document, id: ObjectId) -> Option<(Arc<Stream>, Vec<u8>, Operand)> {
    let Object::Stream(stream) = document.get(id) else {
        return None;
    };
    let data = document.decoded_stream_data(&stream)?.to_vec();
    let operand = first_operand(&data)?;
    Some((stream, data, operand))
}

/// The first operand of a procedure's opening `d0` or `d1`, read as the validator reads it.
fn first_operand(data: &[u8]) -> Option<Operand> {
    let mut lexer = Lexer::new(data);
    let mut first: Option<(usize, usize, f64)> = None;
    for _ in 0..PROCEDURE_TOKENS {
        let before = lexer.position();
        let token = lexer.next_token()?;
        let after = lexer.position();
        let number = match token {
            Token::Integer(value) => Some(f64::from(i32::try_from(value).ok()?)),
            Token::Real(value) => Some(value),
            Token::Keyword(b"d0") => {
                let (start, end, value) = first?;
                return Some(Operand {
                    operator: "d0",
                    start,
                    end,
                    value,
                });
            }
            Token::Keyword(b"d1") => {
                let (start, end, value) = first?;
                return Some(Operand {
                    operator: "d1",
                    start,
                    end,
                    value,
                });
            }
            Token::Keyword(_) => return None,
            _ => None,
        };
        if let Some(value) = number
            && first.is_none()
        {
            first = Some((token_start(data, before, after), after, value));
        }
    }
    None
}

/// Where a token that ended at `after` began, past the white space and comments from `before`
/// (§7.2.3, §7.2.4): a number has no delimiter of its own to start with.
fn token_start(data: &[u8], before: usize, after: usize) -> usize {
    let mut at = before;
    while at < after {
        match data.get(at) {
            Some(b'\0' | b'\t' | b'\n' | b'\x0C' | b'\r' | b' ') => at = at.saturating_add(1),
            Some(b'%') => {
                while at < after && !matches!(data.get(at), Some(b'\n' | b'\r')) {
                    at = at.saturating_add(1);
                }
            }
            _ => break,
        }
    }
    at
}

/// The procedure with its first operand written as `width`, every other byte as it was.
fn with_width(
    stream: &Stream,
    data: &[u8],
    operand: &Operand,
    width: f64,
) -> Result<Object, Because> {
    let mut bytes = Vec::with_capacity(data.len().saturating_add(16));
    bytes.extend_from_slice(data.get(..operand.start).unwrap_or_default());
    bytes.extend_from_slice(spelled(width).as_bytes());
    bytes.extend_from_slice(data.get(operand.end..).unwrap_or_default());
    super::hexadecimal::repaired(stream, &bytes).ok_or(Because::NotBuiltYet(WILL_NOT_RE_ENCODE))
}

/// A width in ISO 32000-2 §7.3.3's spelling: `f64`'s `Display` writes the shortest digits that
/// read back as the same value and never an exponent, which §7.3.3 does not admit.
fn spelled(width: f64) -> String {
    format!("{width}")
}

/// Keeps one record per font of `members`.
fn record(
    out: &mut Vec<RestatedType3Width>,
    members: &[&Claim],
    procedure: ObjectId,
    written_at: ObjectId,
    operand: &Operand,
    data: &[u8],
    width: f64,
) {
    let was = data
        .get(operand.start..operand.end)
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .unwrap_or_default();
    for claim in members {
        out.push(RestatedType3Width {
            font: claim.font,
            glyph: claim.glyph.clone(),
            procedure,
            written_at,
            operator: operand.operator.to_owned(),
            was: was.clone(),
            width,
        });
    }
}

impl Restated {
    /// Whether the restatement writes anything.
    pub(super) fn writes_anything(&self) -> bool {
        !self.widths.is_empty()
    }

    /// The same answer with nothing to write: what a profile that does not grant the loss is
    /// shown, so that it is told the loss rather than that nothing was prepared, while no spare
    /// object it took is written.
    pub(super) fn unwritten(self) -> Self {
        Self {
            widths: self.widths,
            ..Self::default()
        }
    }

    /// The procedures written, in place or as copies, for a stream repair to keep clear of.
    pub(super) fn streams(&self) -> impl Iterator<Item = &ObjectId> {
        self.at.keys()
    }

    /// What `xmpMM:History` says of the widths written: the requirement, the count and the loss,
    /// then each by its procedure, its glyph, the operand the producer wrote and the width written,
    /// the first [`MOST_NAMED`] by name.
    pub(super) fn history(&self, citation: &str) -> String {
        let mut out = format!(
            "{citation}: {} Type 3 glyph width(s) written into their procedures' d0 or d1 as the \
             font's Widths states them; loss {}",
            self.widths.len(),
            super::Loss::Type3WidthAsDrawn.word()
        );
        for (index, written) in self.widths.iter().take(MOST_NAMED).enumerate() {
            // Writing into a `String` cannot fail, so the `write!` result is discarded deliberately.
            let _ = write!(
                out,
                "{}object {} {} /{} {} {} as {}",
                if index == 0 { ": " } else { "; " },
                written.written_at.number,
                written.written_at.generation,
                written.glyph,
                written.operator,
                written.was,
                spelled(written.width)
            );
        }
        if self.widths.len() > MOST_NAMED {
            let _ = write!(
                out,
                ", and {} more",
                self.widths.len().saturating_sub(MOST_NAMED)
            );
        }
        out
    }
}

/// The restatement, unless a procedure it writes is one another stream repair also writes.
pub(super) fn apart_from(
    restated: Restated,
    others: &[&BTreeMap<ObjectId, Object>],
) -> Result<Restated, Because> {
    if restated
        .streams()
        .any(|id| others.iter().any(|other| other.contains_key(id)))
    {
        return Err(Because::NotBuiltYet(ALSO_ANOTHER_REPAIR));
    }
    Ok(restated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_operand_is_found_past_a_comment_and_white_space() {
        let data = b"% glyph a\n  1536 0 d0 0 0 m";
        let operand = first_operand(data).expect("d0 opens it");
        assert_eq!(operand.operator, "d0");
        assert_eq!(data.get(operand.start..operand.end), Some(&b"1536"[..]));
        assert!((operand.value - 1536.0).abs() < f64::EPSILON);
    }

    #[test]
    fn a_procedure_opening_with_another_operator_has_no_operand() {
        assert!(first_operand(b"0 0 m 1 0 d0").is_none());
    }

    #[test]
    fn a_width_is_spelled_without_an_exponent() {
        assert_eq!(spelled(0.0), "0");
        assert_eq!(spelled(1536.0), "1536");
        assert_eq!(spelled(0.0001), "0.0001");
    }
}
