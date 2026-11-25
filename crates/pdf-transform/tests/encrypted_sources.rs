//! Every verb that writes streams it read out of another document, from an AES-256 and an RC4
//! source, read back page by page.
//!
//! A [`Document`] holds every stream decrypted (ISO 32000-2 §7.6.2), so what a verb carries is
//! plaintext under a dictionary the source wrote for ciphertext. Three things in that dictionary
//! were the source's encryption and not its content, and each is checked on every route:
//!
//! - §7.3.8.2's `/Length` "indicates how many bytes of the PDF file are used for the stream's
//!   data", which in the source counted ciphertext;
//! - §7.4.10's `/Crypt` stage names an entry of the *source's* `/CF`, which the output does not
//!   hold (ADR 3685);
//! - a stream the source's decryption refused is held empty by the reader, and is refused by name
//!   rather than written empty (ADR 3685).
//!
//! The verbs are listed from `pdf_transform::Plan`: `merge`; `split`, whose selection is also
//! the extraction of a page; `pages`' reordering and rotation, which write through `merge`'s
//! engine; `optimize`, through the serializer and through the linearised writer; and `archive`'s
//! conversion. `update`'s insertion is `tests/update.rs`'s (ADR 3682).

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and the \
              arithmetic is §7.6.3's own — byte counters XORed into a key, a sum of lengths"
)]

use std::fmt::Write as _;
use std::ops::Not as _;

use md5::{Digest, Md5};
use pdf_archive::{Flavour, Target};
use pdf_model::Pages;
use pdf_syntax::serialize::{ObjectStreams, Streams};
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId, Stream};
use pdf_transform::archive::{ArchivePlan, Authorisations, FormAnswers};
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::pages::{Angle, Edit, PagesPlan};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::tool::ToolOutputs;
use pdf_transform::{
    Budget, MemorySinks, Plan, Policy, Protect, Refusal, Report, Secret, Source, apply_protected,
};
use rc4::{KeyInit, StreamCipher};

/// What page one of every fixture shows.
const FIRST: &str = "CARRIED";

/// What page two shows.
const SECOND: &str = "SECOND";

/// §7.6.4.3.2 step (a)'s padding string, verbatim from the clause.
const PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// The first element of the RC4 fixture's file identifier, which Algorithms 2 and 5 hash.
const ID0: [u8; 16] = *b"rc4-fixture-id-0";

/// Table 22's `/P` with every permission granted.
const EVERYTHING: i32 = -4;

/// The operators of a page showing `word` in Helvetica.
fn showing(word: &str) -> Vec<u8> {
    format!("BT /F1 12 Tf 20 100 Td ({word}) Tj ET").into_bytes()
}

/// A classic-table file of these numbered object bodies, `trailer` added to its trailer.
fn assemble(objects: &[Vec<u8>], trailer: &str) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        bytes.extend_from_slice(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R {trailer}>>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    bytes
}

/// A stream object of `data`, `entries` stated before its `/Length`.
fn stream(entries: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {entries}/Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// Two pages showing [`FIRST`] and [`SECOND`]; `encode` turns each content stream's object number
/// and plaintext into the bytes the file holds, and `first_entries` is stated in page one's
/// content stream's dictionary.
fn two_pages(
    first_entries: &str,
    encode: &dyn Fn(u32, &[u8]) -> Vec<u8>,
    trailer: &str,
    more: Vec<Vec<u8>>,
) -> Vec<u8> {
    let page = |contents: u32| {
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 \
             R >> >> /Contents {contents} 0 R >>"
        )
        .into_bytes()
    };
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>".to_vec(),
        page(4),
        stream(first_entries, &encode(4, &showing(FIRST))),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        page(7),
        stream("", &encode(7, &showing(SECOND))),
    ];
    objects.extend(more);
    assemble(&objects, trailer)
}

/// The fixture in the clear.
fn clear() -> Vec<u8> {
    two_pages("", &|_, data| data.to_vec(), "", Vec::new())
}

