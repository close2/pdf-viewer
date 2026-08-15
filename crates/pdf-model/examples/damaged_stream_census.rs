//! How many documents hold a stream that decodes only as far as its damage, and where.
//!
//! The instrument behind ADR 0343. `doc/todo/03` §8 asked whether drawing the successfully
//! decoded prefix of a stream that then fails is a recovery a reader may perform, and named the
//! population to measure it over: the crawl's undecodable `/Contents` parts. The decision needs
//! two numbers that no gate prints, and this counts both.
//!
//! - **What the rule buys**, on page one's `/Contents`: how many documents keep a prefix, how
//!   many bytes, and whether interpreting that prefix produces any drawing command at all. A
//!   recovery that recovers no marks is still worth having as a *report*, and this is what says
//!   which of the two it is on any given file.
//! - **How wide the silence was**, over every stream object in the file: an [`ISO 32000-2
//!   §7.4`](super) filter that stopped short used to hand its prefix back indistinguishably from
//!   a whole decode, so a partial ICC profile, font program or image reached the code that reads
//!   it with nothing said. `/Contents` is the route this round made loud; the rest is the number
//!   that says what is still owed.
//! - **Which consumer reads each damaged stream**, added in the five-hundred-and-twenty-first
//!   session, because `doc/todo/03` §9 left the remaining 96% owed *per consumer*: whether a
//!   prefix of a thing is a smaller thing of the same kind is a question about what the thing is,
//!   and the answer differs for a form `XObject`, an image, a profile and a function. A count per
//!   role is what says which of those arguments is worth a round.
//! - **How many of them the program says anything about**, added in the five-hundred-and-twenty-fourth
//!   session: every page of every document holding a damaged stream is interpreted, and the reports
//!   that name damage are counted. It is the only line here that measures *this tree* rather than
//!   the files — which is why it is printed beside the population and never instead of it — and it
//!   is the before-and-after ADR 0359 is judged on.
//! - **How many streams are shorter than the file's own arithmetic says they are**, damaged or
//!   not. ISO 32000-2 §7.3.8.2 makes the extent of a stream inferable from the object's own
//!   attributes — "streams are used to represent many objects from whose attributes a length can
//!   be inferred. All of these constraints shall be consistent" — and §7.10.2 states it outright
//!   for a sampled function. A stream that falls short of it is one whose consumer is reading
//!   values the producer never wrote, which is a *different* population from the damaged one and
//!   overlaps it.
//!
//! ```sh
//! cargo run --release -p pdf-model --example damaged_stream_census -- <dir-or-file>...
//! ```
//!
//! One process per directory is the method the surveys use and the reason is the same:
//! `render-cpu` rasterises under `panic = "abort"`, so one document's abort would take every
//! other verdict with it. This example does not rasterise, but it does parse hostile input.
#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus four orders of magnitude below what a usize counts, and \
              this is a measurement rather than a shipped path"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::{Damage, Dictionary, Document, Object, ObjectId, Stream};

use pdf_model::function::{Function, FunctionError};
use pdf_model::page::ContentIssue;

