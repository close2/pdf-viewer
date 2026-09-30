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

use pdf_model::Unsupported;
use pdf_model::page::ContentIssue;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, Limits, SyntaxError};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

#[path = "support/corpus_passwords.rs"]
#[expect(
    dead_code,
    reason = "the references' spelling of a password is the oracle's; this gate opens the \
              document itself and has no reference to hand one to"
)]
mod corpus_passwords;
use render_cpu::CpuRasterizer;

/// Pixel budget per page, generous enough that no real page reaches it.
const PIXEL_BUDGET: u64 = 64 << 20;

/// Documents that cannot be opened at all.
///
/// Zero, and it should stay zero: every file here yields *something*, even the fuzzed and
/// truncated ones, because recovery by scanning for `obj` headers works when no
/// cross-reference table does.
///
/// Held as a ceiling with no slack, which over a population that cannot be negative is the
/// equality this was written as.
const MAX_UNOPENABLE: usize = 0;

/// Documents that are encrypted, refuse the default user password, and have no published one.
///
/// Not a defect: ISO 32000-2 §7.6.4.1 says a reader "shall first try to authenticate the
/// encrypted document using the padding string … (default user password)" and prompt when that
/// fails, which is what a viewer with a window does and what this gate cannot do. Where the
/// password is published beside the file, `corpus_passwords` answers the prompt, [`examine`] opens
/// the document with it, and the document is walked like any other; nine are opened that way
/// (ADR 1377). What is left here is a document nobody has a password for.
///
/// `encrypted-attachment.pdf` states no `/AuthEvent`, so Table 25's default of `DocOpen` requires
/// the key at the open; its twin `auth-event-ef-open.pdf` — the same bytes plus that one line — is
/// the file that opens without one (ADR 1040). No password for it is recorded anywhere.
///
/// Named rather than counted, because a ceiling cannot tell a document that *started* needing a
/// password from one that *stopped*, and both are findings — a file this reader stopped
/// decrypting and a file whose password began working are the same number and opposite news (ADR
/// 1081). A published password that stops opening its document lands here by name.
const LOCKED: [&str; 1] = ["encrypted-attachment.pdf"];

/// Documents whose encryption this reader does not implement.
///
/// **`/R 5` is not among them.** Table 21's "Shall not be used. This value was used by a
/// deprecated proprietary Adobe extension" binds a *writer* choosing a value to store, §7.6.4.1
/// states a requirement about revision 5 rather than a silence, and the extension Table 21
/// points at is a document with an algorithm in it, so `issue21579.pdf` opens. ADR 0820 has the
/// argument and says which of its steps rest on evidence. 33 of the 41 `/R 5` documents across
/// the 90 535 in `doc/pdf.js`, `doc/corpora/` and `corpus-cache/` open.
///
/// What is left is `PDFBOX-4352-0.pdf`, a damaged file whose trailer names an `/Encrypt` that
/// does not resolve to a dictionary at all; `poppler` cannot read its cross-reference table
/// either. Refusing is the only honest answer to a file that says it is encrypted and will
/// not say how — and it is named at runtime rather than drawn as noise (ADR 0031).
///
/// # What is beyond us in it is not §7.6 at all
///
/// The paragraph above describes the *file* rather than the clause, so the file was opened. It is
/// 1481
/// bytes and every object is visible: `6 0 obj` reads `E< /CF 7 0 R /Filter /Standard …`, where
/// §7.3.7 puts `<<`, so the object does not parse and §7.3.10 makes `/Encrypt 6 0 R` the null
/// object — "[a]n indirect reference to an undefined object shall not be considered an error by
/// a PDF processor; it shall be treated as a reference to the null object". What the dictionary
/// *says*, once it is read, is `/V 5 /R 6 /StmF /StdCF` over `/CFM /AESV3`, which this reader
/// has had since ADR 0031 and exercises on `issue7665.pdf`.
///
/// **Measured with a control rather than argued** (trap 13): restoring that one byte opens the
/// document on §7.6.4.1's default user password, and page one interprets to nine commands with
/// nothing reported. `pdf-syntax`'s
/// `an_encrypt_entry_naming_an_unparseable_object_is_refused_by_name` is both halves of that,
/// so a round that made `E<` parse would fail the refusal and keep the control.
///
/// So what is beyond us is not a `/V` revision, a crypt filter method, a public-key handler
/// (§7.6.5) or an AES variant: it is a damaged dictionary, and no clause says how to read one
/// whose opening token is gone. §7.3.7's entries-whole recovery (ADR 0784) does not reach it
/// either — that reading keeps the entries readable *before* the damage, and here the damage is
/// the `<<`, so there are none.
///
/// **What the four references do**: `poppler` 26.08 reads the `E` as
/// a keyword and the `<` as a hex string and refuses the catalogue; `ghostscript` 10.07 refuses
/// the cross-reference table; `hayro` refuses the file. `mupdf` 1.28 repairs the
/// cross-reference table, reports "syntax error in object (6 0 R)", **ignores the entry** and
/// then fails to inflate page one's stream — "zlib error: incorrect header check", which is
/// what ciphertext read as plaintext looks like — and writes a blank page. That is the failure
/// mode this refusal exists to avoid, and §7.6.2 is what forbids it: only "[t]he absence of
/// this entry from the trailer dictionary" lets a processor consider the document unencrypted,
/// and the entry is present.
///
/// **Named rather than counted** (ADR 1081), and on a population of one the difference is the
/// whole of it: a ceiling of one holds just as well when
/// the one document is a *different* document, and this reader declining a second file's
/// encryption while learning the first is exactly the swap a count cannot see.
const UNREADABLE_ENCRYPTION: [&str; 1] = [
    // Not a revision, a crypt filter method, a public-key handler or an AES variant: `6 0 obj`
    // reads `E<` where §7.3.7 puts `<<`, so §7.3.10 makes `/Encrypt 6 0 R` the null object and
    // no clause says how to read a dictionary whose opening token is gone. The paragraphs above
    // are the reading of it, with the control that opens the file when the byte is restored.
    "PDFBOX-4352-0.pdf",
];

