//! Every document in the pdf.js corpus, opened, interpreted and rasterised.
//!
//! # What this gate is for
//!
//! The other tests in this crate check that a feature works. This one checks that nothing
//! *breaks* — across 974 real documents produced by every generator anyone has pointed at
//! pdf.js over fifteen years, including a good number that are damaged, truncated or
//! deliberately hostile.
//!
//! Three things are asserted, and they are different in kind:
//!
//! 1. **Nothing panics.** A panic on untrusted input is a denial of service in a viewer
//!    and, in a crate that forbids unsafe code, the only way a malformed file can take the
//!    process down. Every failure must arrive as a typed error.
//! 2. **Nothing silently disappears.** A content stream that reaches an operator we do not
//!    implement must say so through [`pdf_model::Interpretation::unsupported`]. A viewer
//!    that draws nine tenths of a page and reports success is worse than one that admits
//!    what it left out, because nobody can tell from looking.
//! 3. **The numbers do not get worse.** The counts below are a ratchet. They are what the
//!    corpus produces today, and a change that raises any of them fails the build until
//!    the number is deliberately edited.
//!
//! # Why a ratchet rather than zero
//!
//! Some of these documents cannot be rendered by anything: they are fuzzer output and
//! truncation tests, present in pdf.js precisely to check that a reader refuses them
//! cleanly. Demanding zero failures would mean demanding that we render files with no
//! valid cross-reference table and no recoverable objects, which is not a coherent goal.
//! Demanding that the count never rises is coherent, and it catches the regression that
//! matters: a change that quietly stops handling a class of documents.
//!
//! # Running it
//!
//! The corpus is the `doc/pdf.js` submodule. When it is absent the test reports that and
//! passes, so a checkout without submodules is not a broken build — but CI has it, and the
//! ratchet only means anything where it runs.

#![expect(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "test code: an explanatory panic is the intended failure, and the survey \
              output is the point of the run"
)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, SyntaxError};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use render_cpu::CpuRasterizer;

/// Pixel budget per page, generous enough that no real page reaches it.
const PIXEL_BUDGET: u64 = 64 << 20;

/// Documents that cannot be opened at all.
///
/// Zero, and it should stay zero: every file here yields *something*, even the fuzzed and
/// truncated ones, because recovery by scanning for `obj` headers works when no
/// cross-reference table does.
const MAX_UNOPENABLE: usize = 0;

/// Documents that are encrypted and refuse the default user password.
///
/// Eight, and they are not a defect: ISO 32000-2 §7.6.4.1 says a reader "shall first try to
/// authenticate the encrypted document using the padding string … (default user password)"
/// and prompt when that fails, which is what a viewer with a window does and what this gate
/// cannot do. Seven have passwords the pdf.js manifest records, and
/// `crates/pdf-syntax/tests/encryption.rs` opens every one of them with it; the eighth,
/// `print_protection.pdf`, has one nobody has recorded and `poppler` refuses it too.
///
/// This count going *down* would mean a password had started working that should not.
const MAX_LOCKED: usize = 8;

/// Documents whose encryption this reader does not implement.
///
/// Two, both named at runtime rather than drawn as noise (ADR 0031). `issue21579.pdf` is
/// `/R 5`, which Table 21 describes as "a deprecated proprietary Adobe extension" and states
/// no algorithm for, so implementing it would mean copying another reader rather than
/// reading the standard — the one row in this gate that is a *decision* rather than work
/// owed. `PDFBOX-4352-0.pdf` is a damaged file whose trailer names an `/Encrypt` that does
/// not resolve to a dictionary at all; `poppler` cannot read its cross-reference table
/// either. Refusing is the only honest answer to a file that says it is encrypted and will
/// not say how.
const MAX_UNREADABLE_ENCRYPTION: usize = 2;

