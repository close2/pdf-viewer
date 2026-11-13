//! RFC 5280 section 4.2.1.10's name constraints, and section 6.1's processing of them.
//!
//! Section 6.1.2 (b) and (c) start the state from the inputs `initial-permitted-subtrees` and
//! `initial-excluded-subtrees`, section 6.1.4 (g) narrows it by every intermediate's
//! `nameConstraints`, and section 6.1.3 (b) and (c) check each certificate's subject name and
//! subject alternative names against it. [`NameState`] is that state.
//!
//! # Intersection is kept as a list of sets
//!
//! Section 6.1.4 (g)(1) sets `permitted_subtrees` "to the intersection of its previous value and
//! the value indicated in the extension field". An intersection of two sets of subtrees is not in
//! general a set of subtrees one can write down — `*.example.com` with `*.example.org` is empty,
//! and the empty set of a form has no `GeneralSubtree` spelling — so [`NameState`] keeps every
//! permitted set it was given and asks that a name lie in one subtree of *each*. That is the
//! intersection's definition rather than an approximation of it. A set constrains only the forms
//! it names: section 4.2.1.10, "Restrictions apply only when the specified name form is
//! present."
//!
//! # Comparison
//!
//! Section 7.1 makes a distinguished name's attribute values compare after RFC 4518's string
//! preparation with case folding. `prepared` performs that preparation exactly on ASCII text —
//! where case folding is ASCII lowercase and normalisation changes nothing — and refuses to decide
//! between two values that differ beyond it while either holds a character outside ASCII, because
//! RFC 4518's mapping and normalisation tables are not in this tree. An undecided comparison is
//! [`Match::Undecided`], and a check that meets one is refused by name ([`NameRefusal::Undecided`])
//! rather than decided in either direction: a permitted subtree that might not match and an
//! excluded one that might are both a path this program cannot vouch for.
//!
//! # Bounds
//!
//! Every list here comes out of a stranger's certificate: at most [`MAX_SUBTREES`] subtrees are
//! held across a path and at most [`MAX_NAMES`] names are read from one certificate; past either
//! the path is refused as this program's limit.

use crate::der::{Class, Reader, SEQUENCE, SET, Value};

/// The most subtrees, permitted and excluded together, held for one path.
///
/// A CA states a handful; the bound keeps the product with [`MAX_NAMES`] — every name checked
/// against every subtree — small over a stranger's chain.
pub const MAX_SUBTREES: usize = 64;

/// The most names one certificate's subject and `subjectAltName` are read for.
pub const MAX_NAMES: usize = 32;

/// The most relative distinguished names a `Name` is read for, and attributes in one of them.
const MAX_RDNS: usize = 32;

/// PKCS #9's `emailAddress`, `1.2.840.113549.1.9.1`, which section 4.2.1.10 constrains as an
/// `rfc822Name` where a certificate states no subject alternative name.
const EMAIL_ADDRESS: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x01];

/// How the context tags inside a `NameConstraints` or `PolicyConstraints` are written.
///
/// RFC 5280's own module is `IMPLICIT TAGS`, so a certificate's extension writes `[0]` in place of
/// the tag it replaces. ETSI TS 119 172-3 Annex A and RFC 3125 section A.1 restate both types in
/// `EXPLICIT TAGS` modules, so a signature policy written in either ASN.1 syntax wraps the value
/// inside `[0]` instead (ADR 2724).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tagging {
    /// RFC 5280's module: the context tag replaces the inner one.
    Implicit,
    /// RFC 3125's and TS 119 172-3's modules: the context tag wraps the inner value.
    Explicit,
}

/// One `GeneralName`, as far as name constraints read it — RFC 5280 section 4.2.1.6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Name {
    /// `rfc822Name`, an `IA5String` mailbox.
    Rfc822(String),
    /// `dNSName`.
    Dns(String),
    /// `uniformResourceIdentifier`.
    Uri(String),
    /// `directoryName`: the relative distinguished names, in order.
    Directory(Vec<Rdn>),
    /// `iPAddress`: four or sixteen octets as a name, twice that as a constraint.
    IpAddress(Vec<u8>),
    /// A form this program does not compare — `otherName`, `x400Address`, `ediPartyName` or
    /// `registeredID` — or one of the string forms that is not ASCII, by its tag number.
    Unread(u8),
}