/// Which consumer reads a stream, decided from the stream's own dictionary.
///
/// The question `doc/todo/03` §9 leaves owed is *per consumer* — whether the prefix of a damaged
/// stream is a smaller thing of the same kind depends on what the thing is — so a count of
/// damaged streams is only useful broken down this way. Every arm is a distinguishing entry the
/// standard makes required of that object, named beside it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    /// A page's `/Contents`, Table 31 — the one route ADR 0343 made loud.
    PageContents,
    /// A form `XObject`, Table 93's `/Subtype /Form`, which §7.8.2 makes a content stream too.
    FormXObject,
    /// A tiling pattern's content stream, Table 75's `/PatternType 1`.
    TilingPattern,
    /// An image `XObject`, Table 87's `/Subtype /Image`.
    Image,
    /// A font program, Table 126's `/Length1`, `/Length2`, `/Length3` or `/Subtype`.
    FontProgram,
    /// An ICC profile, Table 66's `/N` on the stream an `ICCBased` space names.
    IccProfile,
    /// A function, Table 38's `/FunctionType`.
    Function,
    /// An object stream, §7.5.7's `/Type /ObjStm`.
    ObjectStream,
    /// A cross-reference stream, §7.5.8's `/Type /XRef`.
    CrossReference,
    /// A metadata stream, §14.3.2's `/Type /Metadata`.
    Metadata,
    /// An embedded file, §7.11.4's `/Type /EmbeddedFile`.
    EmbeddedFile,
    /// A shading whose dictionary is a stream, Table 78's `/ShadingType` 4 to 7.
    Shading,
    /// A `CMap`, §9.7.5.3's `/Type /CMap` or the stream a Type 0 font's `/Encoding` names.
    CMap,
    /// A `/ToUnicode` `CMap`, §9.10.3 — a mapping rather than a content stream (ADR 0359).
    ToUnicode,
    /// A Type 3 glyph description, reached through Table 110's `/CharProcs`.
    Type3Glyph,
    /// An annotation appearance, reached through §12.5.5's `/AP`.
    Appearance,
    /// An `Indexed` colour space's lookup table, §8.6.6.3's fourth array element.
    IndexedLookup,
    /// A page's thumbnail image, §12.3.4's `/Thumb`.
    Thumbnail,
    /// JavaScript, §12.6.4.16's `/JS` — outside this program's scope by `CLAUDE.md`'s exclusion.
    JavaScript,
    /// An XFA packet, Annex K's `/XFA` — excluded by `CLAUDE.md` on §K.1's own permission.
    Xfa,
    /// Everything else: a stream no dictionary in the file names under a key this walk reads.
    Other,
}

impl Role {
    /// Every role, in the order the report prints them.
    const ALL: [Self; 21] = [
        Self::PageContents,
        Self::FormXObject,
        Self::TilingPattern,
        Self::Image,
        Self::FontProgram,
        Self::IccProfile,
        Self::Function,
        Self::ObjectStream,
        Self::CrossReference,
        Self::Metadata,
        Self::EmbeddedFile,
        Self::Shading,
        Self::CMap,
        Self::ToUnicode,
        Self::Type3Glyph,
        Self::Appearance,
        Self::IndexedLookup,
        Self::Thumbnail,
        Self::JavaScript,
        Self::Xfa,
        Self::Other,
    ];