/// Documents that open but whose first page cannot be reached.
///
/// Eleven, and it was nineteen until the twenty-second session: the eight that were
/// "encrypted, which is unimplemented" now decrypt (ADR 0031), and what is left is eleven
/// files whose page tree genuinely cannot be recovered. Two of those eleven,
/// `issue19484_1.pdf` and `issue19484_2.pdf`, *are* encrypted and authenticate correctly —
/// their object streams then fail to inflate whether decrypted or not, which is what
/// `poppler` reports of them too ("Unknown compression method in flate stream", ten times).
/// A damaged file, not a clause.
///
/// This number is worth more than it looks. It was twenty until running this gate for the
/// first time: `outline_goto_action.pdf` declares twelve cross-reference entries and writes
/// eleven, so the twelfth read the `trailer` keyword, and resuming after that keyword
/// stepped over the only thing naming `/Root`. A document with every object intact produced
/// no catalogue and no pages. `pdf-syntax`'s robustness suite now pins it.
const MAX_PAGELESS: usize = 11;

/// Documents whose first page interprets with something reported as unsupported.
///
/// 189, and *not* a defect count — it is the honest-reporting requirement working. The
/// breakdown, by each document's first report, which is the useful part, and recomputed
/// every session because a number nothing recomputes is a number that drifts — this table
/// had said 263 while the ratchet below said 251:
///
/// | reported | count | why |
/// |---|---|---|
/// | `Text` | 67 | see below; embedded `CMap`s and `/CIDToGIDMap` have left this row |
/// | `Annotation` | 24 | no appearance stream whose shape a clause states (ADR 0030) |
/// | `TransparencyGroup` | 18 | §11.4 and §11.5.3, described below |
/// | `Image` | 12 | see below; one joined when an encrypted document became readable |
/// | `Operator` | 9 | malformed streams, and three fewer now that ciphertext is decrypted |
/// | `CompositedInParts` | 4 | §11.6.2, new in the fifteenth session and described below |
/// | `Content` | 1 | a `/Contents` stream that did not decode |
/// | `TextKnockout` | 1 | §9.3.8, new in the fourteenth session and described below |
/// | `LimitReached` | 1 | a bound reached and said so, which is the design |
///
/// **The `Content` row was 10 and is 1**, and the `Operator` row 12 and is 9, for one
/// reason: §7.6's encryption is implemented (ADR 0031). Nine of those ten `Content` reports
/// were an encrypted `/Contents` refusing to inflate because it was ciphertext, and three of
/// the `Operator` reports were the same ciphertext lexing as operator names — `issue15893_reduced.pdf`
/// announced an operator called `)` and two of byte soup. Six of those twelve documents now
/// draw with nothing reported and six say they need a password, which is the honest form of
/// what they were saying badly.
///
/// **The `Text` row fell by 33 documents in the twentieth session** and two other rows rose by
/// one each, which is the same 31 documents rather than a regression: a document whose font
/// report was its *first* now reports its annotation or its transparency group first. §9.7's
/// composite fonts are implemented (ADR 0029), so an embedded `CMap` stream and a
/// `/CIDToGIDMap` in any of its forms both draw. Counting *fonts* rather than documents, the
/// row is 67: 27 with no `/ToUnicode` so a substitute cannot be addressed, 21 whose substitute
/// draws none of the codes the document declares, 15 naming one of Table 116's predefined
/// `CMap`s — which are registered data files rather than an algorithm, so this is a licensing
/// decision — 4 asking for vertical writing, and the rest malformed programs. **Nothing left on
/// it is a `CMap` question.**
///
/// **The `Shading` row is gone.** It held 28 documents, every one of them a soft mask in an
/// `/ExtGState` — which is transparency rather than shading, and was filed there because
/// nothing else fitted. §11.5's masks are implemented as of the eighteenth session (ADR
/// 0027), so 17 documents left this list outright and the rest report something narrower:
/// 7 that the luminosity of their mask group is taken in device RGB rather than in the
/// blending colour space its `/CS` names (§11.5.3), and 1, `knockout_smask.pdf`, that its
/// group is a *knockout* one — a report that had been hidden behind the mask report, because
/// the condition for it is that an element composites and a mask is what makes this one do so.
///
/// The count **rose** by six in the seventeenth session, and both directions of that are the
/// design. Six documents joined by saying that their `/Group` is a *knockout* group (§11.4.6)
/// and one that its group is non-isolated with an element that blends (§11.4.4) — two silences
/// ending, on the pages where the two models can differ rather than on every group there is.
/// One left: `issue15372.pdf` reported §9.3.8's text knockout only because a constant alpha
/// reached its glyphs, and §11.6.6 resets that constant inside the group the glyphs are in,
/// so the report no longer fires and the alpha is applied once, to the group.
///
/// The `Operator` row was 33 before §9.3.6's rendering modes were implemented. What remains
/// is `BT` without `ET`, `BDC` without `EMC`, and the byte soup a fuzzed content stream
/// lexes as operator names — nothing on it is a feature anybody could implement.
///
/// The `Image` row was 161 before JBIG2 and JPEG 2000 landed, 42 after, 30 once inline images
/// (§8.9.7) drew and `Indexed`, `Separation` and `DeviceN` images unpacked, 18 once
/// `CCITTFaxDecode` decoded (§7.4.6), 13 once `/Mask` was applied in both its forms (§8.9.6.3
/// and §8.9.6.4), and is 11 now that an `/SMask` of another size is combined with its image on
/// the finer of the two grids (§11.6.5.2 Table 143). What is left of it is one image apiece,
/// and **nothing on it is a feature**: 4 malformed streams, 3 bit depths the unpacker refuses,
/// one `/Mask` that is not an image mask and so is outside what Table 87 defines the entry to
/// hold, one JBIG2 with a segment type ISO/IEC 14492 does not define, one 212-megapixel JPEG
/// 2000 scan larger than the sandbox is given room to decode, and one `/SMask` — 34862×4332
/// against a 2×2 image — whose combined grid `image::MAX_MASK_GRID` refuses.
///
/// The `CompositedInParts` row is §11.6.2, which says the portions of one object are not
/// composited with one another: `B` fills and strokes one path, and this renderer emits two
/// commands, so the band a centred stroke shares with the fill composites twice under a paint
/// that composites at all. 4 documents reach the report and one has nothing else to say, which
/// is the row here. Its condition is narrow on purpose — the paint has to composite *and* both
/// parts have to mark the page — and three of the six documents that fill and stroke under a
/// `gs` are silent because one of their two parts has an alpha of zero.
///
/// This number has gone *up* five times, and every rise was the point.
///
/// One, when §11.6.2's fill-and-stroke started reporting, described above. It is the smallest
/// rise on this list and it cost the oracle one page it had judged as *agreeing* —
/// `alphatrans.pdf`, whose gradient the same session fixed — which is the whole of the trade a
/// report makes: honesty about a difference nobody can see yet, paid for in comparison.
///
/// Ten, when content-stream decoding started reporting. Nine of those are encrypted
/// documents whose content stream is unreadable without decryption, and they had been
/// rendering as blank pages returning `unsupported: []` — a wrong page indistinguishable
/// from a sparse one. The tenth is `bomb_giant.pdf`, refusing a decompression bomb.
///
/// Five, when text render modes 4 to 7 started reporting. Those modes add the glyphs to
/// the clipping path (ISO 32000-2 §9.3.6), which we did not build, so a rectangle painted
/// afterwards to be seen only through the letters covered its whole area instead. The
/// reference-oracle gate found two of these drawing a solid bar over the text while
/// claiming to be complete; see `oracle.rs`. **All eight modes are implemented as of the
/// thirteenth session**, and the report is gone — which is what a rise is supposed to end
/// in, and the reason a rise is not a regression.
///
/// Five more, when an image's `/Mask` started reporting. An explicit mask or a colour-key
/// range makes part of an image transparent (§8.9.6.3 and §8.9.6.4) and neither was applied,
/// so `colorkeymask.pdf` drew a band all three references correctly hide. Found the same
/// way. **Both forms are implemented as of the fourteenth session**, and five of those six
/// documents have left this count — the sixth writes a `/Mask` that is not an image mask,
/// which is outside what Table 87 defines the entry to hold, and still reports.
///
/// Two, when §9.3.8's text knockout started reporting — the ledger's third `silent` row
/// closed at the cheap end. `Tk`'s initial value is *true*, which makes a text object a
/// non-isolated knockout group so that a later glyph overwrites an earlier one where they
/// overlap; we composite each glyph separately, which is the `Tk` false model. The report
/// costs two documents rather than several hundred because both of the clause's conditions
/// are tested rather than assumed: the paint has to composite — a constant alpha below one
/// or a blend mode other than Normal, since opaque Normal painting gives both models the
/// same pixels — and two glyphs of one text object have to overlap. The looser version of
/// this check, which asked only for two glyphs under a compositing paint, reported seven
/// documents and took three *agreeing* pages out of the oracle's gated set for a difference
/// that could not have been on any of them.
///
/// Seventy-seven, when annotation appearance streams started being drawn — the largest rise
/// yet, and the one that most needs explaining, because it accompanied a *feature landing*.
/// Before it, 148 of 988 first pages carried a visible annotation with an `/AP` and none was
/// drawn or reported; the page simply came out missing its form fields and its highlights,
/// saying nothing. Those now draw. What newly reports is the other side of the same walk:
/// 63 documents carry an annotation with **no** appearance stream at all, which would have
/// to be synthesised from `/IC`, `/C`, `/BS` and the subtype's own rules — 26 `Widget`,
/// 18 `Link`, and the rest markup annotations. 7 set `/NeedAppearances` on their interactive
/// form, which §12.7.4.3 makes a statement that the stored appearance is *not* the one to
/// draw: the field's value is computed at viewing time, so its appearance has to be
/// constructed then. The stored one is still drawn, because it is all the file offers, and
/// the report is what keeps that from passing as correct. 3 carry a malformed appearance —
/// no `/BBox`, no usable `/Rect`, or a stream that did not decode.
///
/// 68 of those 73 documents had reported nothing at all before. The other 9 of the 77 are
/// documents whose *appearance streams* draw content this crate already reports elsewhere:
/// a CID font, a JPX image, a transparency group, now met inside an annotation rather than
/// inside the page — which is why the `Image`, `Text` and `Operator` rows above also grew.
///
/// And down by 118 this session, when JBIG2 and JPEG 2000 started decoding — the largest
/// single fall so far, and the first that came from a *dependency* rather than from code
/// written here. See doc/adr/0014 for why that was the right call and what it costs.
///
/// Down by one and up by 41 this session, and every part of it is one piece of work:
/// implementing ISO 32000-2 §9.6.5.4, the algorithm that turns a character code into an
/// index into a `TrueType` font's `cmap`. `issue5501.pdf` left this list, because its font's
/// only `cmap` subtable is one that algorithm reaches and the previous code did not. The
/// two rises are both gaps that algorithm's absence had been hiding.
///
/// **Type 3 fonts, 24 documents.** A Type 3 font has no font program at all: §9.6.4 makes
/// each glyph a content stream in `/CharProcs`, which is the interpreter's work and not this
/// crate's. Every one of these documents was therefore reaching the *substitution* path,
/// where the names in a Type 3 `/Differences` array — `/a192`, `/g3`, names of procedures —
/// were resolved against a Latin system font. `issue918.pdf` drew 388 text operations of
/// letter fragments at the wrong places and reported `unsupported: []`; poppler draws a page
/// of readable text. This is trap 1 exactly, and the rise is that page saying so.
///
/// **A substitute that draws none of the codes the document declares, 19 documents.** The
/// old test asked whether the substitute reached *any* of the 256 codes, which a Latin face
/// always does — so a font whose `/FirstChar`..`/LastChar` range mapped to nothing at all
/// still passed. `issue20504.pdf` set a line of Chinese in a Type 1 program this crate
/// cannot read, and all four of its codes name glyphs only the original font had; the line
/// drew nothing and said nothing. `tracemonkey.pdf` and its five relatives are the smaller
/// case, and the more instructive one: a Type 1 `CMSY7` subset whose single declared code is
/// `/circlecopyrt`, so the © in the copyright line is missing from a page that otherwise
/// draws perfectly.
///
/// That rule is deliberately about the font rather than about each code. A font that maps
/// *some* of its declared codes and not others is still silent about the rest, which needs
/// a report at the point a glyph is shown rather than at the point a font is loaded.
///
/// Ratcheted downward otherwise: this falls as features land, and a rise that is not a new
/// *report* means something that used to draw no longer does.
///
/// **290 to 280 in the tenth session, and the arithmetic is worth reading rather than the
/// total.** Type 3 fonts landed (§9.6.4), which removed the report from all 24 documents
/// carrying one — and only 10 of them became complete. The other 14 immediately began
/// reporting something the Type 3 refusal had been standing in front of: 10 draw their
/// glyphs as *inline images*, which this interpreter does not decode, one carries a soft
/// mask, two use a stroking text render mode and one has a malformed number in a glyph
/// description. That is this file's own habit in miniature — fixing the mask shows what the
/// mask was hiding — and it is why a feature landing moves this number by less than the
/// count of documents it was blamed for.
///
/// **And 280 back up to 283**, which is the other half of the same session and a rise of the
/// only kind this ratchet allows: three documents began saying that their text is set
/// *vertically*. `Identity-V` was accepted beside `Identity-H` because the two map codes
/// identically — and they differ in the writing mode, which §9.2.4 gives a second set of
/// metrics no part of this tree reads. `vertical.pdf` should set two columns down the right
/// edge of the page; it came out as one overlapping line across the top, reporting
/// `unsupported: []`. Nothing stopped drawing correctly.
///
/// **283 to 263 in the eleventh session, and again the arithmetic rather than the total.**
/// Inline images (§8.9.7) took 13 documents off this list and named the reason on the other
/// 9 — an inline image now reports `CCITTFaxDecode` or a bit depth rather than the bare word
/// `<inline>`. `Indexed`, `Separation`, `DeviceN` and `Lab` images unpack, which took another
/// 10. And 3 came *back*: `chrome-text-selection-markedContent.pdf`, `issue16263.pdf` and
/// `smaskdim.pdf` carry a soft mask whose sample grid is not their image's, which §11.6.5.2
/// Table 143 expressly permits and this tree does not apply. All three were drawing an
/// unmasked image in silence; `issue16263.pdf` puts black bars across its text.
///
/// **263 to 251 in the twelfth session**, and for once the arithmetic is simple: 12
/// documents reported `CCITTFaxDecode` and none of them reports anything else, so all 12
/// became complete when §7.4.6 landed. Nothing came back — which is worth stating rather
/// than passing over, because every other feature in this list uncovered something behind
/// it. What CCITT uncovered is not a *report*, it is a picture: `bug1001080.pdf` is now
/// contradicted by the oracle for a reason that has nothing to do with the filter, and
/// `oracle.rs`'s `CONTRADICTED_IMAGE_RESAMPLING` has it.
///
/// **251 to 235 in the thirteenth session**, and the arithmetic is again simple: §9.3.6's
/// eight text rendering modes are all implemented, so the report that stood in for four of
/// them is gone from 18 documents — 16 of which report nothing else and became complete. The
/// `Operator` row falls from 33 to 15 and what is left on it is malformed streams rather
/// than anything unimplemented. Nothing came back, and nothing newly appeared: no corpus
/// document names a `Tr` operand outside Table 104's eight.
///
/// One of the 16 is worth knowing about, because it is a picture rather than a count.
/// `recursiveCompositGlyf.pdf` shows "hello world" in mode 7 and then paints the page red,
/// expecting to see it through the letters — and its font is a deliberately malformed
/// TrueType whose composite glyph refers to itself. `skrifa` produces no outline for it, so
/// §9.3.6's "if the only glyphs shown have no outlines … no clipping shall occur" applies
/// and the page comes out solidly red. So do poppler's and `hayro`'s; `mupdf` refuses the
/// font and draws nothing; only `ghostscript`, with its own TrueType interpreter, recovers
/// the glyphs. That is a *font* question about a malformed file, not a rendering-mode one,
/// and it is the visible face of a gap this project already knows about: a font reports as a
/// whole, so a glyph that fails to load draws nothing and says nothing.
/// **137 to 129 in the twenty-third session**, and this one has a rise inside it. §12.7.4.3's
/// variable text closed the whole annotation half of the list a clause was owed for: 9
/// documents stopped saying `/NeedAppearances`, 3 stopped saying a widget holds a value, and 4
/// `FreeText` annotations stopped saying their text needs laying out. What replaced them is 5
/// documents whose `/DA` names a font the interactive form dictionary's `/DR` does not define,
/// which is a sharper statement about the same files — and one document, `checkbox_no_appearance.pdf`,
/// that had been silent and now says a check box it draws as empty is one the file calls
/// checked. Nothing on this row is a `/NeedAppearances` any longer.
///
/// **129 to 130 in the twenty-fourth session, and the one that joined is a silence ending.**
/// Reading §8.4.5 against Table 57 found four entries of a graphics state parameter
/// dictionary that reached nothing at all. Three of them — `/LC`, `/LJ` and `/ML` — are now
/// implemented and report nothing, because the operators `J`, `j` and `M` set the same three
/// parameters and always had. The fourth, `/Font`, selects a font by *indirect reference*
/// rather than by the resource name this crate's font cache is keyed on, and one document
/// writes it: `extgstate.pdf`, which now says so instead of drawing its text in whatever font
/// was current. Trap 5's rule, and the price is one page leaving the oracle's judged set.
///
/// **130 to 110 in the thirtieth session, the largest fall it has had since JBIG2**, and the
/// feature that caused it was on the "not implemented" list with a corpus count of **zero**
/// beside it. §9.9's `/FontFile` — a bare Type 1 font program — is now read, and 20 documents
/// stopped reporting. The zero was measuring *reports*, not documents: an unreadable embedded
/// program fell through to substitution, and substitution only speaks when it can address none
/// of the declared codes, so a page set in a Type 1 font drew in some other typeface and said
/// nothing. Trap 5's rule from the other side — the report that never fired was the one for a
/// feature that had a fallback.
///
/// Nothing joined the row, and one thing nearly did. `issue5751.pdf` and
/// `issue11740_reduced.pdf` are **`CIDFonts`** whose descendant descriptors embed a
/// `/FontFile`, which §9.9's Table 124 does not allow there — a Type 1 program is keyed by
/// glyph name and a `CIDFont` selects by CID, so the clause states no route between them. The
/// first draft read the program anyway and reported that it was not an sfnt, which named the
/// wrong defect: the program is fine and its *placement* is what the clause forbids. They get
/// what any `CIDFont` with no usable program gets, which is a substitute.
///
/// **110 to 106 in the thirty-second session**, and the feature was three lines of packing:
/// §8.9.5.1's Table 87 permits five component widths and the unpacker read two, so 2, 4 and
/// 16 bits were refused and named. Three documents were waiting on that; the fourth is
/// `issue14256.pdf`, whose 4-bit image was one of eight inline images testing §8.9.7's
/// abbreviations against their full names, and which now draws all eight alike.
///
/// **106 to 105 in the thirty-third session.** §8.4.5's Table 57 `/Font` selects a font by
/// indirect reference rather than by resource name, and this crate's font cache was keyed by
/// the name — so `extgstate.pdf`, a page whose text says "I should be courier!", said instead
/// that it could not address the font. The cache is now keyed by either, and the page draws in
/// Courier and agrees with the reference consensus.
///
/// **105 to 97 in the thirty-fourth session**, all of them §12.5.6.10's text markup
/// annotations, which had been refused on the argument that the clause "states its
/// /QuadPoints without stating what mark to make in them". Reading it again, it states the
/// mark ("shall appear as highlights, underlines, strikeouts … or jagged ('squiggly')
/// underlines"), the region and the orientation, and leaves a thickness — which is a choice
/// to argue rather than a reason to draw nothing. Eight pages joined the oracle's judged set
/// and six of them agree with the reference consensus; none is contradicted.
const MAX_INCOMPLETE: usize = 97;