impl Name {
    /// The `GeneralName` tag number, which is what "the same name form" means.
    #[must_use]
    pub const fn form(&self) -> u8 {
        match self {
            Self::Rfc822(_) => 1,
            Self::Dns(_) => 2,
            Self::Directory(_) => 4,
            Self::Uri(_) => 6,
            Self::IpAddress(_) => 7,
            Self::Unread(form) => *form,
        }
    }
}

/// The word for a `GeneralName` form, as a refusal says it.
#[must_use]
pub const fn form_word(form: u8) -> &'static str {
    match form {
        0 => "otherName",
        1 => "rfc822Name",
        2 => "dNSName",
        3 => "x400Address",
        4 => "directoryName",
        5 => "ediPartyName",
        6 => "uniformResourceIdentifier",
        7 => "iPAddress",
        _ => "registeredID",
    }
}

/// One relative distinguished name: its attributes, each a type and a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rdn(Vec<Attribute>);

/// One `AttributeTypeAndValue`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The attribute type's object identifier, as encoded.
    kind: Vec<u8>,
    /// The value's whole encoding, which two identical values share whatever they hold.
    encoding: Vec<u8>,
    /// The value as text, where it is one of the string types section 7.1 compares.
    text: Option<String>,
}

/// A `GeneralSubtree` whose base this program reads — section 4.2.1.10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subtree {
    /// `base`.
    pub base: Name,
}

/// A `NameConstraints`: the permitted and the excluded subtrees.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameConstraints {
    /// `permittedSubtrees`, where stated.
    pub permitted: Option<Vec<Subtree>>,
    /// `excludedSubtrees`, empty where not stated.
    pub excluded: Vec<Subtree>,
}

/// Why a `NameConstraints` or a certificate's names could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unreadable {
    /// The encoding is not the type's.
    Malformed,
    /// A subtree states a `minimum` other than zero or a `maximum`, which section 4.2.1.10 forbids
    /// ("Within this profile, the minimum and maximum fields are not used with any name forms,
    /// thus, the minimum MUST be zero, and maximum MUST be absent") and this program does not
    /// process.
    DistanceStated,
    /// More names or subtrees than [`MAX_NAMES`] or [`MAX_SUBTREES`].
    PastBound,
}

/// Reads a `NameConstraints` under `tagging`.
///
/// # Errors
///
/// [`Unreadable`] where it is not one, or past the bound.
pub fn read_constraints(encoded: &[u8], tagging: Tagging) -> Result<NameConstraints, Unreadable> {
    let value = single(encoded)?;
    if value.identifier != SEQUENCE {
        return Err(Unreadable::Malformed);
    }
    let mut read = NameConstraints::default();
    let mut members = value.children().map_err(|_| Unreadable::Malformed)?;
    while let Some(member) = members.next_value().map_err(|_| Unreadable::Malformed)? {
        if member.class() != Class::ContextSpecific || !member.is_constructed() {
            return Err(Unreadable::Malformed);
        }
        let list = match tagging {
            Tagging::Implicit => member,
            Tagging::Explicit => only_child(&member)?,
        };
        let mut subtrees = Vec::new();
        let mut entries = list.children().map_err(|_| Unreadable::Malformed)?;
        while let Some(entry) = entries.next_value().map_err(|_| Unreadable::Malformed)? {
            if subtrees.len() >= MAX_SUBTREES {
                return Err(Unreadable::PastBound);
            }
            subtrees.push(read_subtree(&entry, tagging)?);
        }
        // "GeneralSubtrees ::= SEQUENCE SIZE (1..MAX) OF GeneralSubtree".
        if subtrees.is_empty() {
            return Err(Unreadable::Malformed);
        }
        match member.tag_number() {
            0 if read.permitted.is_none() => read.permitted = Some(subtrees),
            1 if read.excluded.is_empty() => read.excluded = subtrees,
            _ => return Err(Unreadable::Malformed),
        }
    }
    // Section 4.2.1.10: "Conforming CAs MUST NOT issue certificates where name constraints is an
    // empty sequence. That is, either the permittedSubtrees field or the excludedSubtrees MUST be
    // present."
    if read.permitted.is_none() && read.excluded.is_empty() {
        return Err(Unreadable::Malformed);
    }
    Ok(read)
}