    /// What the report calls it.
    const fn label(self) -> &'static str {
        match self {
            Self::PageContents => "a page's /Contents",
            Self::FormXObject => "a form XObject",
            Self::TilingPattern => "a tiling pattern",
            Self::Image => "an image",
            Self::FontProgram => "a font program",
            Self::IccProfile => "an ICC profile",
            Self::Function => "a function",
            Self::ObjectStream => "an object stream",
            Self::CrossReference => "a cross-reference stream",
            Self::Metadata => "a metadata stream",
            Self::EmbeddedFile => "an embedded file",
            Self::Shading => "a shading",
            Self::CMap => "a CMap",
            Self::ToUnicode => "a /ToUnicode CMap",
            Self::Type3Glyph => "a Type 3 glyph description",
            Self::Appearance => "an annotation appearance",
            Self::IndexedLookup => "an Indexed lookup table",
            Self::Thumbnail => "a thumbnail",
            Self::JavaScript => "JavaScript",
            Self::Xfa => "an XFA packet",
            Self::Other => "unclassified",
        }
    }

    /// Its slot in [`Tally::damaged_by_role`].
    const fn index(self) -> usize {
        match self {
            Self::PageContents => 0,
            Self::FormXObject => 1,
            Self::TilingPattern => 2,
            Self::Image => 3,
            Self::FontProgram => 4,
            Self::IccProfile => 5,
            Self::Function => 6,
            Self::ObjectStream => 7,
            Self::CrossReference => 8,
            Self::Metadata => 9,
            Self::EmbeddedFile => 10,
            Self::Shading => 11,
            Self::CMap => 12,
            Self::ToUnicode => 13,
            Self::Type3Glyph => 14,
            Self::Appearance => 15,
            Self::IndexedLookup => 16,
            Self::Thumbnail => 17,
            Self::JavaScript => 18,
            Self::Xfa => 19,
            Self::Other => 20,
        }
    }

    /// Reads the role off the stream's own dictionary, or off the entry that names it.
    ///
    /// `named` is what [`who_names_what`] found: the roles a *referring* dictionary states, which
    /// is the only statement there is for a stream whose own dictionary carries nothing but Table
    /// 5's entries. A page's `/Contents` is the oldest member of that set and the rest were the
    /// five-hundred-and-thirty-first session's, which split `unclassified` because `doc/todo/03`
    /// §11 said the largest silent bucket was not one thing.
    fn of(document: &Document, stream: &Stream, number: u32, named: &BTreeMap<u32, Self>) -> Self {
        let dict = &stream.dict;
        let name_is = |key: &str, value: &[u8]| {
            document
                .get_key(dict, key)
                .as_name()
                .is_some_and(|name| name.as_bytes() == value)
        };
        // What a page or another dictionary says this stream is outranks what the stream's own
        // dictionary suggests, because the naming entry is the standard's own statement of the
        // role: Table 31's `/Contents`, Table 110's `/CharProcs`, §12.5.5's `/AP`.
        if let Some(role) = named.get(&number) {
            return *role;
        }
        if name_is("Type", b"ObjStm") {
            return Self::ObjectStream;
        }
        if name_is("Type", b"XRef") {
            return Self::CrossReference;
        }
        if name_is("Type", b"Metadata") {
            return Self::Metadata;
        }
        if name_is("Type", b"EmbeddedFile") {
            return Self::EmbeddedFile;
        }
        // §9.7.5.3 makes an embedded CMap a stream with `/Type /CMap`, and Table 78 makes
        // `/ShadingType` required of every shading — types 4 to 7 of which are streams.
        if name_is("Type", b"CMap") {
            return Self::CMap;
        }
        if dict.get("ShadingType").is_some() {
            return Self::Shading;
        }
        if name_is("Subtype", b"Image") {
            return Self::Image;
        }
        if name_is("Subtype", b"Form") {
            return Self::FormXObject;
        }
        if dict.get("PatternType").is_some() {
            return Self::TilingPattern;
        }
        if dict.get("FunctionType").is_some() {
            return Self::Function;
        }
        // Table 126 gives a font program stream its own length entries, and PDF 2.0's
        // `/Subtype` names the three that are not Type 1.
        if dict.get("Length1").is_some()
            || dict.get("Length2").is_some()
            || dict.get("Length3").is_some()
            || name_is("Subtype", b"Type1C")
            || name_is("Subtype", b"CIDFontType0C")
            || name_is("Subtype", b"OpenType")
        {
            return Self::FontProgram;
        }
        // Table 66 makes `/N` required of the stream an `ICCBased` space names, and no other
        // stream this walk classifies carries it — `/Type /ObjStm`'s `/N` is taken above.
        if dict.get("N").is_some() {
            return Self::IccProfile;
        }
        Self::Other
    }
}

/// What one population came to.
#[derive(Default)]
struct Tally {
    /// Files looked at.
    files: usize,
    /// Files [`Document::open`] accepted.
    opened: usize,
    /// Documents whose page-one `/Contents` reported [`ContentIssue::Damaged`].
    contents_damaged: usize,
    /// Of those, the ones whose damage is [`Damage::Truncated`].
    contents_truncated: usize,
    /// Of those, the ones whose damage is [`Damage::Corrupt`].
    contents_corrupt: usize,
    /// Documents whose page-one `/Contents` reported [`ContentIssue::Undecodable`] — nothing
    /// survived at all, which is the case the prefix rule cannot help.
    contents_undecodable: usize,
    /// Bytes kept across every damaged `/Contents` part.
    contents_kept: usize,
    /// Damaged `/Contents` documents whose kept prefix yields at least one drawing command.
    contents_drew: usize,
    /// Stream objects examined anywhere in a document.
    streams: usize,
    /// Those that decoded only as far as their damage.
    streams_damaged: usize,
    /// Documents holding at least one such stream.
    documents_with_damage: usize,
    /// Damaged streams by the consumer that reads them, indexed by [`Role::index`].
    damaged_by_role: [usize; Role::ALL.len()],
    /// Reports naming damage, over every page of every document holding a damaged stream.
    ///
    /// Distinct reports rather than damaged streams: `Interpreter::note` keys by the report, so
    /// one form drawn a hundred times is one of these. What it measures is how much of the
    /// population above the program is willing to say out loud.
    damage_reports: usize,
    /// Of those, the ones ADR 0359 added — §7.8.2's four kinds that are not a page's `/Contents`.
    ///
    /// Split out rather than left in the total because the total on its own cannot be compared
    /// against a tree that does not have them: subtract this and what remains is what ADR 0343's
    /// route alone said, which is the before to this line's after.
    other_content_stream_reports: usize,
    /// Documents where at least one of those was made.
    documents_reporting_damage: usize,
    /// Objects an object stream names and whose bytes its decoded prefix does not carry.
    ///
    /// The shipped answer rather than a second copy of the rule (trap 8):
    /// [`Document::objects_lost_to_damage`] is what refuses them, and this reads it after every
    /// object in the file has been asked for, which is what expands every object stream.
    objects_lost: usize,
    /// Documents holding at least one of those.
    documents_losing_objects: usize,
    /// Cross-reference entries a stream's own `/Index` and `/W` state and its data does not carry.
    xref_entries_lost: u64,
    /// Documents holding at least one of those.
    documents_losing_xref_entries: usize,
    /// Image `XObject`s whose decoded samples fall short of `/Width` × `/Height`, §7.3.8.2.
    short_images: usize,
    /// Documents holding at least one of those.
    documents_with_short_images: usize,
    /// Sampled functions whose data falls short of the sample array, §7.10.2.
    short_functions: usize,
    /// Documents holding at least one of those.
    documents_with_short_functions: usize,
    /// The documents worth naming, with what they said.
    witnesses: Vec<String>,
}