/// Opens `bytes` with the empty user password.
fn read(bytes: &[u8]) -> Document {
    Document::open(bytes.to_vec()).expect("the file opens")
}

/// [`clear`] written again under §7.6.4's standard security handler at `/V` 5 (AES-256), with the
/// empty user password, so that it opens without one.
fn under_aes() -> Vec<u8> {
    use pdf_syntax::serialize::{Assembly, Form, Options};
    let source = clear();
    let document = read(&source);
    let mut assembly = Assembly::new(vec![&document]);
    for number in 1..=7 {
        assembly
            .copy(0, ObjectId::new(number, 0))
            .expect("the fixture's object copies");
    }
    assembly.set_root(ObjectId::new(1, 0));
    let mut out = Vec::new();
    pdf_syntax::serialize_encrypted(
        &assembly,
        pdf_syntax::Version { major: 1, minor: 7 },
        Options::new(Form::Table),
        &pdf_syntax::Protection::owner_only("source-owner"),
        &mut pdf_syntax::SystemEntropy,
        &mut out,
    )
    .expect("the fixture is encrypted");
    out
}

/// RC4 under `key`.
fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    let mut cipher = rc4::Rc4::new_from_slice(key).expect("RC4 takes 1 to 256 key bytes");
    cipher.apply_keystream(&mut out);
    out
}

/// MD5 over several pieces in order.
fn md5(pieces: &[&[u8]]) -> [u8; 16] {
    let mut hash = Md5::new();
    for piece in pieces {
        hash.update(piece);
    }
    hash.finalize().into()
}

/// §7.6.4.3.2 step (a): a password padded or truncated to 32 bytes.
fn padded(password: &[u8]) -> [u8; 32] {
    let mut out = PADDING;
    let used = password.len().min(32);
    out[..used].copy_from_slice(&password[..used]);
    out[used..].copy_from_slice(&PADDING[..32 - used]);
    out
}

/// The RC4 fixture's file encryption key, `/O` and `/U`: §7.6.4.4.2 Algorithm 3,
/// §7.6.4.3.2 Algorithm 2 and §7.6.4.4.4 Algorithm 5 at revision 4, a sixteen-byte key, the empty
/// user password.
fn rc4_keys() -> ([u8; 16], Vec<u8>, Vec<u8>) {
    let user = padded(b"");
    // Algorithm 3: the owner password's key, MD5 fifty more times (step (c)).
    let mut owner_key = md5(&[&padded(b"source-owner")]);
    for _ in 0..50 {
        owner_key = md5(&[&owner_key]);
    }
    // Steps (e) to (g): the padded user password under that key, nineteen more rounds.
    let mut owner_entry = rc4(&owner_key, &user);
    for counter in 1..=19_u8 {
        let round: Vec<u8> = owner_key.iter().map(|byte| byte ^ counter).collect();
        owner_entry = rc4(&round, &owner_entry);
    }
    // Algorithm 2 steps (a) to (e) and (h).
    let mut key = md5(&[&user, &owner_entry, &EVERYTHING.to_le_bytes(), &ID0]);
    for _ in 0..50 {
        key = md5(&[&key]);
    }
    // Algorithm 5: MD5 of the padding and the identifier, under the key, nineteen more rounds,
    // and sixteen bytes of arbitrary padding (step (f)).
    let mut user_entry = rc4(&key, &md5(&[&PADDING, &ID0]));
    for counter in 1..=19_u8 {
        let round: Vec<u8> = key.iter().map(|byte| byte ^ counter).collect();
        user_entry = rc4(&round, &user_entry);
    }
    user_entry.extend_from_slice(&[0xA5; 16]);
    (key, owner_entry, user_entry)
}

/// Uppercase hexadecimal.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02X}");
        out
    })
}