/// A subtree from the parts TS 119 172-2 clause 4.22.3.2's XML states separately: the DER of a
/// `GeneralName`, and the distances.
///
/// # Errors
///
/// [`Unreadable`] where the base is not a `GeneralName` or a distance is stated.
pub fn subtree_of(base: &[u8], minimum: i64, maximum: Option<i64>) -> Result<Subtree, Unreadable> {
    if minimum != 0 || maximum.is_some() {
        return Err(Unreadable::DistanceStated);
    }
    Ok(Subtree {
        base: general_name(&single(base)?)?,
    })
}

/// One `GeneralSubtree`.
fn read_subtree(entry: &Value<'_>, tagging: Tagging) -> Result<Subtree, Unreadable> {
    if entry.identifier != SEQUENCE {
        return Err(Unreadable::Malformed);
    }
    let mut parts = entry.children().map_err(|_| Unreadable::Malformed)?;
    let base = parts
        .next_value()
        .map_err(|_| Unreadable::Malformed)?
        .ok_or(Unreadable::Malformed)?;
    let base = general_name(&base)?;
    while let Some(distance) = parts.next_value().map_err(|_| Unreadable::Malformed)? {
        let integer = match tagging {
            Tagging::Implicit => distance.contents,
            Tagging::Explicit => only_child(&distance)?.contents,
        };
        // A `minimum` of zero is its DEFAULT written out, which DER forbids and BER permits; a
        // `maximum`, or any other `minimum`, is a distance this program does not process.
        if !(distance.is_context(0) && integer.iter().all(|&octet| octet == 0)) {
            return Err(Unreadable::DistanceStated);
        }
    }
    Ok(Subtree { base })
}

/// RFC 5280 section 4.2.1.6's `GeneralName`, from its `IMPLICIT`-tagged encoding.
///
/// # Errors
///
/// [`Unreadable::Malformed`] where a form this program reads is not shaped like itself.
pub fn general_name(value: &Value<'_>) -> Result<Name, Unreadable> {
    if value.class() != Class::ContextSpecific {
        return Err(Unreadable::Malformed);
    }
    let form = value.tag_number();
    let ascii = || {
        std::str::from_utf8(value.contents)
            .ok()
            .filter(|text| text.is_ascii())
            .map(str::to_owned)
    };
    Ok(match form {
        1 => ascii().map_or(Name::Unread(1), Name::Rfc822),
        2 => ascii().map_or(Name::Unread(2), Name::Dns),
        6 => ascii().map_or(Name::Unread(6), Name::Uri),
        // `directoryName [4] Name` is explicit by necessity, because `Name` is a CHOICE.
        4 => Name::Directory(directory(&only_child(value)?)?),
        7 => Name::IpAddress(value.contents.to_vec()),
        0 | 3 | 5 | 8 => Name::Unread(form),
        _ => return Err(Unreadable::Malformed),
    })
}