/// Documents that open but whose first page cannot be reached.
///
/// A document encrypted under a method this reader implements decrypts (ADR 0031), so what is
/// here is files whose page tree cannot be recovered.
///
/// **Two recovery rules keep documents off this list** (ADR 0097).
/// §7.5.5 makes the trailer's `/Root` "[t]he catalog dictionary for the PDF file", so a
/// cross-reference table that leads to no catalog has been disproved by the file itself — and
/// `Document::open` rebuilds by scanning and tries again, where `xref::read` alone scans only
/// when the table is *absent, unreadable or empty*. Table 31 makes `/Type`
/// required of a page object and says it "shall be Page", so a document whose page *tree* yields
/// nothing can still be asked which of its objects say they are pages, with §7.7.3.4's
/// inheritance applied up each one's own `/Parent`. Documents reached that way —
/// `issue18986.pdf` (which then **agrees with the reference consensus**), `issue9418.pdf`,
/// `operator_list_cycle.pdf` among them — are new *pages*, not new failures, where they report
/// something: `issue9418.pdf` is drawn and judged in `oracle.rs`'s
/// `NOT_COMPARABLE_ONE_REFERENCE_REBUILT_THE_FILE`, and `operator_list_cycle.pdf`'s page reports
/// its nesting cycle by name (ADR 1411).
///
/// # A page this tree should never have had
///
/// `poppler-937-0-fuzzed.pdf` is here, and the direction is the honest one rather than a
/// regression. Its `/Pages` node states `/Type /Pages` and a `/Kids` whose `[` was fuzzed into a
/// NUL — which §7.2.3 makes white space — so the entry resolves to a bare dictionary rather than
/// to Table 30's required array. Reading a node with no usable `/Kids` as a **leaf** would draw
/// a page with no `/Contents`, blank, and silent about all of it. §7.7.3.2
/// and §7.7.3.3 settle it in the file's own words, and `page.rs`'s `declares_a_node` is that
/// reading (ADR 0305); nothing in the file declares `/Type /Page` either, so the recovery scan
/// finds nothing and the document has no first page.
///
/// **All three references agree, each having read it independently**, which is principle 5's
/// direction of inference and not the reason: `poppler` prints *Kids object (page 1) is wrong
/// type (dictionary)* and writes one 1×1 pixel, `mutool` refuses with *invalid page number: -1*,
/// and `ghostscript` says *Requested `FirstPage` is greater than the number of pages in the file:
/// 0*. A blank page for it would be this reader's invention.
///
/// # A bound with no slack
///
/// A bound left above the population lets that many documents' worth of regression arrive here
/// before the gate speaks; the run prints the population and a constant does not, so the two
/// are kept equal (ADR 1075).
///
/// # The five, opened one by one
///
/// Nobody had asked of this population the question it is *for*: is the page tree genuinely
/// absent, or is it a tree this reader fails to walk that somebody else walks? Each was put to
/// all four references and looked at, and each answer is placed under `CLAUDE.md` principle 5's
/// three cases. [`why_no_page_one`] is what the run prints beside each name, so the sentences
/// below are arguments rather than a table that can drift from the gate.
///
/// - **`REDHAT-1531897-0.pdf` — the file broke it, and there is nothing behind the damage.** It
///   is a linearised file truncated at 871 bytes of the 7945 its own `/L` states; the two
///   cross-reference streams that survive name `/Root 8 0 R` and `/Info 6 0 R`, and no object
///   past 13 is in the file at all. All four references refuse: `poppler` "Catalog object is
///   wrong type (null)", `mupdf` "truncated xref stream" and then nothing, `ghostscript` "No
///   pages will be processed", `hayro` "not a PDF". There is no page to lose.
/// - **`bug1020226.pdf` — the file broke it**, and it is 184 bytes of unterminated dictionaries
///   with no `xref` and no `startxref`. All four refuse. The Mozilla bug it is named after is a
///   null dereference in Firefox's worker shutdown, so the file was never a document.
/// - **`poppler-937-0-fuzzed.pdf` — the file broke it twice over**, and the gate's sentence is
///   what says so rather than ADR 0305's, which named only the first. The `/Kids` `[` was
///   fuzzed to a NUL, which §7.2.3 makes white space, so the entry is the bare reference
///   `3 0 R` where §7.7.3.2's Table 30 requires "[a]n array of indirect references" — and
///   object 3 does not parse either, its `/MediaBox` array closing with a SEMICOLON, so what
///   the entry resolves to is §7.3.10's null object. Nothing else declares Table 31's
///   `/Type /Page`: object 3's own `/Type` has its second byte fuzzed to 0xEC. All four
///   references refuse.
/// - **`poppler-85140-0.pdf` — the file broke it, and one reference of four draws it anyway.**
///   The catalogue and the page tree's root read; `/Kids [3 0 R]` names one child, and the only
///   `3 … obj` header in the file reads `3 18446744073709551616 obj` — a generation number
///   outside any integer representation §7.3.3 permits a reader to have ("[t]he range and
///   precision of numbers may be limited by the internal representations used in the computer
///   on which the PDF processor is running"), and far past the 32 bits Table C.1 advises. So
///   the file defines no object `3 0`, and §7.3.10 settles what the reference then is: "[a]n
///   indirect reference to an undefined object shall not be considered an error by a PDF
///   processor; it shall be treated as a reference to the null object". A null is neither
///   Table 30's node nor Table 31's page. `poppler`, `mupdf` and `ghostscript` all refuse —
///   each naming a different one of the file's faults — and `hayro` draws a 595 × 65535 raster
///   with 1785 black pixels in its bottom three rows, having ignored the generation number and
///   clamped `/MediaBox [0 0 595 2147483647]`. That is evidence about `hayro`, and principle 5
///   runs one way: it is not a clause.
/// - **`Brotli-Prototype-FileA.pdf` — unspecified.** It is a prototype of the `/BrotliDecode`
///   filter the PDF Association is standardising (pdf.js issue #20290), and the references
///   divide over it: `mupdf` 1.28 and `ghostscript` 10.07 both decode
///   `/BrotliDecode` and draw the page **in full** — a 1224 × 792 architectural drawing, looked
///   at rather than counted (trap 1). `poppler` 26.08 refuses, naming the filter, and `hayro`
///   refuses. Every
///   object in the file is Brotli-compressed including its cross-reference stream, so the
///   catalogue this reader reads comes from a scan and its `/Pages 30 0 R` lives in the one
///   `/ObjStm`, which is why the tree ends at the root. **Nothing changes here**, and the
///   reason is principle 5 rather than effort: `§` in this tree means ISO 32000-2, and ISO
///   32000-2 defines no `BrotliDecode` — `doc/md/` holds not one occurrence of the name. Two
///   references agreeing is the evidence that would *raise confidence in a reading*, and there
///   is no reading to raise confidence in until the filter is published. What is owed meanwhile
///   is loudness, and the file is not silent: the rebuild note says one of its object streams
///   could not be read, and the gate now says which clause the page tree stopped at.
///
/// **Named rather than counted** (ADR 1081). The paragraphs above are what opening each of the
/// five answered; a count of five could not say that one of them had been replaced by a sixth
/// file failing for a sixth reason, which is the direction this population moves in when it moves
/// at all. [`why_no_page_one`] is what the run prints beside each name, so the clause a file
/// stopped at is on the output rather than only here.
const PAGELESS: [&str; 5] = [
    // 184 bytes of unterminated dictionaries, no `xref` and no `startxref`. All four references
    // refuse; the Mozilla bug it is named after is a null dereference in Firefox's worker
    // shutdown, so the file was never a document.
    "bug1020226.pdf",
    // Linearised and truncated at 871 of the 7945 bytes its own `/L` states; no object past 13 is
    // in the file. All four references refuse. There is no page to lose.
    "REDHAT-1531897-0.pdf",
    // Fuzzed twice over: the `/Kids` `[` became a NUL, which §7.2.3 makes white space, so the
    // entry is a bare reference where Table 30 requires an array — and object 3 does not parse
    // either. Nothing declares Table 31's `/Type /Page`. All four references refuse.
    "poppler-937-0-fuzzed.pdf",
    // `3 18446744073709551616 obj` is a generation number outside any representation §7.3.3
    // permits, so the file defines no object `3 0` and §7.3.10 makes the reference the null
    // object. Three references refuse; `hayro` draws a 595 × 65535 raster, which is evidence
    // about `hayro`.
    "poppler-85140-0.pdf",
    // Every object including the cross-reference stream is `/BrotliDecode`, which ISO 32000-2
    // does not define — `doc/md/` holds not one occurrence of the name — so the catalogue comes
    // from a scan and its `/Pages` lives in the one object stream this reader cannot inflate.
    // `mupdf` and `ghostscript` draw it in full; nothing is owed until the filter is published.
    "Brotli-Prototype-FileA.pdf",
];