/// [`clear`] under §7.6.4's standard security handler at `/V` 4 `/R` 4 with an RC4 crypt filter
/// (Table 25's `/CFM /V2`), `first_entries` stated in page one's content stream's dictionary.
fn under_rc4_stating(first_entries: &str) -> Vec<u8> {
    let (key, owner_entry, user_entry) = rc4_keys();
    // §7.6.3.2 Algorithm 1 steps (a) to (d): the object's key, "up to a maximum of 16" bytes.
    let encode = move |number: u32, data: &[u8]| {
        let number = number.to_le_bytes();
        let object_key = md5(&[&key, &number[..3], &[0, 0]]);
        rc4(&object_key, data)
    };
    let encrypt = format!(
        "<< /Filter /Standard /V 4 /R 4 /Length 128 /CF << /StdCF << /CFM /V2 /AuthEvent \
         /DocOpen /Length 16 >> >> /StmF /StdCF /StrF /StdCF /O <{}> /U <{}> /P {EVERYTHING} >>",
        hex(&owner_entry),
        hex(&user_entry)
    );
    let id = hex(&ID0);
    two_pages(
        first_entries,
        &encode,
        &format!("/Encrypt 8 0 R /ID [<{id}> <{id}>] "),
        vec![encrypt.into_bytes()],
    )
}

/// [`clear`] under RC4.
fn under_rc4() -> Vec<u8> {
    under_rc4_stating("")
}

/// `encrypted` with page one's content stream naming its crypt filter itself: §7.4.10's `/Crypt`
/// first in `/Filter`, Table 14's `/Name` the document's own `/StdCF`, written back by an update
/// that encrypts it under that filter (§7.6.6).
fn with_own_crypt_filter(encrypted: &[u8]) -> Vec<u8> {
    let document = read(encrypted);
    let id = ObjectId::new(4, 0);
    let held = document.get(id);
    let held = held.as_stream().expect("page one's content stream");
    let mut dict = held.dict.clone();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Array(vec![Object::Name(Name::new(&b"Crypt"[..]))]),
    );
    let mut parms = Dictionary::new();
    parms.insert(
        Name::new(&b"Name"[..]),
        Object::Name(Name::new(&b"StdCF"[..])),
    );
    dict.insert(
        Name::new(&b"DecodeParms"[..]),
        Object::Array(vec![Object::Dictionary(parms)]),
    );
    let replacement = Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: std::sync::Arc::clone(&held.data),
        decryption_failed: false,
    }));
    let updated = pdf_syntax::write::incremental_update(
        &document,
        &std::collections::BTreeMap::from([(id, replacement)]),
    )
    .expect("the fixture's update is written");
    assert_eq!(
        texts(&updated),
        [FIRST, SECOND],
        "the plant: the stream decrypts through its own /Crypt filter"
    );
    updated
}

/// Each page's text, trimmed.
fn texts(bytes: &[u8]) -> Vec<String> {
    let document = read(bytes);
    let pages = Pages::new(&document);
    (0..pages.len())
        .map(|index| {
            let page = pages.get(index).expect("a page");
            pdf_model::interpret(&document, &page)
                .text
                .trim()
                .to_owned()
        })
        .collect()
}

/// Page `page` (counted from 1) of `bytes`, drawn at 72 dpi as a PPM.
fn draw(bytes: &[u8], page: usize) -> Vec<u8> {
    let sinks = MemorySinks::new();
    run(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: page.to_string().parse::<Selection>().expect("a selection"),
            size: Sizing::Dpi(72.0),
            format: ImageFormat::Ppm,
            page_box: None,
            annotations: true,
            names: "page.ppm".parse().expect("a pattern"),
            strips: None,
        }),
        bytes,
        &sinks,
        None,
    )
    .expect("the page draws");
    sinks.into_outputs().pop().expect("one raster").1
}

/// Runs `plan` over `source`, protected where `protect` says.
fn run(
    plan: &Plan,
    source: &[u8],
    sinks: &MemorySinks,
    protect: Option<&Protect>,
) -> Result<Report, Refusal> {
    let source = Source::new(source.to_vec());
    apply_protected(
        plan,
        &[&source],
        sinks,
        &Policy::default(),
        &Budget::default(),
        protect,
    )
}