/// A `Name`'s `RDNSequence`, read into its relative distinguished names.
///
/// # Errors
///
/// [`Unreadable`] where it is not an `RDNSequence`, or past `MAX_RDNS`.
pub fn directory(name: &Value<'_>) -> Result<Vec<Rdn>, Unreadable> {
    if name.identifier != SEQUENCE {
        return Err(Unreadable::Malformed);
    }
    let mut rdns = Vec::new();
    let mut sequence = name.children().map_err(|_| Unreadable::Malformed)?;
    while let Some(set) = sequence.next_value().map_err(|_| Unreadable::Malformed)? {
        if set.identifier != SET || rdns.len() >= MAX_RDNS {
            return Err(if set.identifier == SET {
                Unreadable::PastBound
            } else {
                Unreadable::Malformed
            });
        }
        let mut attributes = Vec::new();
        let mut members = set.children().map_err(|_| Unreadable::Malformed)?;
        while let Some(pair) = members.next_value().map_err(|_| Unreadable::Malformed)? {
            if pair.identifier != SEQUENCE || attributes.len() >= MAX_RDNS {
                return Err(Unreadable::Malformed);
            }
            let mut fields = pair.children().map_err(|_| Unreadable::Malformed)?;
            let kind = fields
                .next_value()
                .map_err(|_| Unreadable::Malformed)?
                .and_then(|kind| kind.object_identifier())
                .ok_or(Unreadable::Malformed)?;
            let value = fields
                .next_value()
                .map_err(|_| Unreadable::Malformed)?
                .ok_or(Unreadable::Malformed)?;
            attributes.push(Attribute {
                kind: kind.to_vec(),
                encoding: value.encoding().to_vec(),
                text: text(&value),
            });
        }
        if attributes.is_empty() {
            return Err(Unreadable::Malformed);
        }
        rdns.push(Rdn(attributes));
    }
    Ok(rdns)
}

/// An attribute value as text, where it is one of the string types section 7.1 compares:
/// `PrintableString`, `UTF8String`, `IA5String` and `VisibleString` as written, `BMPString` and
/// `UniversalString` decoded. `TeletexString`, whose support section 7.1 calls optional, is not.
fn text(value: &Value<'_>) -> Option<String> {
    const UTF8_STRING: u8 = 0x0C;
    const PRINTABLE_STRING: u8 = 0x13;
    const IA5_STRING: u8 = 0x16;
    const VISIBLE_STRING: u8 = 0x1A;
    const UNIVERSAL_STRING: u8 = 0x1C;
    const BMP_STRING: u8 = 0x1E;
    match value.identifier {
        UTF8_STRING | PRINTABLE_STRING | IA5_STRING | VISIBLE_STRING => {
            std::str::from_utf8(value.contents).ok().map(str::to_owned)
        }
        BMP_STRING => {
            let units = value
                .contents
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
            (value.contents.len().is_multiple_of(2))
                .then(|| {
                    char::decode_utf16(units)
                        .collect::<Result<String, _>>()
                        .ok()
                })
                .flatten()
        }
        UNIVERSAL_STRING => (value.contents.len().is_multiple_of(4))
            .then(|| {
                value
                    .contents
                    .chunks_exact(4)
                    .map(|quad| {
                        char::from_u32(u32::from_be_bytes([quad[0], quad[1], quad[2], quad[3]]))
                    })
                    .collect::<Option<String>>()
            })
            .flatten(),
        _ => None,
    }
}