/// Documents whose first page interprets with something reported as unsupported.
///
/// **Not a defect count** — it is the honest-reporting requirement working, which is why the
/// bound is a ratchet rather than a zero.
///
/// # The composition is printed by the run
///
/// A breakdown kept by hand in this comment drifts — one figure in the text, a second summed from
/// its rows and a third in the ratchet below — which is the shape `CLAUDE.md`'s rule about
/// derived facts is written against, met by a promise instead of by an instrument.
///
/// [`whose_defect`] is the instrument, and [`print_the_composition`] is what the run prints:
/// every report placed under a mechanism and a class, the classes summing to the population, and
/// a report the table cannot place stopping the gate rather than rounding to nothing. Read that
/// output; do not write a table here again. ADR 0730.
///
/// # What a change to the population means
///
/// It falls as features land, and a document that joins without a new *report* is one that used
/// to draw and no longer does. A document that joins *with* one is the design rather than a
/// regression — trap 5: a silence ending, on a page that was being drawn wrong without a word — so
/// the list admits it, beside the clause it rests on. Why each move happened is argued in the ADR
/// of the change that made it, never in this comment, and the argument for every earlier move is
/// kept whole, as a record, in ADR 1415's appendix.
///
/// # Named, and each name read against its clause
///
/// **Held by name rather than by count** (ADR 1401, which amends ADR 1081's consequence that left
/// this one a count). The composition above prints the partition, and printing is not asserting:
/// a document that stops reporting while another starts, or one that moves from the file's column
/// to this reader's, leaves the count where it was and the gate green. `gate_ratchet::population`
/// fails on either direction and names the file, so every entry below is a document somebody has
/// opened, and a change to the list is a change with its reason beside the name.
///
/// The groups follow [`whose_defect`]'s partition — the mechanism that decides each document is
/// the most-owed one it carries — and the sentence beside a group is the clause it rests on. Of the
/// fifty-nine, none is this reader's — `freetext_no_appearance.pdf`, the last, draws since ADRs
/// 1413 and 1414 set its Arabic joined and right to left in a face from the machine; four are no
/// route the standard states; fifty-five are the file's (ADR 1401, ADR 1411).
const INCOMPLETE: [&str; 59] = [
    // ── The file's: §9.7.5.2, "[t]he Identity-H and Identity-V CMaps shall not be used with a
    // non-embedded font". Every one is a `CIDFontType2` naming a system face (Arial, Calibri,
    // Times New Roman …) whose CIDs are that face's glyph indices, which §9.7.4.2 says are "not
    // meaningful to refer to … in an external font program", and none states a `/ToUnicode`
    // with a mapping in it to reach a substitute by character instead (ADR 0433).
    "issue15441.pdf",
    "issue6127.pdf",
    "issue15443.pdf",
    "issue15594_reduced.pdf",
    "issue15977_reduced.pdf",
    "issue7835.pdf",
    "issue20453.pdf",
    "issue4722.pdf",
    // Its `/ToUnicode` is Adobe's Identity-H CMap itself — `begincidrange` lines, which §9.10.3
    // does not admit into a `/ToUnicode` CMap — so it maps no code to a character.
    "issue5801.pdf",
    "issue11242_reduced.pdf",
    "issue11578_reduced.pdf",
    // `/ToUnicode /Identity-H`, a name where §9.10.1 requires a stream.
    "issue11915.pdf",
    "issue12418_reduced.pdf",
    "bug1365930.pdf",
    "issue19550.pdf",
    "issue19695.pdf",
    "issue13916.pdf",
    "ThuluthFeatures.pdf",
    // ── The file's: §7.8.2 and §7.2.3 — a token that is neither an operand nor an operator the
    // standard defines, or an operator given fewer operands than its table states. Every
    // operator ISO 32000-2 defines is implemented, so each of these is the file's bytes.
    // A form whose stream was mangled on purpose (pdf.js's "Form XObject with errors"): `12.9f`,
    // `c02` and their neighbours are numbers run into operators.
    "issue6342.pdf",
    // A form ending `Q W`, a clip with no path (§8.5.4); its image's JPEG frame also contradicts
    // the dictionary (§7.4.8).
    "issue6413.pdf",
    "issue17554.pdf",
    "operator-in-TJ-array.pdf",
    "poppler-90-0-fuzzed.pdf",
    // `ETBT` is one token under §7.2.3, since nothing delimits the two keywords.
    "sci-notation.pdf",
    "issue9252.pdf",
    "issue2391-1.pdf",
    "issue5039.pdf",
    // RC4 under `/V 4` with a `/Length` of 40 and 48 bits and no `/CF`, which Table 20 requires
    // when `/V` is 4: `/U` authenticates under the short key the file declares, and the streams
    // then fail to inflate, as `poppler` also reports. pdf.js issue #19484 says Acrobat pads the
    // key to 16 bytes, and padding it opens both files; that is not done, because the standard
    // defines nothing for the `/StdCF` they name and the file states its key twice in
    // disagreement — the shape of ADR 0052's byte-swapped `indexToLocFormat`, whose argument is
    // written before its code.
    "issue19484_1.pdf",
    "issue19484_2.pdf",
    "bug1953099.pdf",
    // `BT` then `Q` and the stream ends: a special graphics state operator inside a text object,
    // which §9.4.1 does not admit, and no `ET`.
    "issue14165.pdf",
    // ── The file's: Table 31's required, inheritable `/MediaBox` is nowhere in the ancestry, or
    // encloses no area (§7.7.3.4, §7.9.5).
    "issue15590.pdf",
    "boundingBox_invalid.pdf",
    // And §7.7.3.4 again for its `/F1`: the one `/Resources` the page inherits is empty.
    "issue9105_other.pdf",
    // ── The file's: a resource name §7.8.3's current resource dictionary does not define.
    // A tiling pattern's own `/Resources` omits the page's `/R41`; §7.8.3 inherits only into a
    // form or Type 3 font that states no `/Resources`, and this pattern states one.
    "issue6541.pdf",
    // `/Meta6` resolves to a dictionary stored in an object stream, which §7.5.7 forbids a
    // stream to be, so it is not the form Table 93 describes.
    "issue8702.pdf",
    // The page states its own `/Resources` with no `/Font`; §7.7.3.4 stops the search at the
    // first `/Resources` found and uses it "in its entirety", so the parent's `/F1` is not it.
    "issue5954.pdf",
    // ── The file's: an embedded font program whose filter reports damage (§7.4.1, ADR 0836).
    "issue11651.pdf",
    "bug1050040.pdf",
    "issue13316_reduced.pdf",
    // `/DescendantFonts [ null ]` (§9.7.6.1).
    "issue12823.pdf",
    // ── The file's: an image the dictionary and the data do not agree on.
    // A 1×1 JPEG frame under a 200×100 dictionary (§7.4.8).
    "xobject-image.pdf",
    // `/Mask` is a one-bit `DeviceGray` image with no `/ImageMask`, which §8.9.6.3 does not admit.
    "issue6621.pdf",
    // `/DCTDecode` over the four bytes `1234`.
    "issue18042.pdf",
    // `/Width /Height`, a name where Table 87 requires an integer.
    "issue4575.pdf",
    "jbig2_file_header.pdf",
    // ── The file's: an annotation its own clause cannot draw.
    // No `/Subtype` (Table 166).
    "issue7446.pdf",
    // The check box's on appearance has no `/BBox`, which Table 93 requires and §12.5.5 needs.
    "checkbox-bad-appearance.pdf",
    "checkbox_no_appearance.pdf",
    // `/NeedAppearances true` and a `/DA` naming `/F1` in a form with no `/DR` (§12.7.4.3).
    "issue19389.pdf",
    // The same `/DR` fault, and a `/Contents` whose flate data will not inflate.
    "poppler-395-0-fuzzed.pdf",
    // ── The file's: `/Contents` parts under `/JBIG2Decode`, which §7.4.7 defines as decoding
    // monochrome image data, not a content stream.
    "bomb_giant.pdf",
    // A page object that stops part-way, read as far as §7.3.7 states it (ADR 0784).
    "poppler-742-0-fuzzed.pdf",
    // ── The file's: an embedded program whose own statement is what leaves the text undrawn
    // (ADR 1411). Every code reaches `space`, a composite whose components include itself, so no
    // depth the TrueType Reference Manual's `maxp` can state describes it (Table 124); shown in
    // render mode 7 and nothing is painted after, so the page loses a clip nothing uses.
    "recursiveCompositGlyf.pdf",
    // `/Flags 36`, Symbolic and Nonsymbolic both set, which §9.8.2 says "shall not both be set";
    // read as Symbolic, §9.6.5.4 ignores the `/Encoding` and the (3, 0) subtable reaches an empty
    // `G` where the `/Differences` named `/Ccedilla` (`oracle.rs` has it).
    "issue20232.pdf",
    // ── Neither one: §9.6.5.4's last sentence, "a PDF processor may supply a mapping of its
    // choosing", over a code no route maps. Code 0 through `MacRomanEncoding`, which names
    // nothing there, into a (1, 0) subtable holding code 165 alone; this reader's choice is no
    // glyph (ADR 0270, ADR 0520; `silent_fonts.rs` is its argument).
    "issue17333.pdf",
    // ── Neither one: a chain of nested content streams that re-enters itself, which §9.6.4's
    // Errata Collection 3 paragraph makes implementation-dependent for a glyph and Table C.1
    // leaves to the processor for a form. Each was read and is a cycle whatever state it
    // inherits — the form or glyph re-entered selects its own paint — so no value of
    // `MAX_FORM_DEPTH` finishes it, and the report names the stream re-entered (ADR 1411).
    // Form 7 → form 9 → pattern 11's cell → form 7.
    "operator_list_cycle.pdf",
    // `/X1` → `/X0` → `/X1`, each at half the scale.
    "issue19800.pdf",
    // Glyph `a` of `/FType3A` → glyph `c` of `/FType3B` → pattern `/P1`'s cell → `/FType3A`.
    "ContentStreamCycleType3insideType3.pdf",
];

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
    /// Every code shown on a page one that reached no glyph **without the page reporting**.
    ///
    /// A measurement rather than a gate — nothing here fails on it. `doc/todo/21` asks whether
    /// ADR 0152's trade still holds: the tree reports a font that drew *nothing* and stays quiet
    /// about one that drew most of its codes, because naming every uncovered code named 13
    /// documents that mostly draw fine and each report costs the oracle a judged page (trap 11).
    /// The input to that question is a count, and this is where it is counted.
    codes_without_a_glyph: Vec<(String, usize)>,
    /// Every code shown on a page one that reached a glyph its font program describes as
    /// **empty**, on the same silent pages.
    ///
    /// The other half of the branch above, separated because only one of the two is a mark the
    /// reader loses: a glyph the program
    /// contains and draws as nothing is a space, however its `/ToUnicode` reads it back.
    /// ADR 0270.
    codes_reaching_a_blank_glyph: Vec<(String, usize)>,
    /// Every code shown on a page one whose *vertical form* the substituted face did not have.
    ///
    /// The fourth silence, and the newest (ADR 0764). §9.7.5.1's NOTE makes a vertical `CMap`'s
    /// CID a choice of **shape** — "different shapes are used when writing horizontally and
    /// vertically" — and §9.7.4.2 leaves a substitute reachable only by character, so a face
    /// with no `vert` or `vrt2` feature draws the character standing up. Unlike the three above
    /// it is a mark *made*, in the producer's place, in the wrong shape; ADR 0763 declined to
    /// report it on ADR 0152's arithmetic and left it counted by nothing, which is what this
    /// line ends. **A zero here is a statement about `doc/pdf.js` and about the faces this
    /// machine has**, and `examples/vertical_form_census` is what asks a wider population.
    codes_without_a_vertical_form: Vec<(String, usize)>,
    /// Every code shown on a page one that §9.10.2 could not name, on pages that report nothing.
    ///
    /// The reading half of the two above, and the population `doc/todo/21` §5 is about: the
    /// clause's own "there is no way to determine what the character code represents", counted
    /// so that a page which draws its text and hands back none of it says so. Same rule as the
    /// two above — a page that already reports is not silent — but a much larger number, because
    /// a font that cannot name a code usually draws it perfectly well.
    codes_without_a_character: Vec<(String, usize)>,
    /// Every document whose page one hands a backend a path that begins with a *segment*.
    ///
    /// ISO 32000-2 §8.5.2.1: "the first one invoked shall be m or re to begin a new subpath",
    /// and a segment issued with no current point is the clause's own error case. Such a path
    /// has no first endpoint, so what it draws is whichever library the display list reaches:
    /// `tiny-skia` injects a move to the **origin of user space** and draws an edge from the
    /// corner of the page, `kurbo` fires a `debug_assert!`, and no two backends need agree.
    /// That is trap 2 — a decision either backend can make alone is a decision neither has made
    /// — so the shape is refused where the path is built and this is the gate over real
    /// documents that says so. ADR 0563.
    open_subpaths: Vec<(String, usize)>,
    unopenable: Vec<String>,
    locked: Vec<String>,
    unreadable_encryption: Vec<String>,
    /// Every document that opens and yields no page one, with the reason in the standard's
    /// own terms.
    ///
    /// A reason beside each name rather than [`Tally::unopenable`]'s one word, because a file
    /// with no catalogue, a file whose page tree names an object the file does not define, and a
    /// file whose page tree lives inside a filter this reader does not have are three different
    /// facts. [`why_no_page_one`] is what says which, and the gate prints it.
    pageless: Vec<(String, String)>,
    /// Every document whose page one reports something, with the reports themselves.
    ///
    /// **Held as the values rather than as their `Debug` string**, because [`whose_defect`]
    /// classifies them and a classification derived from a formatted string is one that decays
    /// without saying so.
    incomplete: Vec<(String, Vec<Unsupported>)>,
    slow: Vec<(String, Duration)>,
}