/// One verb as these tests drive it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verb {
    /// Both pages, through `merge`.
    Merge,
    /// One file per page, through `split`.
    Split,
    /// Page two alone, through `split`'s selection.
    Extract,
    /// Page two moved in front of page one, through `pages`.
    Reorder,
    /// Page one turned a quarter, through `pages`.
    Rotate,
    /// The serializer's own pass, through `optimize`.
    Optimize,
    /// `optimize`'s linearised writer.
    Linearize,
    /// A conversion to PDF/A-4.
    Archive,
}

impl Verb {
    /// Every verb.
    const ALL: [Self; 8] = [
        Self::Merge,
        Self::Split,
        Self::Extract,
        Self::Reorder,
        Self::Rotate,
        Self::Optimize,
        Self::Linearize,
        Self::Archive,
    ];

    /// Whether the verb writes an encrypted file when asked; `archive` cannot, because ISO
    /// 19005-4 forbids encryption.
    fn can_protect(self) -> bool {
        self != Self::Archive
    }

    /// The plan.
    fn plan(self) -> Plan {
        let selection = |pages: &str| pages.parse::<Selection>().expect("a selection");
        let optimize = |linearize| {
            Plan::Optimize(OptimizePlan {
                source: 0,
                names: "out.pdf".parse().expect("a pattern"),
                prune: true,
                object_streams: ObjectStreams::DEFAULT,
                streams: Streams::DEFAULT,
                linearize,
                images: None,
            })
        };
        let pages = |edit| {
            Plan::Pages(PagesPlan {
                source: 0,
                edits: vec![edit],
                names: "out.pdf".parse().expect("a pattern"),
            })
        };
        let split = |pages: &str, pieces| {
            Plan::Split(SplitPlan {
                source: 0,
                pages: selection(pages),
                pieces,
                names: "piece-%d.pdf".parse().expect("a pattern"),
            })
        };
        match self {
            Self::Merge => Plan::Merge(MergePlan {
                inputs: vec![Input {
                    source: 0,
                    pages: selection("1-2"),
                }],
                collate: false,
                information: Vec::new(),
                names: "out.pdf".parse().expect("a pattern"),
            }),
            Self::Split => split("1-2", Pieces::EachPage),
            Self::Extract => split("2", Pieces::Groups),
            Self::Reorder => pages(Edit::Move {
                pages: selection("2"),
                to: 1,
            }),
            Self::Rotate => pages(Edit::Rotate {
                angle: Angle::Absolute(90),
                pages: selection("1"),
            }),
            Self::Optimize => optimize(false),
            Self::Linearize => optimize(true),
            Self::Archive => Plan::Archive(ArchivePlan {
                source: 0,
                names: "out.pdf".parse().expect("a pattern"),
                target: Target::Four(Flavour::Plain),
                // ISO 19005-4 section 6.1.3 forbids encryption, so the conversion of an encrypted
                // source is a loss somebody has to authorise.
                authorised: Authorisations {
                    encryption: true,
                    ..Authorisations::default()
                },
                profile: None,
                press: None,
                substitute_fonts: true,
                supplied_fonts: std::collections::BTreeMap::new(),
                cjk_pack: None,
                lgc_pack: None,
                departures: Vec::new(),
                claim_conformance: false,
                forms: FormAnswers::default(),
                derivations: Vec::new(),
                supplies: Vec::new(),
                preservations: Vec::new(),
                annotation_flags: None,
                resolutions: Vec::new(),
                tool_outputs: ToolOutputs::new(),
                external_data: std::collections::BTreeMap::new(),
                clock: pdf_transform::archive::Clock::System,
            }),
        }
    }

    /// Each output's pages, as indices (from 1) of the source's pages.
    fn expected(self) -> Vec<Vec<usize>> {
        match self {
            Self::Merge | Self::Rotate | Self::Optimize | Self::Linearize | Self::Archive => {
                vec![vec![1, 2]]
            }
            Self::Split => vec![vec![1], vec![2]],
            Self::Extract => vec![vec![2]],
            Self::Reorder => vec![vec![2, 1]],
        }
    }