impl Tally {
    fn absorb(&mut self, other: Self) {
        self.files += other.files;
        self.opened += other.opened;
        self.contents_damaged += other.contents_damaged;
        self.contents_truncated += other.contents_truncated;
        self.contents_corrupt += other.contents_corrupt;
        self.contents_undecodable += other.contents_undecodable;
        self.contents_kept += other.contents_kept;
        self.contents_drew += other.contents_drew;
        self.streams += other.streams;
        self.streams_damaged += other.streams_damaged;
        self.documents_with_damage += other.documents_with_damage;
        for (slot, count) in self.damaged_by_role.iter_mut().zip(other.damaged_by_role) {
            *slot += count;
        }
        self.damage_reports += other.damage_reports;
        self.other_content_stream_reports += other.other_content_stream_reports;
        self.documents_reporting_damage += other.documents_reporting_damage;
        self.objects_lost += other.objects_lost;
        self.documents_losing_objects += other.documents_losing_objects;
        self.xref_entries_lost += other.xref_entries_lost;
        self.documents_losing_xref_entries += other.documents_losing_xref_entries;
        self.short_images += other.short_images;
        self.documents_with_short_images += other.documents_with_short_images;
        self.short_functions += other.short_functions;
        self.documents_with_short_functions += other.documents_with_short_functions;
        self.witnesses.extend(other.witnesses);
    }
}

fn main() {
    let roots: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        println!("usage: damaged_stream_census <dir-or-file>...");
        return;
    }

    let mut files = Vec::new();
    for root in &roots {
        collect(root, &mut files);
    }
    files.sort();

    let mut total = Tally::default();
    for path in &files {
        total.absorb(examine(path));
    }

    println!(
        "{} files, {} opened: {} page-one /Contents damaged ({} truncated, {} corrupt), \
         {} undecodable",
        total.files,
        total.opened,
        total.contents_damaged,
        total.contents_truncated,
        total.contents_corrupt,
        total.contents_undecodable,
    );
    println!(
        "  the prefix rule keeps {} bytes over those parts, and {} of {} damaged documents \
         draw at least one command from it",
        total.contents_kept, total.contents_drew, total.contents_damaged,
    );
    println!(
        "  over every stream object: {} of {} streams damaged, in {} documents",
        total.streams_damaged, total.streams, total.documents_with_damage,
    );
    for role in Role::ALL {
        let count = total.damaged_by_role[role.index()];
        if count > 0 {
            println!("    {count} damaged: {}", role.label());
        }
    }
    println!(
        "  and this program names damage in {} reports over {} of those documents, {} of them \
         one of §7.8.2's four other content streams (ADR 0359)",
        total.damage_reports, total.documents_reporting_damage, total.other_content_stream_reports,
    );
    println!(
        "  §7.5's two storage structures: {} objects in {} documents were not read from an \
         object stream's damaged prefix, and {} cross-reference entries in {} documents were \
         stated and not carried",
        total.objects_lost,
        total.documents_losing_objects,
        total.xref_entries_lost,
        total.documents_losing_xref_entries,
    );
    println!(
        "  short of their stated extent (§7.3.8.2): {} images in {} documents, \
         {} sampled functions in {} documents",
        total.short_images,
        total.documents_with_short_images,
        total.short_functions,
        total.documents_with_short_functions,
    );
    for witness in &total.witnesses {
        println!("    {witness}");
    }
}