/// Whose defect a report names, which is the question `incomplete` was never asked.
///
/// The count that heads this gate's summary is a *population*, and for most of this file's life
/// its composition lived in the doc comment above [`INCOMPLETE`] as a hand-kept table. By the
/// time it was deleted it stated one figure in its opening sentence, a second in the sum of its
/// own rows and a third in the ratchet below it, and none of the three was what the gate printed
/// — which is exactly what `CLAUDE.md`'s rule about derived facts predicts. So the composition is
/// computed here and printed by the run instead.
///
/// Three classes, and the boundary between them is *who has to do something*:
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Whose {
    /// The report names something the file states that a clause forbids, or omits that a clause
    /// requires. Nothing is owed but the report itself.
    TheFile,
    /// Neither one: the standard defines no route from what the file gives to what would be
    /// drawn, or this program's own bound was reached and said so, or a model this tree departs
    /// from where the clause lets the two differ. Closed by a reading or by a decision.
    NeitherOne,
    /// The report names a clause this tree does not carry out. Work owed, and this is the row a
    /// round takes from.
    ThisReader,
}

impl Whose {
    /// The word the summary prints.
    const fn word(self) -> &'static str {
        match self {
            Self::TheFile => "the file",
            Self::NeitherOne => "neither one",
            Self::ThisReader => "this reader",
        }
    }
}

/// What one report is, at the grain a round can act on, or `None` where this table cannot say.
///
/// **A report this function cannot place fails the gate**, and that is the price of a
/// classification that does not decay: a new [`Unsupported`] variant is a compile error here
/// because the `match` is exhaustive, and a reworded message inside one of the three variants
/// whose payload is prose falls into the unplaced bucket the run asserts is empty. The
/// alternative — an `other` row — is the shape that let the old table drift.
///
/// Three variants carry a string flattened out of a lower layer's typed error, so the mechanism
/// inside them is read back out of that string: [`Unsupported::Font`], [`Unsupported::Image`] and
/// [`Unsupported::Annotation`]. Everywhere else the variant *is* the mechanism, because each
/// one's own doc comment states one condition and one clause. [`Unsupported::Content`] needs no
/// marker at all: its payload is [`ContentIssue`], which is already typed.
fn whose_defect(report: &Unsupported) -> Option<(Whose, &'static str)> {
    Some(match report {
        // **A consequence rather than a mechanism**, and classed for the case where it stands
        // alone: §9.3.1 gives a text object no initial font — "they shall be specified explicitly
        // by using Tf before any text is shown" — so a show with none set is the file's. Where it
        // stands beside a font refusal, that refusal is the mechanism and this is the count of
        // what the refusal cost; the class it is given here cannot decide such a document,
        // because `TheFile` is the least of the three and the partition takes the greatest. The
        // gate asserts it never stands alone in this corpus.
        Unsupported::Text { .. } => (Whose::TheFile, "a show operator this page could not draw"),

        Unsupported::Font { detail } => return a_font_refusal(detail),
        Unsupported::Image { name } => return an_image_refusal(name),
        Unsupported::Annotation { detail } => return an_annotation_refusal(detail),
        Unsupported::Content { issue } => match issue {
            ContentIssue::TooLarge { .. } | ContentIssue::TokenTooLong { .. } => (
                Whose::NeitherOne,
                "a bound this program set, reached on /Contents",
            ),
            ContentIssue::Undecodable { .. } => (
                Whose::TheFile,
                "a /Contents part whose filter chain could not be applied",
            ),
            ContentIssue::Damaged { .. } => (
                Whose::TheFile,
                "a /Contents part that decoded only as far as its damage (§7.4.1)",
            ),
            ContentIssue::NotAStream { .. } | ContentIssue::Unreachable { .. } => {
                (Whose::TheFile, "a /Contents entry Table 31 does not admit")
            }
            _ => return None,
        },

        // Each of these is one condition and one clause, stated by the variant's own doc comment.
        Unsupported::Shading { .. } => (Whose::ThisReader, "a shading or pattern used as paint"),
        Unsupported::ShadingBackground { .. } => (
            Whose::ThisReader,
            "Table 77's /Background, unpainted around the shading",
        ),
        // **The mechanism's wording is the claim, and it is checkable.** This variant's own doc
        // comment says "[a]n operator this interpreter does not implement", which would be this
        // reader's — so the class rests on every operator the standard defines being implemented,
        // which was read off the population rather than assumed: what the corpus reports here is
        // byte soup out of a fuzzed stream, a keyword a file ran into its neighbour, or one of
        // this interpreter's own sentences about a token §7.3.6, §7.8.2 or §8.5.4 does not admit
        // where it stands. A round that leaves a *defined* operator unimplemented owes this arm a second
        // row, and the wording above is what makes that visible rather than silent.
        Unsupported::Operator { .. } => (
            Whose::TheFile,
            "a token §7.8.2 admits neither as an operand nor as an operator",
        ),
        Unsupported::LimitReached { .. } | Unsupported::SpotColourantsWithoutAPlane { .. } => {
            (Whose::NeitherOne, "a bound this program set")
        }
        // §9.6.4's Errata Collection 3 paragraph makes a Type 3 glyph that "refers to itself
        // directly or indirectly" implementation-dependent, and Table C.1 leaves nested
        // `XObject`s to the processor: a chain that re-enters itself has no end the standard
        // states and none the file broke a clause to write (ADR 1411).
        Unsupported::NestingCycle { .. } => (
            Whose::NeitherOne,
            "a chain of nested content streams that re-enters itself (§9.6.4, Table C.1)",
        ),
        Unsupported::SeparationGivenUp { .. } => (
            Whose::ThisReader,
            "§10.8.3's separation given up, the page simulated per painting operation",
        ),
        Unsupported::TextKnockout { .. }
        | Unsupported::CompositedInParts { .. }
        | Unsupported::TransparencyGroup { .. }
        | Unsupported::SoftMask { .. }
        | Unsupported::BlackGeneration { .. }
        | Unsupported::Overprint { .. } => (
            Whose::NeitherOne,
            "a transparency model this tree departs from where the two can differ",
        ),
        Unsupported::MissingResource { .. } => (
            Whose::TheFile,
            "a name §7.8.3's resource dictionary does not define",
        ),
        Unsupported::DamagedContentStream { .. } => (
            Whose::TheFile,
            "one of §7.8.2's other content streams, drawn as far as its damage",
        ),
        Unsupported::OptionalContent { .. } => (
            Whose::NeitherOne,
            "a visibility expression nested past this program's bound",
        ),
        Unsupported::MediaBox { .. } => (
            Whose::TheFile,
            "no usable /MediaBox anywhere in the page's ancestry (§7.7.3.4)",
        ),
        // The file's, and unambiguously: the page object's own bytes stop before its dictionary
        // closes, so what is drawn is the entries §7.3.7 states readably and no more (ADR 0784).
        Unsupported::PageDictionary { .. } => (
            Whose::TheFile,
            "a page dictionary that stops part-way, read as far as it states (§7.3.7)",
        ),
        Unsupported::NoninvertibleMatrix { .. } => (
            Whose::TheFile,
            "marks stated under a matrix with no inverse (§8.3.4)",
        ),
        Unsupported::UndefinedCurrentPoint { .. } => (
            Whose::TheFile,
            "a path segment issued with no current point (§8.5.2.1)",
        ),
        Unsupported::OperandShortfall { .. } => (
            Whose::TheFile,
            "an operator given fewer operands than its table states (§7.8.2)",
        ),
        // The file's, in all four of its shapes. Three of them are a `/Ref` that names a target
        // document or page the host's files do not hold — §8.10.4.1's own "unavailable", said out
        // loud because the reader *was* given files — and the fourth is §14.4's "a different
        // version of the correct PDF file has been found", which is the file identifying itself.
        // Nothing here is owed by this reader: it drew the proxy, which is the clause's answer.
        Unsupported::ReferenceXObject { .. } => (
            Whose::TheFile,
            "a reference XObject whose target the supplied files do not hold (§8.10.4)",
        ),
    })
}