/// How long one document may take before it counts as a failure.
///
/// A viewer that takes half a minute to open a page has failed to open it. This bounds a
/// single document rather than the suite so that a failure names the file.
///
/// # This bound reports; it cannot enforce
///
/// The elapsed time is checked after the work finishes, because a Rust thread cannot be
/// cancelled from outside. A document that genuinely never returns hangs this test rather
/// than failing it. Bounding the work itself belongs inside the interpreter and the
/// rasteriser, which is where principle 3's "explicit time budgets" have to live; this is
/// the detector, not the guard. `cargo run --release -p pdf-model --example open_one` runs
/// one document in a process that *can* be killed, which is how a hang gets isolated.
const PER_DOCUMENT_BUDGET: Duration = Duration::from_secs(30);

/// Documents already known to exceed [`PER_DOCUMENT_BUDGET`], with the reason.
///
/// Named rather than counted, so that a new slow document fails the gate even though the
/// total has not risen — and so that fixing the cause deletes an entry rather than
/// decrementing a number nobody can interpret.
///
/// **Empty, and it earned that.** `bug1721218_reduced.pdf` was the only entry: a 612×792
/// page holding 3576 distinct clips, which rasterised in 39.6 s and held 1.7 GB. The CPU
/// backend now draws each command into the rows its clip admits rather than into the page
/// (ADR 0010), which takes it to 0.24 s and 25 MB of masks. Keeping the list empty is the
/// point: the next document to cross the budget fails the gate rather than joining a
/// list.
const KNOWN_SLOW: [&str; 0] = [];