/// Every `.pdf` under `root`, or `root` itself where it is one.
fn collect(root: &Path, into: &mut Vec<PathBuf>) {
    if root.is_file() {
        into.push(root.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|e| e == "pdf") {
            into.push(path);
        }
    }
}

/// What each stream is, according to the dictionary that names it.
///
/// **The split `doc/todo/03` §11 asked for.** A stream whose own dictionary carries nothing but
/// Table 5's entries — a page's `/Contents`, a Type 3 glyph description, an `Indexed` lookup
/// table — can only be classified by the entry some other object names it under, and the
/// standard makes that entry a statement of the role. So this walks every object in the file
/// once and records, for each key the standard gives a stream role to, the number it points at.
///
/// Where two dictionaries name one stream the first wins, which is a real ambiguity rather than
/// an implementation choice — a form drawn both as a `/Do` and as an appearance is both — and it
/// is why the count this feeds is a population rather than a partition of the file.
fn who_names_what(document: &Document, pages: &pdf_model::Pages) -> BTreeMap<u32, Role> {
    let mut named = BTreeMap::new();

    // Table 31's `/Contents`, walked through the page tree rather than through the object table,
    // because a page is what makes a content stream one.
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        match page.dict.get("Contents") {
            Some(object @ Object::Reference(_)) => note(&mut named, object, Role::PageContents),
            Some(Object::Array(items)) => {
                for item in items {
                    note(&mut named, item, Role::PageContents);
                }
            }
            _ => {}
        }
        if let Some(thumb) = page.dict.get("Thumb") {
            note(&mut named, thumb, Role::Thumbnail);
        }
    }

    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        // §8.6.6.3: "[a]n Indexed colour space shall be defined by a four-element array:
        // [/Indexed base hival lookup ]", whose fourth element "may be either a stream or
        // ( PDF 1.2 ) a byte string".
        if let Some(items) = object.as_array()
            && items
                .first()
                .and_then(Object::as_name)
                .is_some_and(|name| name.as_bytes() == b"Indexed" || name.as_bytes() == b"I")
            && let Some(lookup) = items.get(3)
        {
            note(&mut named, lookup, Role::IndexedLookup);
        }
        let Some(dict) = object.as_dict() else {
            continue;
        };
        for (key, role) in [
            // §9.10.3's `/ToUnicode`, a mapping and not a content stream (ADR 0359).
            ("ToUnicode", Role::ToUnicode),
            // Table 122's `/Encoding` on a Type 0 font, which "shall be" a predefined CMap's
            // name or "a stream containing a CMap" (§9.7.5.3).
            ("Encoding", Role::CMap),
            // §9.7.5.2's `/UseCMap`, the other place a CMap stream is named.
            ("UseCMap", Role::CMap),
            // §12.6.4.16's `/JS`, "a text string or text stream containing the JavaScript".
            ("JS", Role::JavaScript),
            // Annex K's `/XFA`, "either a stream … or an array".
            ("XFA", Role::Xfa),
        ] {
            if let Some(value) = dict.get(key) {
                note(&mut named, value, role);
                if key == "XFA"
                    && let Some(items) = value.as_array()
                {
                    for item in items {
                        note(&mut named, item, Role::Xfa);
                    }
                }
            }
        }
        // Table 110's `/CharProcs`, "a dictionary in which each key shall be a glyph name and
        // the value associated with that key shall be a content stream".
        if let Some(procs) = dict.get("CharProcs") {
            let procs = document.resolve(procs);
            if let Some(procs) = procs.as_dict() {
                for (_, value) in procs.iter() {
                    note(&mut named, value, Role::Type3Glyph);
                }
            }
        }
        // §12.5.5's `/AP`, whose `/N`, `/R` and `/D` are each "a stream … or a subdictionary"
        // keyed by appearance state.
        if let Some(appearances) = dict.get("AP") {
            let appearances = document.resolve(appearances);
            if let Some(appearances) = appearances.as_dict() {
                for (_, value) in appearances.iter() {
                    note(&mut named, value, Role::Appearance);
                    let states = document.resolve(value);
                    if let Some(states) = states.as_dict() {
                        for (_, state) in states.iter() {
                            note(&mut named, state, Role::Appearance);
                        }
                    }
                }
            }
        }
    }
    named
}