/// [`whose_defect`]'s [`Unsupported::Font`] arm, which is the largest of the three prose ones.
///
/// Every marker below is a phrase one raise site in `pdf-font` or `pdf-model` writes, and none of
/// them is a catch-all — the note on the §9.7.4.2 row says what a catch-all cost.
fn a_font_refusal(detail: &str) -> Option<(Whose, &'static str)> {
    let has = |marker: &str| detail.contains(marker);
    Some(match detail {
        // ADR 0433's population, now said by the refusal itself rather than read off the ink
        // sweep by hand: §9.7.5.2's "shall not be used with a non-embedded font".
        _ if has("§9.7.5.2 says shall not be used") => (
            Whose::TheFile,
            "an Identity CMap over a font the file did not embed (§9.7.5.2)",
        ),
        _ if has("no readable /CIDSystemInfo") => (
            Whose::TheFile,
            "a CIDFont with no /CIDSystemInfo, which Table 115 requires",
        ),
        _ if has("glyph order of a program nobody supplied") => (
            Whose::NeitherOne,
            "an Identity character collection, whose CIDs no table can name (§9.7.3)",
        ),
        _ if has("carries no CID-to-Unicode table") => (
            Whose::ThisReader,
            "a character collection beyond the four §9.7.5.2 requires",
        ),
        _ if has("no /Font resource named") => (
            Whose::TheFile,
            "a /Font name §7.8.3's resource dictionary does not define",
        ),
        // ADR 0808: a CID-keyed CFF some of whose Font DICTs cannot be read draws the glyphs
        // under them against an empty Private DICT, and the page names the codes it shows that
        // reach a glyph the empty DICT cannot draw.
        _ if has("Font DICTs its CID-keyed CFF selects cannot be read") => (
            Whose::TheFile,
            "a CID-keyed CFF whose Font DICTs cannot all be read, drawn against an empty Private DICT (ADR 0808)",
        ),
        // **Three rows the row below used to swallow, and the swallowing was measurable.**
        // `FontError::Malformed`'s message says a program "could not be parsed", and four of its
        // raise sites were font *dictionary* faults with no program read at all — so
        // `issue12823.pdf`, whose `/DescendantFonts` is `[ null ]`, was counted under the row
        // below and made its population 4 where its clause's is 3. `FontError::MalformedDictionary`
        // says which table instead, and these are its three mechanisms.
        _ if has("/DescendantFonts selects no CIDFont dictionary") => (
            Whose::TheFile,
            "a Type 0 font whose /DescendantFonts selects no CIDFont (§9.7.6.1)",
        ),
        _ if has("/CIDToGIDMap is neither a name nor a stream")
            || has("/CIDToGIDMap stream could not be decoded") =>
        {
            (
                Whose::TheFile,
                "a /CIDToGIDMap outside the stream or name Table 115 types it as (§9.7.4.2)",
            )
        }
        _ if has("/Encoding CMap stream could not be decoded") => (
            Whose::TheFile,
            "an /Encoding CMap stream that could not be decoded (§9.7.5.3)",
        ),
        _ if has("could not be parsed") || has("Type 3 glyph for code") => (
            Whose::TheFile,
            "an embedded font program that would not parse",
        ),
        // ADR 1411: the same report where the program or the descriptor states the reason — a
        // composite glyph that includes itself, which Table 124's TrueType Reference Manual gives
        // no depth for, and §9.8.2's two flags both set, which "shall not both be set".
        _ if has("whose components include itself") => (
            Whose::TheFile,
            "a TrueType composite glyph that includes itself (Table 124)",
        ),
        _ if has("sets both the Symbolic and the Nonsymbolic flag") => (
            Whose::TheFile,
            "a font descriptor setting both flags §9.8.2 says shall not both be set",
        ),
        // ADR 0270: an embedded subset that contains no glyph for the codes its own document
        // shows is traced to the end of every route the standard states.
        _ if has("has no outline for any of the") || has("draws none of the") => (
            Whose::NeitherOne,
            "a font with no glyph for any code the page shows through it",
        ),
        _ if has("uses unsupported program type") => (
            Whose::ThisReader,
            "an embedded font program in a format this crate does not read",
        ),
        // **Narrow on purpose, and it was not.** The first draft of this row matched the whole
        // of "cannot be substituted", which swallowed the §9.7.5.2 case above — so breaking
        // that marker deliberately, which is trap 13's calibration, left the gate green with
        // eighteen documents silently reclassified. A marker table with a catch-all in it is
        // not a table.
        _ if has("§9.7.4.2 leaves to reach a substitute") => (
            Whose::ThisReader,
            "no face this machine offers can be addressed by character (§9.7.4.2)",
        ),
        _ if has("uses unsupported encoding") => (
            Whose::ThisReader,
            "an encoding or CMap this crate does not implement",
        ),
        _ if has("Table 57's /Font") => (
            Whose::TheFile,
            "Table 57's /Font stated as something other than a reference and a size",
        ),
        _ => return None,
    })
}

/// [`whose_defect`]'s [`Unsupported::Image`] arm.
///
/// The payload is `ImageError`'s `Display` or one of `pdf_model::image`'s own sentences, so the
/// mechanism is read back out of the prose.
fn an_image_refusal(name: &str) -> Option<(Whose, &'static str)> {
    let has = |marker: &str| name.contains(marker);
    Some(match name {
        _ if has("malformed image") || has("an alternate image dictionary states no /Image") => (
            Whose::TheFile,
            "an image whose samples or dictionary are malformed",
        ),
        // ADR 0340: §7.4.8 puts the dimensions in the encoded data, so the codestream's grid
        // is drawn and the dictionary's disagreement is said out loud.
        _ if has("the JPEG frame is") => (
            Whose::TheFile,
            "a JPEG frame that contradicts its own image dictionary (§7.4.8)",
        ),
        _ if has("is not an image mask") => (
            Whose::TheFile,
            "a /Mask outside what Table 87 defines the entry to hold",
        ),
        // §7.4.7 closes the set of segments an embedded JBIG2 stream may carry — "[t]he JBIG2
        // file header, end-of-page segments, and end-of-file segment shall not be present" — so a
        // segment header the decoder calls unknown or reserved is one ISO/IEC 14492 does not
        // define, and the file is what states it. `jbig2_file_header.pdf` is named for its own
        // defect: the file header it must not carry is read as a segment, because that is what a
        // decoder handed the embedded organisation has to assume it is looking at.
        _ if has("unknown or reserved segment type") => (
            Whose::TheFile,
            "a JBIG2 segment ISO/IEC 14492 does not define (§7.4.7)",
        ),
        _ if has("JBIG2") || has("JPX") || has("JPEG 2000") => {
            (Whose::ThisReader, "an image codec refusal")
        }
        _ => return None,
    })
}