/// RFC 4518's preparation of a stored value as section 7.1 asks for it, where it is exact.
///
/// Step 2 maps the white-space controls to SPACE and the other controls to nothing and folds case;
/// step 6 compresses white space, which for comparison is trimming both ends and making every run
/// one SPACE. On ASCII that is the whole algorithm. `None` where the text holds anything outside
/// ASCII, because the mapping and normalisation tables steps 2 and 3 name are not in this tree.
fn prepared(text: &str) -> Option<String> {
    if !text.is_ascii() {
        return None;
    }
    let mapped: String = text
        .chars()
        .filter_map(|c| match c {
            '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' => Some(' '),
            c if c.is_ascii_control() => None,
            c => Some(c.to_ascii_lowercase()),
        })
        .collect();
    Some(
        mapped
            .split(' ')
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// The answer to whether a name lies in a subtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    /// It does.
    Yes,
    /// It does not.
    No,
    /// This program cannot decide: see the module's comparison section.
    Undecided,
}

impl Match {
    const fn of(decided: bool) -> Self {
        if decided { Self::Yes } else { Self::No }
    }
}

/// Section 7.1's attribute match.
fn attribute_matches(left: &Attribute, right: &Attribute) -> Match {
    if left.kind != right.kind {
        return Match::No;
    }
    if left.encoding == right.encoding {
        return Match::Yes;
    }
    match (&left.text, &right.text) {
        (Some(a), Some(b)) => match (prepared(a), prepared(b)) {
            (Some(a), Some(b)) => Match::of(a == b),
            _ => Match::Undecided,
        },
        // A string against a value that is not one, of the same attribute type: different values
        // unless one of them is a string type this program does not decode.
        _ => Match::Undecided,
    }
}

/// Section 7.1's RDN match: "the same number of naming attributes and for each naming attribute in
/// RDN1 there is a matching naming attribute in RDN2".
fn rdn_matches(left: &Rdn, right: &Rdn) -> Match {
    if left.0.len() != right.0.len() {
        return Match::No;
    }
    let mut answer = Match::Yes;
    for attribute in &left.0 {
        let best = right.0.iter().fold(Match::No, |best, other| {
            match (best, attribute_matches(attribute, other)) {
                (Match::Yes, _) | (_, Match::Yes) => Match::Yes,
                (Match::Undecided, _) | (_, Match::Undecided) => Match::Undecided,
                _ => Match::No,
            }
        });
        answer = match (answer, best) {
            (_, Match::No) | (Match::No, _) => return Match::No,
            (Match::Undecided, _) | (_, Match::Undecided) => Match::Undecided,
            _ => Match::Yes,
        };
    }
    answer
}

/// Whether `name` lies in the subtree `base` — section 4.2.1.10's rule for each form.
#[must_use]
pub fn within(name: &Name, base: &Name) -> Match {
    match (name, base) {
        // "A distinguished name DN1 is within the subtree defined by the distinguished name DN2 if
        // DN1 contains at least as many RDNs as DN2, and DN1 and DN2 are a match when trailing
        // RDNs in DN1 are ignored" (section 7.1).
        (Name::Directory(name), Name::Directory(base)) => {
            if name.len() < base.len() {
                return Match::No;
            }
            let mut answer = Match::Yes;
            for (held, wanted) in name.iter().zip(base) {
                match rdn_matches(held, wanted) {
                    Match::No => return Match::No,
                    Match::Undecided => answer = Match::Undecided,
                    Match::Yes => {}
                }
            }
            answer
        }
        (Name::Rfc822(name), Name::Rfc822(base)) => mailbox_within(name, base),
        (Name::Dns(name), Name::Dns(base)) => Match::of(domain_within(name, base, false)),
        (Name::Uri(name), Name::Uri(base)) => uri_host(name).map_or(Match::Undecided, |host| {
            Match::of(domain_within(host, base, true))
        }),
        (Name::IpAddress(name), Name::IpAddress(base)) => address_within(name, base),
        _ => Match::No,
    }
}

/// Section 4.2.1.10's `rfc822Name` rule: a constraint with `@` is one mailbox, one beginning with a
/// period is every host in that domain, any other is every mailbox on that one host. Section 7.5
/// compares the local part exactly and the host without regard to case.
fn mailbox_within(name: &str, base: &str) -> Match {
    let Some((local, host)) = name.rsplit_once('@') else {
        return Match::Undecided;
    };
    if let Some((wanted_local, wanted_host)) = base.rsplit_once('@') {
        return Match::of(local == wanted_local && host.eq_ignore_ascii_case(wanted_host));
    }
    if base.starts_with('.') {
        return Match::of(ends_with_ignoring_case(host, base));
    }
    Match::of(host.eq_ignore_ascii_case(base))
}

/// Section 4.2.1.10's `dNSName` rule — "any DNS name that can be constructed by simply adding zero
/// or more labels to the left-hand side of the name satisfies the name constraint" — and its URI
/// rule, under which a constraint beginning with a period admits only names below it. A `dNSName`
/// constraint beginning with a period is read the URI's way, since no name can equal it.
fn domain_within(name: &str, base: &str, uri: bool) -> bool {
    if base.is_empty() {
        return true;
    }
    if base.starts_with('.') {
        return ends_with_ignoring_case(name, base);
    }
    if name.eq_ignore_ascii_case(base) {
        return true;
    }
    !uri && name.len() > base.len() && {
        let cut = name.len().saturating_sub(base.len());
        name.as_bytes().get(cut.saturating_sub(1)) == Some(&b'.')
            && name
                .get(cut..)
                .is_some_and(|tail| tail.eq_ignore_ascii_case(base))
    }
}

/// Whether `text` ends with `suffix`, ASCII case ignored.
fn ends_with_ignoring_case(text: &str, suffix: &str) -> bool {
    text.len() >= suffix.len()
        && text
            .get(text.len().saturating_sub(suffix.len())..)
            .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
}

/// The host of a URI's authority, where it is a domain name — section 4.2.1.10: "The constraint
/// MUST be applied to the host part of the name." `None` where there is no authority or its host is
/// an IP literal, which the same section makes a rejection where the form is constrained.
fn uri_host(uri: &str) -> Option<&str> {
    let (_, rest) = uri.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    if host_port.starts_with('[') {
        return None;
    }
    let host = host_port.split(':').next()?;
    let ip_literal = host.chars().all(|c| c.is_ascii_digit() || c == '.');
    (!host.is_empty() && !ip_literal).then_some(host)
}

/// Section 4.2.1.10's `iPAddress` rule: the constraint is an address and a mask of twice the
/// name's length, and a name of the other family is outside it.
fn address_within(name: &[u8], base: &[u8]) -> Match {
    if !matches!(base.len(), 8 | 32) {
        return Match::Undecided;
    }
    let (address, mask) = base.split_at(base.len() / 2);
    if name.len() != address.len() {
        return Match::No;
    }
    Match::of(
        name.iter()
            .zip(address)
            .zip(mask)
            .all(|((n, a), m)| n & m == a & m),
    )
}

/// Why a certificate's names are outside the constraints of the path above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameRefusal {
    /// A name lies in no subtree of a permitted set that constrains its form — section 6.1.3 (b).
    NotPermitted(u8),
    /// A name lies in an excluded subtree — section 6.1.3 (c).
    Excluded(u8),
    /// A URI under a URI constraint names no domain host, which section 4.2.1.10 makes a rejection.
    UriWithoutDomainHost,
    /// A name of a form this program does not compare stands under a constraint on that form.
    FormNotProcessed(u8),
    /// A comparison section 7.1 would decide and this program cannot.
    Undecided(u8),
}