/// Records what an entry says a stream is, where the entry names one.
///
/// The first statement wins: a stream two dictionaries name is genuinely both, and taking the
/// later one would make the count depend on object order.
fn note(named: &mut BTreeMap<u32, Role>, object: &Object, role: Role) {
    if let Object::Reference(id) = object {
        named.entry(id.number).or_insert(role);
    }
}

/// Every stream object in the file, counted three ways, with the damaged ones handed back.
///
/// The width of the silence rather than the part of it ADR 0343 made loud: the damage each stream
/// carries, the consumer that reads it, and — a different population that overlaps this one —
/// whether the stream falls short of the extent §7.3.8.2 infers from the object itself.
fn every_stream(
    document: &Document,
    pages: &pdf_model::Pages,
    name: &str,
    tally: &mut Tally,
) -> Vec<(u32, Damage, usize, Role)> {
    let named = who_names_what(document, pages);

    let mut damaged = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(stream) = object.as_stream() else {
            continue;
        };
        tally.streams += 1;
        if let Ok(decoded) = document.decoded_stream_data_reported(stream)
            && let Some(damage) = decoded.damage
        {
            tally.streams_damaged += 1;
            let role = Role::of(document, stream, number, &named);
            tally.damaged_by_role[role.index()] += 1;
            damaged.push((number, damage, decoded.data.len(), role));
        }
        // The shipped statement of §7.3.8.2's arithmetic, asked with no resources: an image
        // whose colour space is a *name* is not counted rather than counted wrongly, since the
        // dictionary that would resolve it belongs to the `Do` and not to the image.
        if let Some(shortfall) =
            pdf_model::image::short_of_its_grid(document, stream, &Dictionary::new())
        {
            tally.short_images += 1;
            if tally.documents_with_short_images == 0 {
                tally.documents_with_short_images = 1;
                tally
                    .witnesses
                    .push(format!("{name}: object {number} {shortfall}"));
            }
        }
        if let Some(shortfall) = short_sampled_function(document, number) {
            tally.short_functions += 1;
            if tally.documents_with_short_functions == 0 {
                tally.documents_with_short_functions = 1;
                tally
                    .witnesses
                    .push(format!("{name}: object {number} {shortfall}"));
            }
        }
    }
    damaged
}