/// [`whose_defect`]'s [`Unsupported::Annotation`] arm.
///
/// The detail is built out of the subtype and what was wrong with it, so the subtype is the part
/// of it these markers deliberately do not read: what decides the class is the *fault*.
fn an_annotation_refusal(detail: &str) -> Option<(Whose, &'static str)> {
    let has = |marker: &str| detail.contains(marker);
    Some(match detail {
        // `doc/todo/22`: the value's script has no code in the standard font that stood in.
        _ if has("states no code for") => (
            Whose::ThisReader,
            "a /DA font stood in for that cannot draw the value's script",
        ),
        _ if has("/DR does not define") => (
            Whose::TheFile,
            "a /DA naming a font the form's /DR does not define (§12.7.4.3)",
        ),
        _ if has("no appearance stream") => (
            Whose::TheFile,
            "an annotation with no /AP and nothing its clause can build one from",
        ),
        _ if has("appearance stream") => (
            Whose::TheFile,
            "an appearance stream the file states and this reader cannot use",
        ),
        _ => return None,
    })
}

/// Prints what the incomplete population is made of, and hands back what it could not place.
///
/// Two counts per line, and they are different questions: how many *documents* carry the
/// mechanism at all, and how many carry it and nothing that is owed more. The second is a
/// partition — every incomplete document is in exactly one of its cells — so the classes' second
/// column sums to the population and the first does not.
fn print_the_composition(incomplete: &[(String, Vec<Unsupported>)]) {
    use std::collections::BTreeMap;

    let mut unplaced = Vec::new();
    let mut carrying: BTreeMap<(Whose, &'static str), usize> = BTreeMap::new();
    let mut deciding: BTreeMap<(Whose, &'static str), usize> = BTreeMap::new();

    for (name, reports) in incomplete {
        let mut placed = Vec::new();
        for report in reports {
            match whose_defect(report) {
                Some(mechanism) => placed.push(mechanism),
                None => unplaced.push((name.clone(), format!("{report:?}"))),
            }
        }
        placed.sort_unstable();
        placed.dedup();
        for mechanism in &placed {
            let seen: &mut usize = carrying.entry(*mechanism).or_default();
            *seen = seen.saturating_add(1);
        }
        // The most-owed mechanism decides the document, so the partition never understates what
        // the population owes: `Whose` orders `ThisReader` last and `max` takes it.
        if let Some(worst) = placed.iter().max() {
            let seen: &mut usize = deciding.entry(*worst).or_default();
            *seen = seen.saturating_add(1);
        }
    }

    println!(
        "  incomplete by whose defect it is, over {} documents:",
        incomplete.len()
    );
    for whose in [Whose::TheFile, Whose::NeitherOne, Whose::ThisReader] {
        let carried: usize = carrying
            .iter()
            .filter(|((class, _), _)| *class == whose)
            .map(|(_, count)| *count)
            .sum();
        let decided: usize = deciding
            .iter()
            .filter(|((class, _), _)| *class == whose)
            .map(|(_, count)| *count)
            .sum();
        println!(
            "    {}: {decided} documents owe nothing more than this, {carried} carry one \
             (mechanism totals below)",
            whose.word()
        );
        let mut rows: Vec<_> = carrying
            .iter()
            .filter(|((class, _), _)| *class == whose)
            .collect();
        rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.1.cmp(b.0.1)));
        for ((_, mechanism), count) in rows {
            println!("      {count:4}  {mechanism}");
        }
    }

    // See [`whose_defect`]: an `other` row is what let the old composition table drift, so a
    // report the classification cannot place stops the run instead of being counted as nothing.
    assert!(
        unplaced.is_empty(),
        "these reports have no row in `whose_defect`, so the composition printed above is \
         incomplete — give each one a mechanism and a class: {unplaced:?}"
    );
    // Trap 11 from the other side. `Unsupported::Text` says a show operator drew nothing and
    // never says why; the report that does stands beside it. A page where it stands alone is a
    // font refused in silence, which is the failure `pdf-model`'s whole reporting rule exists to
    // prevent — and no corpus document has ever been one.
    let mute: Vec<&String> = incomplete
        .iter()
        .filter(|(_, reports)| {
            reports.len() == 1 && matches!(reports.first(), Some(Unsupported::Text { .. }))
        })
        .map(|(name, _)| name)
        .collect();
    assert!(
        mute.is_empty(),
        "a page that skipped text and said nothing about which font it could not use: {mute:?}"
    );
}

/// The four populations a page can lose *without reporting*, printed together.
///
/// Together because reading them side by side is what makes each of them mean anything: the same
/// code can be a mark missed, a mark the font meant not to make, a mark made in a shape its
/// producer did not choose, or a mark made that nothing can name. Not one of the four is a gate —
/// `doc/todo/21` is the standing question they are the input to.
fn the_silences(tally: &Tally) {
    silence(
        "codes reaching no glyph *in silence*",
        "measurement, not a gate; doc/todo/21",
        &tally.codes_without_a_glyph,
    );
    silence(
        "codes reaching a glyph the font draws blank",
        "not a mark missed; ADR 0270",
        &tally.codes_reaching_a_blank_glyph,
    );
    silence(
        "codes drawn upright where §9.7.5.1 named a vertical form",
        "a mark made in the wrong shape; ADR 0764",
        &tally.codes_without_a_vertical_form,
    );
    silence(
        "codes §9.10.2 could not name *in silence*",
        "a readback missed, not a mark; doc/todo/21 §5, ADR 0311",
        &tally.codes_without_a_character,
    );
}

/// Prints one of the four populations a page can lose *without reporting*, worst ten first.
///
/// One function for all four because they are the same measurement of different things — a
/// total, a document count, and the documents that carry most of it — and because reading them
/// side by side is the whole point: a code can be a mark missed, a mark the font meant not to
/// make, a mark made in a shape the producer did not choose, or a mark made that nothing can
/// name, and only the last of those leaves the picture right.
fn silence(what: &str, caveat: &str, counted: &[(String, usize)]) {
    let total: usize = counted.iter().map(|(_, count)| *count).sum();
    println!(
        "  {what}: {total} over {} documents ({caveat})",
        counted.len()
    );
    let mut worst = counted.to_vec();
    worst.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (name, count) in worst.iter().take(10) {
        println!("    {count:6} {name}");
    }
}