/// What happened to one document.
#[derive(Debug, Default)]
struct Tally {
    unopenable: Vec<String>,
    locked: Vec<String>,
    unreadable_encryption: Vec<String>,
    pageless: Vec<String>,
    incomplete: Vec<(String, String)>,
    slow: Vec<(String, Duration)>,
}

/// Names a document on stderr when `PDFVIEWER_CORPUS_TRACE` is set.
///
/// Stderr rather than stdout because the test harness buffers stdout, and the whole value
/// of this is that it survives the run being killed.
fn trace(what: &str, name: &str) {
    if std::env::var_os("PDFVIEWER_CORPUS_TRACE").is_some() {
        eprintln!("{what} {name}");
    }
}

/// Adds to the shared tally, ignoring a poisoned lock.
///
/// A poisoned lock means another document's examination panicked, which the test as a
/// whole will report; losing one tally entry to it changes nothing.
fn record(tally: &Mutex<Tally>, update: impl FnOnce(&mut Tally)) {
    if let Ok(mut tally) = tally.lock() {
        update(&mut tally);
    }
}

/// The corpus files, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    if files.is_empty() {
        return None;
    }
    files.sort();
    Some(files)
}

/// Opens, interprets and rasterises one document's first page.
///
/// Returns what went wrong, or nothing. Rasterisation is included because it is where a
/// display list with impossible geometry — an infinite coordinate, a degenerate transform —
/// would surface, and the interpreter is perfectly capable of producing one.
fn examine(path: &Path, tally: &Mutex<Tally>) {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into(),
    );
    let started = Instant::now();
    // Named on stderr before and after, so that a document which never returns can be
    // identified from a killed run. There is no way to bound the work from outside: a
    // thread cannot be cancelled, so a genuinely unbounded loop hangs the suite and this
    // trace is the only thing that says which file caused it.
    trace("start", &name);

    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let document = match Document::open(bytes) {
        Ok(document) => document,
        // ISO 32000-2 §7.6.4.1 has a reader try the default user password and then prompt.
        // A file that refuses it is *locked*, not unreadable, and the distinction is the
        // point: the first is a document waiting for a person and the second is work owed.
        Err(SyntaxError::PasswordRequired) => {
            record(tally, |t| t.locked.push(name));
            return;
        }
        Err(SyntaxError::UnsupportedEncryption { .. }) => {
            record(tally, |t| t.unreadable_encryption.push(name));
            return;
        }
        Err(_) => {
            record(tally, |t| t.unopenable.push(name));
            return;
        }
    };
    let Some(page) = pdf_model::Pages::new(&document).get(0) else {
        record(tally, |t| t.pageless.push(name));
        return;
    };

    let interpretation = pdf_model::interpret(&document, &page);
    if !interpretation.is_complete() {
        let reported = format!("{:?}", interpretation.unsupported);
        record(tally, |t| t.incomplete.push((name.clone(), reported)));
    }

    // A page whose extent cannot be targeted — empty, or larger than the budget — is a
    // reported outcome rather than a defect, so it is not counted.
    if let Ok(target) = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET) {
        // The result is discarded deliberately: an unsupported command is a *reported*
        // outcome, already counted above. What this call is here to prove is that the
        // rasteriser returns rather than panicking or looping.
        drop(CpuRasterizer::new().rasterize(&interpretation.display_list, target));
    }

    let taken = started.elapsed();
    trace("done ", &name);
    if taken > PER_DOCUMENT_BUDGET {
        record(tally, |t| t.slow.push((name, taken)));
    }
}