fn examine(path: &Path) -> Tally {
    let mut tally = Tally {
        files: 1,
        ..Tally::default()
    };
    let Ok(bytes) = std::fs::read(path) else {
        return tally;
    };
    let Ok(document) = Document::open(bytes) else {
        return tally;
    };
    tally.opened = 1;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let pages = pdf_model::Pages::new(&document);
    let damaged_here = every_stream(&document, &pages, &name, &mut tally);

    // §7.5's two storage structures, asked *after* the walk above, which has resolved every
    // object number in the file and therefore expanded every object stream reachable from one.
    let lost = document.objects_lost_to_damage();
    if !lost.is_empty() {
        tally.objects_lost = lost.count();
        tally.documents_losing_objects = 1;
        tally.witnesses.push(format!(
            "{name}: {} object(s) not read from object stream(s) {:?} that decoded only in part, \
             {} of them not even named (§7.5.7)",
            lost.count(),
            lost.streams,
            lost.unnamed,
        ));
    }
    let entries_lost = document.cross_reference_entries_lost();
    if entries_lost > 0 {
        tally.xref_entries_lost = entries_lost;
        tally.documents_losing_xref_entries = 1;
        tally.witnesses.push(format!(
            "{name}: {entries_lost} cross-reference entries stated by /Index and /W and not \
             carried by the stream's data (§7.5.8)"
        ));
    }

    if !damaged_here.is_empty() {
        tally.documents_with_damage = 1;
        let (all, others) = reports_naming_damage(&document, &pages);
        tally.damage_reports = all;
        tally.other_content_stream_reports = others;
        tally.documents_reporting_damage = usize::from(all > 0);
    }

    // Page one's `/Contents`, which is where `doc/todo/03` §8's question was asked.
    let Some(page) = pages.get(0) else {
        return tally;
    };
    let (_content, issues) = page.content_with_report(&document);
    let mut damaged_contents = None;
    for issue in &issues {
        match issue {
            ContentIssue::Damaged { damage, kept, .. } => {
                tally.contents_damaged = 1;
                tally.contents_kept += kept;
                match damage {
                    Damage::Truncated => tally.contents_truncated = 1,
                    Damage::Corrupt => tally.contents_corrupt = 1,
                }
                damaged_contents = Some((*damage, *kept));
            }
            ContentIssue::Undecodable { .. } => tally.contents_undecodable = 1,
            _ => {}
        }
    }

    if let Some((damage, kept)) = damaged_contents {
        // Whether the prefix is worth drawing rather than merely worth reporting.
        let commands = pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .len();
        if commands > 0 {
            tally.contents_drew = 1;
        }
        tally.witnesses.push(format!(
            "{name}: /Contents {damage:?}, {kept} bytes kept, {commands} commands"
        ));
    } else if tally.contents_undecodable == 1 {
        tally
            .witnesses
            .push(format!("{name}: /Contents undecodable, nothing survived"));
    } else if !damaged_here.is_empty() {
        let (number, damage, kept, role) = damaged_here[0];
        // Whether the tree is silent about this document is now a question rather than an
        // assumption: ADR 0359 made four of §7.8.2's five content streams loud, so the line has
        // to say which of the two it is or it would go on asserting the older answer.
        let said = if tally.damage_reports > 0 {
            format!("{} report(s) name damage", tally.damage_reports)
        } else {
            "nothing says so".to_owned()
        };
        tally.witnesses.push(format!(
            "{name}: object {number} {damage:?} in {}, {kept} bytes kept, and {said} \
             ({} damaged streams)",
            role.label(),
            damaged_here.len()
        ));
    }
    tally
}

/// How many reports naming damage this program makes over every page of a document.
///
/// The two it can make: [`ContentIssue::Damaged`] for a part of a page's `/Contents` (ADR 0343)
/// and [`pdf_model::Unsupported::DamagedContentStream`] for the four other content streams §7.8.2
/// names (ADR 0359). **Every page, not page one** — a form `XObject` or an appearance is reached
/// through whatever page draws it, and `comments.pdf`'s damaged form is not on the first.
///
/// Asked only of documents that hold a damaged stream, which is what keeps it affordable: 726 of
/// the crawl's 65 944 do.
fn reports_naming_damage(document: &Document, pages: &pdf_model::Pages) -> (usize, usize) {
    let (mut reports, mut others) = (0, 0);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        for item in &pdf_model::interpret(document, &page).unsupported {
            match item {
                pdf_model::Unsupported::DamagedContentStream { .. } => {
                    reports += 1;
                    others += 1;
                }
                pdf_model::Unsupported::Content {
                    issue: ContentIssue::Damaged { .. },
                } => reports += 1,
                _ => {}
            }
        }
    }
    (reports, others)
}

/// How short a Type 0 function's stream is of its sample array, ISO 32000-2 §7.10.2.
///
/// > The stream data shall be long enough to contain the entire sample array, as indicated by
/// > Size , Range , and BitsPerSample ; see 7.3.8.2, "Stream extent".
///
/// Asked of [`Function::parse`] rather than computed here, for the reason trap 8's last paragraph
/// gives: a census whose predicate is a second copy of the rule measures the copy. The refusal's
/// own wording is the predicate, which is why this is an example and not a gate.
fn short_sampled_function(document: &Document, number: u32) -> Option<String> {
    let object = Object::Reference(ObjectId {
        number,
        generation: 0,
    });
    match Function::parse(document, &object) {
        Err(FunctionError::Malformed { detail }) if detail.contains("sample array needs") => {
            Some(detail)
        }
        _ => None,
    }
}