/// How many paths in a display list — marks and clips alike — begin with something other than a
/// move.
///
/// Groups are walked because a group's elements are commands of the same kind, and every clip
/// chain a command references because ISO 32000-2 §8.5.4 builds a clip out of the very path
/// §8.5.2 constructed.
fn paths_beginning_with_a_segment(list: &pdf_render::DisplayList) -> usize {
    fn headless(path: &pdf_render::Path) -> bool {
        matches!(
            path.commands().first(),
            Some(pdf_render::PathCommand::LineTo(_) | pdf_render::PathCommand::CurveTo(..))
        )
    }
    fn walk(
        list: &pdf_render::DisplayList,
        commands: &[pdf_render::Command],
        seen: &mut std::collections::HashSet<usize>,
        found: &mut usize,
    ) {
        for command in commands {
            match command {
                pdf_render::Command::Fill { path, .. }
                | pdf_render::Command::Stroke { path, .. } => {
                    *found = found.saturating_add(usize::from(headless(path)));
                }
                pdf_render::Command::Group { commands, .. } => walk(list, commands, seen, found),
                _ => {}
            }
            // The chain rather than the command's own clip: a clip is a child of the one in
            // force when `W` ran, and every link is a path the interpreter built.
            let mut next = command.clip();
            while let Some(id) = next {
                if !seen.insert(id.index()) {
                    break;
                }
                let Some(clip) = list.clip(id) else { break };
                *found = found.saturating_add(usize::from(headless(&clip.path)));
                next = clip.parent;
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut found = 0;
    walk(list, list.commands(), &mut seen, &mut found);
    found
}

/// Calibrates [`paths_beginning_with_a_segment`] against the shape it is a sweep for.
///
/// Trap 13: a sweep run only over a population that turns out to be clean has measured nothing.
/// The gate above prints no document today, and this is what makes that a fact about the corpus
/// rather than about the helper — a list built by hand with the shape in it, in each of the three
/// places a path reaches a backend from. `pdf-model/tests/path_construction.rs` is the other half:
/// there the *interpreter* is asked to build one, and refuses.
#[test]
fn the_open_subpath_sweep_names_a_path_that_begins_with_a_segment() {
    use pdf_render::{
        BlendMode, Clip, Color, Command, DisplayList, FillRule, Paint, PathCommand, Point, Size,
        Transform,
    };

    let headless = || {
        let mut path = pdf_render::Path::new();
        path.push(PathCommand::LineTo(Point::new(10.0, 10.0)));
        path
    };
    let mut list = DisplayList::new(Size::new(100.0, 100.0));
    assert_eq!(paths_beginning_with_a_segment(&list), 0, "an empty list");

    let clip = list
        .add_clip(Clip {
            path: headless(),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .expect("the list holds one clip");
    let fill = |clip| Command::Fill {
        path: std::sync::Arc::new(headless()),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip,
        mask: None,
        blend: BlendMode::Normal,
    };
    list.push(fill(Some(clip)));
    list.push(Command::Group {
        commands: vec![fill(None)],
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: true,
        knockout: false,
        alpha_is_shape: false,
        blending: None,
    });
    assert_eq!(
        paths_beginning_with_a_segment(&list),
        3,
        "a mark, a mark inside a group, and the clip the first one references"
    );
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

/// Why a document that opened has no page one, in the standard's own terms.
///
/// The gate printed one word — `unusable` — over this population *and* over the documents
/// that do not open at all, so five files with five different faults read as one fact. A
/// refusal's wording is a measurement, and a word shared by two populations measures neither.
///
/// The questions are asked in the order the standard makes them: the catalogue, then Table
/// 28's `/Pages`, then Table 30's `/Kids`, then §7.3.10's meaning of a reference into nothing,
/// and last whether the scan §C.4 licenses found a page the tree did not. Each answer names
/// the clause it comes from, and none of them consults another renderer.
///
/// **It reports and it does not judge.** Nothing here decides whether the document *should*
/// have had a page — [`PAGELESS`] is where that is held, and its doc comment is where each
/// of these files is argued.
///
/// Every one of its six answers is planted and named in
/// [`the_page_tree_diagnosis_names_each_clause_it_can_stop_at`], which is trap 13 and which
/// corrected three readings of this reader's own recoveries while it was being written: a
/// trailer that loses `/Root` over an object declaring `/Type /Catalog` is recovered rather
/// than refused, a `/Kids` that is one reference to a page object is recovered by the scan, and
/// §7.7.3.3 makes a child declaring no node a *page* — so a plain dictionary under `/Kids` is
/// page one and never reaches the last answer.
fn why_no_page_one(document: &Document) -> String {
    // §7.5.5's Table 15 makes `/Root` "( Required; shall be an indirect reference ) The catalog
    // dictionary for the PDF file", so a trailer that yields no dictionary has lost the file's
    // one entrance and nothing below can be asked.
    let catalog = match document.catalog() {
        Ok(catalog) => catalog,
        Err(error) => {
            return format!("§7.5.5: the trailer yields no catalogue — {error}");
        }
    };

    // §7.7.2's Table 28: "( Required; shall be an indirect reference ) The page tree node that
    // shall be the root of the document's page tree".
    let root = document.get_key(&catalog, "Pages");
    let Some(node) = root.as_dict() else {
        return format!(
            "§7.7.2: the catalogue's /Pages resolves to {}, where Table 28 puts the root of \
             the page tree{}",
            root.type_name(),
            unreadable_object_streams(document)
        );
    };

    // §7.7.3.2's Table 30: "( Required ) An array of indirect references to the immediate
    // children of this node. The children shall only be page objects or other page tree nodes."
    let kids = document.get_key(node, "Kids");
    let Some(children) = kids.as_array() else {
        return format!(
            "§7.7.3.2: the page tree's root states a /Kids that resolves to {}, where Table 30 \
             requires an array{}",
            kids.type_name(),
            unreadable_object_streams(document)
        );
    };
    if children.is_empty() {
        return "§7.7.3.2: the page tree's root states an empty /Kids, which Errata Collection \
                3's Issue #271 gives a floor of one entry — a file with no pages"
            .to_owned();
    }

    // §7.3.10 decides what a child that is in no cross-reference entry means: "An indirect
    // reference to an undefined object shall not be considered an error by a PDF processor; it
    // shall be treated as a reference to the null object." A null is neither Table 30's node
    // nor Table 31's page, so a root whose every child is one has no descendants.
    let undefined: Vec<String> = children
        .iter()
        .filter(|kid| document.resolve(kid).as_dict().is_none())
        .map(|kid| match kid.as_reference() {
            Some(id) => format!("{} {} R", id.number, id.generation),
            None => document.resolve(kid).type_name().to_owned(),
        })
        .collect();
    if undefined.len() == children.len() {
        return format!(
            "§7.3.10: the page tree's root names {} child(ren) — {} — and the file defines \
             none of them, so every one is the null object and the root has no descendants",
            children.len(),
            undefined.join(", ")
        );
    }

    // The tree yielded something at every step above and still produced no page, which leaves
    // the walk itself: a child that is a node with no children of its own, a depth or node
    // budget, or a cycle. Nothing narrower can be said from here without walking the tree a
    // second time, and this sentence's job is to say *which* of the questions above was the
    // last one answered — so it names the walk and the scan that did not rescue it. §7.7.3.3
    // is why a child that merely resolves is not one of these: a kid declaring no node is a
    // page object, so a plain dictionary under `/Kids` reaches `get(0)` rather than this line.
    format!(
        "§7.7.3.2: the page tree's root has {} child(ren) that resolve and the walk from it \
         reaches no page — a node with no children of its own, a budget, or a cycle; the scan \
         §C.4 licenses found no object declaring Table 31's /Type /Page either{}",
        children.len(),
        unreadable_object_streams(document)
    )
}

/// What §7.5.7's storage cost this document, where it cost it anything.
///
/// A clause appended to the sentences above rather than a sentence of its own, because it is
/// never the whole answer: an object stream this reader could not read is *why* the entry above
/// resolved to nothing, and the two halves belong in one line or a reader has to join them.
fn unreadable_object_streams(document: &Document) -> String {
    let recovered = document.compressed_objects_recovered();
    let lost = recovered
        .unreadable
        .saturating_add(recovered.beyond_the_budget);
    if lost == 0 {
        return String::new();
    }
    format!(
        " — and {lost} of this file's {} object stream(s) (§7.5.7) could not be read, so what \
         they hold is not here",
        recovered.streams
    )
}

/// Calibrates [`why_no_page_one`] against every answer it can give.
///
/// Trap 13, and the same shape as `the_open_subpath_sweep_names_a_path_that_begins_with_a_segment`
/// above: the gate prints five sentences over five real documents, and four of the six branches
/// below are the ones those five reach. A classifier read only over the population it happens to
/// have is a classifier nobody has seen answer, so each answer is planted here and named. The
/// documents are fragments, which is trap 8's caution and is why they are *only* the calibration —
/// what says the classifier is right about a file is the run over the corpus, whose five sentences
/// were each read against the file's own bytes before this calibration was written.
#[test]
fn the_page_tree_diagnosis_names_each_clause_it_can_stop_at() {
    /// A document with the objects given and a trailer, and no cross-reference table — which
    /// `Document::open` rebuilds by scanning, the way `poppler-85140-0.pdf` is read.
    fn assemble(objects: &[(u32, &str)], trailer: &str) -> Document {
        let mut bytes = b"%PDF-1.7\n".to_vec();
        for (number, body) in objects {
            bytes.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
        }
        bytes.extend_from_slice(format!("trailer\n<< {trailer} >>\n%%EOF\n").as_bytes());
        Document::open(bytes).expect("the fragment opens")
    }

    let cases = [
        // §7.5.5: Table 15 requires `/Root`, and a trailer without one names no catalogue —
        // nor does anything else here, which the first draft of this case got wrong. A file
        // whose trailer has lost its `/Root` but still holds an object declaring
        // `/Type /Catalog` is *recovered* rather than refused (ADR 0305's neighbour in
        // `Document::open`), so planting this answer means planting a file with no catalogue
        // object either — which is what `bug1020226.pdf` is.
        (assemble(&[(1, "<< /Colours 4 >>")], "/Size 2"), "§7.5.5:"),
        // §7.7.2: Table 28's `/Pages` is "[t]he page tree node that shall be the root of the
        // document's page tree", and a reference into nothing is not one.
        (
            assemble(&[(1, "<< /Type /Catalog /Pages 99 0 R >>")], "/Root 1 0 R"),
            "§7.7.2:",
        ),
        // §7.7.3.2: Table 30 requires `/Kids` to be an array, which one reference is not — and
        // object 3 may not declare Table 31's `/Type /Page` either, or the scan §C.4 licenses
        // recovers it and the document has a page after all. That is not a hole in the case;
        // it is `poppler-937-0-fuzzed.pdf`, whose object 3 has its `/Type` fuzzed as well.
        (
            assemble(
                &[
                    (1, "<< /Type /Catalog /Pages 2 0 R >>"),
                    (2, "<< /Type /Pages /Kids 3 0 R /Count 1 >>"),
                    (3, "<< /Parent 2 0 R /Colours 4 >>"),
                ],
                "/Root 1 0 R",
            ),
            "Table 30 requires an array",
        ),
        // §7.7.3.2 again, through Errata Collection 3's Issue #271: an array of no children.
        (
            assemble(
                &[
                    (1, "<< /Type /Catalog /Pages 2 0 R >>"),
                    (2, "<< /Type /Pages /Kids [] /Count 0 >>"),
                ],
                "/Root 1 0 R",
            ),
            "empty /Kids",
        ),
        // §7.3.10: every child names an object the file does not define, so every child is null.
        (
            assemble(
                &[
                    (1, "<< /Type /Catalog /Pages 2 0 R >>"),
                    (2, "<< /Type /Pages /Kids [99 0 R] /Count 1 >>"),
                ],
                "/Root 1 0 R",
            ),
            "§7.3.10:",
        ),
        // The children resolve and the walk still reaches no page: here a child that is
        // itself a node with no children of its own. This is the branch no corpus document
        // reaches today, and planting it is what says the sentence exists. **A child that
        // resolves to a plain dictionary does not reach it** — §7.7.3.3 makes a kid that
        // declares no node a page object, so `<< /Colours 4 >>` under a `/Kids` *is* page one
        // (ADR 0305), which is the first thing this case was written as and the reason it is
        // written as this instead.
        (
            assemble(
                &[
                    (1, "<< /Type /Catalog /Pages 2 0 R >>"),
                    (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                    (3, "<< /Type /Pages /Parent 2 0 R /Kids [] /Count 0 >>"),
                ],
                "/Root 1 0 R",
            ),
            "the walk from it reaches no page",
        ),
    ];

    for (document, expected) in &cases {
        assert!(
            pdf_model::Pages::new(document).get(0).is_none(),
            "the fragment for {expected:?} should have no page one"
        );
        let said = why_no_page_one(document);
        assert!(
            said.contains(expected),
            "expected {expected:?} in the diagnosis, got {said:?}"
        );
    }
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
    // ISO 32000-2 §7.6.4.1 has a reader try the default user password and then prompt, and the
    // published password `corpus_passwords` holds is this gate's answer to the prompt.
    let opened = match (
        Document::open(bytes.clone()),
        corpus_passwords::corpus_password(&name),
    ) {
        (Err(SyntaxError::PasswordRequired), Some(known)) => {
            Document::open_with_password(bytes, Limits::default(), known.password)
        }
        (opened, _) => opened,
    };
    let document = match opened {
        Ok(document) => document,
        // A file that refuses both is *locked*, not unreadable, and the distinction is the
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
        let reason = why_no_page_one(&document);
        record(tally, |t| t.pageless.push((name, reason)));
        return;
    };

    let interpretation = pdf_model::interpret(&document, &page);
    // Counted only where the page reports nothing, because that is the population the
    // question is about: a document whose font already says "no outline for any of the codes
    // this page shows" is not silent about it, and is on the incomplete list below.
    if interpretation.codes_without_a_glyph > 0 && interpretation.is_complete() {
        let missed = interpretation.codes_without_a_glyph;
        let named = name.clone();
        record(tally, |t| t.codes_without_a_glyph.push((named, missed)));
    }
    if interpretation.codes_reaching_a_blank_glyph > 0 && interpretation.is_complete() {
        let blank = interpretation.codes_reaching_a_blank_glyph;
        let named = name.clone();
        record(tally, |t| {
            t.codes_reaching_a_blank_glyph.push((named, blank));
        });
    }
    if interpretation.codes_without_a_vertical_form > 0 && interpretation.is_complete() {
        let upright = interpretation.codes_without_a_vertical_form;
        let named = name.clone();
        record(tally, |t| {
            t.codes_without_a_vertical_form.push((named, upright));
        });
    }
    if interpretation.codes_without_a_character.total() > 0 && interpretation.is_complete() {
        let unnamed = interpretation.codes_without_a_character.total();
        let named = name.clone();
        record(tally, |t| {
            t.codes_without_a_character.push((named, unnamed));
        });
    }
    if !interpretation.is_complete() {
        let reported = interpretation.unsupported.clone();
        record(tally, |t| t.incomplete.push((name.clone(), reported)));
    }

    // ISO 32000-2 §8.5.2.1's shape, asked of the finished list rather than of the operators:
    // whatever route built a path — a content stream, a glyph outline, an annotation's
    // appearance — none of them may hand a backend geometry with no first point. See
    // `Tally::open_subpaths`.
    let headless = paths_beginning_with_a_segment(&interpretation.display_list);
    if headless > 0 {
        let named = name.clone();
        record(tally, |t| t.open_subpaths.push((named, headless)));
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

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). A build without
/// it draws every other image and none of those three, so what follows would be a measurement of
/// the build rather than of the tree — which is exactly what moved the accessibility census's
/// ratchet by nine elements while four rounds read the difference as something else
/// (ADR 0557, trap 16).
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
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
    the_silences(&tally);
    print_the_composition(&tally.incomplete);
    for (name, reported) in &tally.incomplete {
        println!("  incomplete: {name}: {reported:?}");
    }
    for (name, headless) in &tally.open_subpaths {
        println!("  path with no first point: {name}: {headless}");
    }
    for name in &tally.locked {
        println!("  locked: {name}");
    }
    for name in &tally.unreadable_encryption {
        println!("  encryption we do not implement: {name}");
    }
    for name in &tally.unopenable {
        println!("  unopenable: {name}");
    }
    for (name, reason) in &tally.pageless {
        println!("  no page one: {name}: {reason}");
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
    // Each bound printed beside the population it bounds, because this file is where the defect
    // that rule exists for was found twice: a ceiling thirty above its population admits thirty
    // documents' worth of regression and reads exactly like one that cannot (ADR 1075).
    gate_ratchet::ceiling(
        "documents that cannot be opened",
        tally.unopenable.len(),
        MAX_UNOPENABLE,
    );
    // Four of the five are held by **name** rather than by count: a ceiling says ten and ten again
    // when one document has left and another arrived, and both halves of that swap are findings
    // (ADRs 1081 and 1401). The count still prints, beside the length of the list, so the table
    // these lines print is one table. The fifth has no members to name.
    gate_ratchet::population(
        "documents that need a password",
        tally.locked.iter().cloned(),
        &LOCKED,
    );
    gate_ratchet::population(
        "documents encrypted in a way this reader does not implement",
        tally.unreadable_encryption.iter().cloned(),
        &UNREADABLE_ENCRYPTION,
    );
    gate_ratchet::population(
        "documents with no reachable first page",
        tally.pageless.iter().map(|(name, _)| name.clone()),
        &PAGELESS,
    );
    gate_ratchet::population(
        "documents that draw incompletely",
        tally.incomplete.iter().map(|(name, _)| name.clone()),
        &INCOMPLETE,
    );
    assert!(
        tally.open_subpaths.is_empty(),
        "ISO 32000-2 §8.5.2.1: a path handed to a backend must begin with a move, or the \
         library it reaches chooses where the first point is: {:?}",
        tally.open_subpaths
    );
}

/// Calibrates the unplaced bucket against the shape it exists for.
///
/// Trap 13: the gate above asserts that every report was placed, and an assertion that has never
/// been made to fail is a claim about the classification rather than about the corpus. This shows
/// [`whose_defect`] declining a report it has no row for — the exact shape a reworded message or a
/// new refusal produces — and placing one it does, so the empty bucket in the run above is a fact
/// about the 974 documents.
#[test]
fn the_classification_declines_a_report_it_has_no_row_for() {
    assert_eq!(
        whose_defect(&Unsupported::Font {
            detail: "a sentence no raise site in this tree writes".to_owned(),
        }),
        None,
        "a font refusal outside the marker table has to arrive as unplaced, or the gate's \
         composition is silently short of it"
    );
    assert_eq!(
        whose_defect(&Unsupported::Image {
            name: "Im0: a sentence no raise site in this tree writes".to_owned(),
        }),
        None,
        "the same for an image, whose payload is a lower layer's error flattened to prose"
    );
    assert_eq!(
        whose_defect(&Unsupported::Font {
            detail: "font /F1 cannot be substituted: the file states /Encoding /Identity-H over \
                     a descendant with no embedded program, which §9.7.5.2 says shall not be used"
                .to_owned(),
        })
        .map(|(whose, _)| whose),
        Some(Whose::TheFile),
        "and the message `pdf-font` does write is placed, or the test above proves nothing"
    );
}

/// The three classes are ordered by what they owe, and the partition depends on that order.
///
/// [`print_the_composition`] takes the *greatest* mechanism a document carries so that the
/// partition never understates the population's debt. That is a property of the `Ord` derive,
/// which nothing else in this file would notice the loss of.
#[test]
fn the_partition_counts_a_document_under_the_most_it_owes() {
    assert!(Whose::ThisReader > Whose::NeitherOne);
    assert!(Whose::NeitherOne > Whose::TheFile);
    assert_eq!(
        [Whose::TheFile, Whose::ThisReader, Whose::NeitherOne]
            .into_iter()
            .max(),
        Some(Whose::ThisReader),
    );
}