    /// Whether each output page is drawn as the source page it came from: every verb but the
    /// rotation and the conversion, which embeds a substitute for Helvetica.
    fn draws_the_same(self) -> bool {
        !matches!(self, Self::Rotate | Self::Archive)
    }
}

/// The protection an encrypted output is written under, opening on the empty user password.
fn protection() -> Protect {
    Protect::owner_only(Secret::from("output-owner".to_owned()))
}

/// Runs `verb` over `source` and checks every output page: it shows what its source page shows,
/// drawn the same where the verb does not change the drawing; every stream's `/Length` counts the
/// bytes the file holds for it; no stream of a clear output names a §7.4.10 crypt filter; and
/// nothing was repaired on reading it back.
fn check(verb: Verb, source: &[u8], protected: bool) {
    let protect = protected.then(protection);
    let sinks = MemorySinks::new();
    let report = run(&verb.plan(), source, &sinks, protect.as_ref())
        .unwrap_or_else(|refusal| panic!("{verb:?}: {refusal}"));
    let mut outputs = sinks.into_outputs();
    outputs.sort_by(|one, other| one.0.cmp(&other.0));
    let expected = verb.expected();
    assert_eq!(
        outputs.len(),
        expected.len(),
        "{verb:?}: the outputs, after {report:?}"
    );
    let words = [FIRST, SECOND];
    for ((name, bytes), pages) in outputs.iter().zip(&expected) {
        let shown: Vec<&str> = pages.iter().map(|page| words[page - 1]).collect();
        assert_eq!(texts(bytes), shown, "{verb:?} {name}: the pages' text");
        let document = read(bytes);
        assert_eq!(document.is_encrypted(), protected, "{verb:?} {name}");
        assert!(
            document.repairs().is_empty(),
            "{verb:?} {name}: read back with repairs {:?}",
            document.repairs()
        );
        if !protected {
            every_stream_is_written_as_stated(verb, name, &document);
        }
        if verb.draws_the_same() {
            for (at, page) in pages.iter().enumerate() {
                assert!(
                    draw(bytes, at + 1) == draw(source, *page),
                    "{verb:?} {name}: page {} is drawn as the source's page {page}",
                    at + 1
                );
            }
        }
    }
}

/// In a clear output, the reader holds each stream's bytes as the file holds them: so §7.3.8.2's
/// `/Length` equals them, and no `/Filter` names `/Crypt`, which Table 14's `/Name` would make
/// name an entry of a `/CF` this file does not have.
fn every_stream_is_written_as_stated(verb: Verb, name: &str, document: &Document) {
    let size = document
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| u32::try_from(size).ok())
        .expect("the trailer states /Size");
    for number in 1..size {
        let id = document.defined_id(number);
        let Object::Stream(stream) = document.get(id) else {
            continue;
        };
        if stream
            .dict
            .get("Type")
            .and_then(Object::as_name)
            .map(Name::as_bytes)
            == Some(b"XRef")
        {
            continue;
        }
        let stated = document.resolve(stream.dict.get("Length").unwrap_or(&Object::Null));
        assert_eq!(
            stated.as_integer(),
            i64::try_from(stream.data.len()).ok(),
            "{verb:?} {name}: object {id:?}'s /Length states the bytes written"
        );
        let filter = document.resolve(stream.dict.get("Filter").unwrap_or(&Object::Null));
        let names_crypt = match &filter {
            Object::Name(name) => name.as_bytes() == b"Crypt",
            Object::Array(stages) => stages.iter().any(|stage| {
                stage
                    .as_name()
                    .is_some_and(|name| name.as_bytes() == b"Crypt")
            }),
            _ => false,
        };
        assert!(!names_crypt, "{verb:?} {name}: object {id:?} names /Crypt");
    }
}