/// Section 6.1.2 (b) and (c)'s `permitted_subtrees` and `excluded_subtrees`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameState {
    /// Every permitted set stated so far; a name must lie in one subtree of each that constrains
    /// its form. Empty is section 6.1.2 (b)'s "set of all names of each name type".
    permitted: Vec<Vec<Subtree>>,
    /// The union of every excluded set stated so far.
    excluded: Vec<Subtree>,
    /// How many subtrees are held, against [`MAX_SUBTREES`].
    held: usize,
}

impl NameState {
    /// Section 6.1.4 (g): narrows the permitted subtrees by `constraints` and adds its excluded ones.
    ///
    /// # Errors
    ///
    /// [`Unreadable::PastBound`] past [`MAX_SUBTREES`].
    pub fn admit(&mut self, constraints: NameConstraints) -> Result<(), Unreadable> {
        let count = constraints
            .permitted
            .as_ref()
            .map_or(0, Vec::len)
            .saturating_add(constraints.excluded.len());
        self.held = self.held.saturating_add(count);
        if self.held > MAX_SUBTREES {
            return Err(Unreadable::PastBound);
        }
        if let Some(permitted) = constraints.permitted {
            self.permitted.push(permitted);
        }
        self.excluded.extend(constraints.excluded);
        Ok(())
    }

