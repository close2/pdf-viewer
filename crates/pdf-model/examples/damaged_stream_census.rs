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

use std::collections::BTreeSet;
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
    /// Everything else: a Type 3 glyph description, an `Indexed` palette, a `/JS`, an appearance
    /// this walk did not reach through a page.
    Other,
}

impl Role {
    /// Every role, in the order the report prints them.
    const ALL: [Self; 12] = [
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
            Self::Other => 11,
        }
    }

    /// Reads the role off the stream's own dictionary.
    ///
    /// `contents` is every object a page of this document names in its `/Contents`, which is the
    /// one role a dictionary does not state: a content stream's dictionary carries nothing but
    /// Table 5's entries, so what makes it one is being named by a page.
    fn of(document: &Document, stream: &Stream, number: u32, contents: &BTreeSet<u32>) -> Self {
        let dict = &stream.dict;
        let name_is = |key: &str, value: &[u8]| {
            document
                .get_key(dict, key)
                .as_name()
                .is_some_and(|name| name.as_bytes() == value)
        };
        if contents.contains(&number) {
            return Self::PageContents;
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
    damaged_by_role: [usize; 12],
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
    // Every object a page names in its `/Contents`, which is the one role a stream's own
    // dictionary cannot state: Table 5 is all such a stream carries.
    let mut contents = BTreeSet::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        match page.dict.get("Contents") {
            Some(Object::Reference(id)) => {
                contents.insert(id.number);
            }
            Some(Object::Array(items)) => {
                for item in items {
                    if let Object::Reference(id) = item {
                        contents.insert(id.number);
                    }
                }
            }
            _ => {}
        }
    }

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
            let role = Role::of(document, stream, number, &contents);
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
    if !damaged_here.is_empty() {
        tally.documents_with_damage = 1;
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
        tally.witnesses.push(format!(
            "{name}: object {number} {damage:?} in {}, {kept} bytes kept, and nothing says so \
             ({} damaged streams)",
            role.label(),
            damaged_here.len()
        ));
    }
    tally
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