/// Fails the gate if the sandboxed decoder is not available.
///
/// JBIG2 and JPEG 2000 are decoded by a separate program, and Cargo does not build another
/// package's binaries when it tests this one. Without that check a missing worker would not
/// fail anything — it would quietly turn 152 documents' images into reports and move the
/// ratchets, which is the kind of silent number change this whole file exists to prevent.
fn require_the_sandbox() {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
        panic!(
            "the sandboxed image decoder is not available, so the counts below would be \
             wrong: {error}"
        );
    }
}

/// The gate.
///
/// Ignored by default because it is a minute of work in release and fifteen in debug —
/// too slow to sit in the edit-test loop, and misleading there anyway, since the timing
/// bound is meaningless at debug speed. Run it deliberately:
///
/// ```text
/// cargo test --release -p pdf-model --test corpus -- --ignored --nocapture
/// ```
///
/// `PDFVIEWER_CORPUS_TRACE=1` additionally names each document on stderr as it starts and
/// finishes, which is how a document that never returns is identified from a killed run.
#[test]
#[ignore = "one minute over 974 documents; run explicitly, in release"]
fn the_corpus_opens_interprets_and_rasterises() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let tally = Mutex::new(Tally::default());
    let started = Instant::now();
    files.par_iter().for_each(|path| examine(path, &tally));
    let elapsed = started.elapsed();

    let tally = tally.into_inner().expect("no examination panicked");

    println!(
        "{} documents in {:.1}s: {} unopenable, {} locked, {} encrypted beyond us, \
         {} pageless, {} incomplete, {} slow",
        files.len(),
        elapsed.as_secs_f64(),
        tally.unopenable.len(),
        tally.locked.len(),
        tally.unreadable_encryption.len(),
        tally.pageless.len(),
        tally.incomplete.len(),
        tally.slow.len()
    );
    for (name, reported) in &tally.incomplete {
        println!("  incomplete: {name}: {reported}");
    }
    for name in &tally.locked {
        println!("  locked: {name}");
    }
    for name in &tally.unreadable_encryption {
        println!("  encryption we do not implement: {name}");
    }
    for name in tally.unopenable.iter().chain(&tally.pageless) {
        println!("  unusable: {name}");
    }
    for (name, taken) in &tally.slow {
        println!("  slow: {name}: {taken:?}");
    }

    let unexpected: Vec<&(String, Duration)> = tally
        .slow
        .iter()
        .filter(|(name, _)| !KNOWN_SLOW.contains(&name.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "a document must not take longer than {PER_DOCUMENT_BUDGET:?} to open and draw: \
         {unexpected:?}"
    );
    assert!(
        tally.unopenable.len() == MAX_UNOPENABLE,
        "{} documents cannot be opened, was {MAX_UNOPENABLE}",
        tally.unopenable.len()
    );
    assert!(
        tally.locked.len() <= MAX_LOCKED,
        "{} documents need a password, was {MAX_LOCKED}",
        tally.locked.len()
    );
    assert!(
        tally.unreadable_encryption.len() <= MAX_UNREADABLE_ENCRYPTION,
        "{} documents are encrypted in a way this reader does not implement, was \
         {MAX_UNREADABLE_ENCRYPTION}",
        tally.unreadable_encryption.len()
    );
    assert!(
        tally.pageless.len() <= MAX_PAGELESS,
        "{} documents have no reachable first page, was {MAX_PAGELESS}",
        tally.pageless.len()
    );
    assert!(
        tally.incomplete.len() <= MAX_INCOMPLETE,
        "{} documents draw incompletely, was {MAX_INCOMPLETE}",
        tally.incomplete.len()
    );
}