    /// Whether no constraint is held at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.permitted.is_empty() && self.excluded.is_empty()
    }

    /// Section 6.1.3 (b) and (c) over one certificate's names.
    ///
    /// # Errors
    ///
    /// The first [`NameRefusal`], name by name in the order given.
    pub fn check(&self, names: &[Name]) -> Result<(), NameRefusal> {
        for name in names {
            let form = name.form();
            let constrained = self
                .permitted
                .iter()
                .flatten()
                .chain(&self.excluded)
                .any(|subtree| subtree.base.form() == form);
            if !constrained {
                continue;
            }
            if matches!(name, Name::Unread(_)) {
                return Err(NameRefusal::FormNotProcessed(form));
            }
            if let Name::Uri(uri) = name
                && uri_host(uri).is_none()
            {
                return Err(NameRefusal::UriWithoutDomainHost);
            }
            for set in &self.permitted {
                let mut answer = None;
                for subtree in set.iter().filter(|subtree| subtree.base.form() == form) {
                    let found = within(name, &subtree.base);
                    answer = Some(match (answer, found) {
                        (Some(Match::Yes), _) | (_, Match::Yes) => Match::Yes,
                        (Some(Match::Undecided), _) | (_, Match::Undecided) => Match::Undecided,
                        _ => Match::No,
                    });
                }
                match answer {
                    Some(Match::No) => return Err(NameRefusal::NotPermitted(form)),
                    Some(Match::Undecided) => return Err(NameRefusal::Undecided(form)),
                    _ => {}
                }
            }
            for subtree in self
                .excluded
                .iter()
                .filter(|subtree| subtree.base.form() == form)
            {
                match within(name, &subtree.base) {
                    Match::Yes => return Err(NameRefusal::Excluded(form)),
                    Match::Undecided => return Err(NameRefusal::Undecided(form)),
                    Match::No => {}
                }
            }
        }
        Ok(())
    }
}

/// The names section 6.1.3 (b) and (c) check for one certificate: the subject, where it is not
/// empty, every `subjectAltName` entry, and — where there is no `subjectAltName`, as section
/// 4.2.1.10 says — each `emailAddress` in the subject as an `rfc822Name`.
///
/// # Errors
///
/// [`Unreadable`] where the subject or the extension does not read, or past [`MAX_NAMES`].
pub fn names_of(
    subject_encoding: &[u8],
    alt_names: Option<&[u8]>,
) -> Result<Vec<Name>, Unreadable> {
    let rdns = directory(&single(subject_encoding)?)?;
    let mut names = Vec::new();
    if alt_names.is_none() {
        for attribute in rdns.iter().flat_map(|rdn| &rdn.0) {
            if attribute.kind == EMAIL_ADDRESS {
                names.push(
                    attribute
                        .text
                        .clone()
                        .filter(|text| text.is_ascii())
                        .map_or(Name::Unread(1), Name::Rfc822),
                );
            }
        }
    }
    if !rdns.is_empty() {
        names.push(Name::Directory(rdns));
    }
    if let Some(encoded) = alt_names {
        let list = single(encoded)?;
        if list.identifier != SEQUENCE {
            return Err(Unreadable::Malformed);
        }
        let mut entries = list.children().map_err(|_| Unreadable::Malformed)?;
        while let Some(entry) = entries.next_value().map_err(|_| Unreadable::Malformed)? {
            names.push(general_name(&entry)?);
            if names.len() > MAX_NAMES {
                return Err(Unreadable::PastBound);
            }
        }
    }
    if names.len() > MAX_NAMES {
        return Err(Unreadable::PastBound);
    }
    Ok(names)
}

/// One attribute of a distinguished name, flattened out of its relative distinguished name: the
/// type's dotted identifier and the value as text, where it is a string type section 7.1 compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedAttribute {
    /// The attribute type, dotted.
    pub kind: String,
    /// The value as text; `None` where it is not a string type this program decodes.
    pub text: Option<String>,
}