/// ISO 32000-2 §7.3.8.2: "Every stream dictionary shall have a Length entry that indicates how
/// many bytes of the PDF file are used for the stream's data". An AES-256 source's streams are
/// held decrypted (§7.6.2), sixteen bytes of initialisation vector and the padding shorter than
/// its `/Length`; each verb writes them under a `/Length` that counts what it wrote.
#[test]
fn every_verb_carries_an_aes_source_into_a_clear_file() {
    let source = under_aes();
    for verb in Verb::ALL {
        check(verb, &source, false);
    }
}

/// The same from an RC4 source, whose ciphertext is as long as its plaintext: the route on which
/// a `/Length` copied from the source would happen to be right.
#[test]
fn every_verb_carries_an_rc4_source_into_a_clear_file() {
    let source = under_rc4();
    for verb in Verb::ALL {
        check(verb, &source, false);
    }
}

/// Into an encrypted output, which writes every stream under its own key (§7.6.2 on the way out).
#[test]
fn every_verb_that_protects_carries_both_sources_into_an_encrypted_file() {
    for source in [under_aes(), under_rc4()] {
        for verb in Verb::ALL.into_iter().filter(|verb| verb.can_protect()) {
            check(verb, &source, true);
        }
    }
}

/// ISO 32000-2 §7.4.10: the `Crypt` filter "allows the document-level security handler (see 7.6,
/// "Encryption") to determine which algorithms should be used to decrypt the input data", and
/// Table 14's `/Name` "shall correspond to an entry in the CF entry of the encryption
/// dictionary". The data every verb carries is already out of the source's crypt filter, and
/// the output's `/CF` is not the source's, so the stage goes and the output's own §7.6.2 decides
/// how the bytes are written. Kept, it names a filter a clear output does not define, and the
/// page reads back with no content.
#[test]
fn a_stream_s_own_crypt_filter_stays_with_its_source() {
    for source in [
        with_own_crypt_filter(&under_aes()),
        with_own_crypt_filter(&under_rc4()),
    ] {
        for verb in Verb::ALL {
            check(verb, &source, false);
        }
        for verb in Verb::ALL.into_iter().filter(|verb| verb.can_protect()) {
            check(verb, &source, true);
        }
    }
}

/// A stream §7.6's decryption refused is refused by name rather than carried empty.
///
/// Page one's content stream in an RC4 source states a `/Crypt` stage naming `/Secret`, which
/// Table 14's `/Name` "shall correspond to an entry in the CF entry of the encryption
/// dictionary": the source's `/CF` holds only `/StdCF`, so the reader holds the stream empty and
/// marks it, and writing it would replace the page's content with nothing. The same in a source
/// with no encryption dictionary at all.
#[test]
fn a_stream_whose_decryption_failed_is_refused_by_every_verb() {
    let undecryptable = "/Filter [/Crypt] /DecodeParms [<< /Name /Secret >>] ";
    for source in [
        under_rc4_stating(undecryptable),
        two_pages(undecryptable, &|_, data| data.to_vec(), "", Vec::new()),
    ] {
        assert_eq!(
            texts(&source),
            ["", SECOND],
            "the plant: the reader holds page one's stream empty"
        );
        // `Extract` takes page two alone and never reaches the stream.
        for verb in Verb::ALL.into_iter().filter(|verb| *verb != Verb::Extract) {
            let sinks = MemorySinks::new();
            let outcome = run(&verb.plan(), &source, &sinks, None);
            let said = match &outcome {
                Ok(report) => format!("{report:?}"),
                Err(refusal) => refusal.to_string(),
            };
            assert!(
                said.contains("decryption"),
                "{verb:?}: the refusal names the decryption: {said}"
            );
            // A refused write leaves what the sink was handed before the refusal — no
            // cross-reference section and no trailer — so only an output that reads back whole is
            // a file somebody could take for the source's pages.
            for (name, bytes) in sinks.into_outputs() {
                if Document::open(bytes.clone())
                    .is_ok_and(|document| document.repairs().is_empty())
                    .not()
                {
                    continue;
                }
                assert!(
                    !texts(&bytes).iter().any(String::is_empty),
                    "{verb:?} {name}: no page is written empty, after {said}"
                );
            }
        }
    }
}