/// The attributes of a `Name` given as a certificate's `issuer` or `subject` field holds it — the
/// `RDNSequence`'s contents, its header taken off (`pdf_pki::x509::Certificate::issuer`).
///
/// # Errors
///
/// [`Unreadable`] where the contents are not an `RDNSequence`'s, or past the bound.
pub fn attributes_of(contents: &[u8]) -> Result<Vec<NamedAttribute>, Unreadable> {
    let rdns = directory(&single(&sequence_around(contents))?)?;
    Ok(rdns
        .iter()
        .flat_map(|rdn| &rdn.0)
        .map(|attribute| NamedAttribute {
            kind: crate::x509::dotted(&attribute.kind).unwrap_or_default(),
            text: attribute.text.clone(),
        })
        .collect())
}

/// Section 7.1's comparison of two attribute values held as text, after RFC 4518's preparation
/// where it is exact (`prepared`); identical text matches whatever it holds, and text outside
/// ASCII that differs is [`Match::Undecided`], because the case folding the preparation asks of it
/// is not in this tree.
#[must_use]
pub fn texts_match(left: &str, right: &str) -> Match {
    if left == right {
        return Match::Yes;
    }
    match (prepared(left), prepared(right)) {
        (Some(a), Some(b)) => Match::of(a == b),
        _ => Match::Undecided,
    }
}

/// Section 7.1's distinguished name match between two `Name`s given as a certificate's fields
/// hold them (the `RDNSequence`'s contents): the same number of RDNs, each matching its peer in
/// order.
#[must_use]
pub fn names_match(left: &[u8], right: &[u8]) -> Match {
    let read =
        |contents: &[u8]| single(&sequence_around(contents)).and_then(|value| directory(&value));
    let (Ok(left), Ok(right)) = (read(left), read(right)) else {
        return Match::Undecided;
    };
    if left.len() != right.len() {
        return Match::No;
    }
    let mut answer = Match::Yes;
    for (a, b) in left.iter().zip(&right) {
        answer = match (answer, rdn_matches(a, b)) {
            (_, Match::No) | (Match::No, _) => return Match::No,
            (Match::Undecided, _) | (_, Match::Undecided) => Match::Undecided,
            _ => Match::Yes,
        };
    }
    answer
}

/// `contents` with a `SEQUENCE` header put back in front of them, in X.690 clause 8.1.3's
/// definite form.
fn sequence_around(contents: &[u8]) -> Vec<u8> {
    let mut encoded = vec![SEQUENCE];
    let length = contents.len();
    if length < 0x80 {
        // Below 0x80 the short form is the length itself, which fits an octet.
        encoded.push(u8::try_from(length).unwrap_or(0));
    } else {
        let octets: Vec<u8> = length
            .to_be_bytes()
            .into_iter()
            .skip_while(|&octet| octet == 0)
            .collect();
        encoded.push(0x80 | u8::try_from(octets.len()).unwrap_or(0));
        encoded.extend_from_slice(&octets);
    }
    encoded.extend_from_slice(contents);
    encoded
}

/// The one value `encoded` holds, with nothing after it.
fn single(encoded: &[u8]) -> Result<Value<'_>, Unreadable> {
    let mut reader = Reader::new(encoded).map_err(|_| Unreadable::Malformed)?;
    let value = reader
        .next_value()
        .map_err(|_| Unreadable::Malformed)?
        .ok_or(Unreadable::Malformed)?;
    match reader.next_value() {
        Ok(None) => Ok(value),
        _ => Err(Unreadable::Malformed),
    }
}

/// The one value inside a constructed tag.
fn only_child<'a>(tagged: &Value<'a>) -> Result<Value<'a>, Unreadable> {
    let mut children = tagged.children().map_err(|_| Unreadable::Malformed)?;
    let child = children
        .next_value()
        .map_err(|_| Unreadable::Malformed)?
        .ok_or(Unreadable::Malformed)?;
    match children.next_value() {
        Ok(None) => Ok(child),
        _ => Err(Unreadable::Malformed),
    }
}

#[cfg(test)]
mod tests;
