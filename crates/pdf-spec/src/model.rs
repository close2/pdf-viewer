//! The types the generated Arlington tables are built from.
//!
//! Everything here is `'static` data. The generated tables are `static` arrays of these
//! structures, so the object model costs *zero* parsing at startup — it lives in the
//! binary's read-only data and is addressed directly. That is a requirement, not an
//! optimisation: see `CLAUDE.md` principle 2 on startup time.

use std::fmt;

/// A PDF version, as used by Arlington's `SinceVersion` and `DeprecatedIn` columns.
///
/// Ordered, so `>=` comparisons express "at least this version" directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    /// PDF 1.0.
    V1_0,
    /// PDF 1.1.
    V1_1,
    /// PDF 1.2.
    V1_2,
    /// PDF 1.3.
    V1_3,
    /// PDF 1.4.
    V1_4,
    /// PDF 1.5.
    V1_5,
    /// PDF 1.6.
    V1_6,
    /// PDF 1.7.
    V1_7,
    /// PDF 2.0, that is ISO 32000-2.
    V2_0,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::V1_0 => "1.0",
            Self::V1_1 => "1.1",
            Self::V1_2 => "1.2",
            Self::V1_3 => "1.3",
            Self::V1_4 => "1.4",
            Self::V1_5 => "1.5",
            Self::V1_6 => "1.6",
            Self::V1_7 => "1.7",
            Self::V2_0 => "2.0",
        };
        f.write_str(text)
    }
}

/// A value type an object key may hold.
///
/// These are Arlington's own type names rather than the specification's prose
/// categories, because the tables are generated from Arlington and a translation layer
/// would only be somewhere for a mistake to hide. The build fails on an unrecognised
/// token, so a vocabulary change upstream cannot be silently dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PdfType {
    /// An array object.
    Array,
    /// An integer used as a set of flags.
    Bitmask,
    /// A boolean object.
    Boolean,
    /// A string in PDF date format.
    Date,
    /// A dictionary object.
    Dictionary,
    /// An integer number.
    Integer,
    /// A six-element array holding a transformation matrix.
    Matrix,
    /// A name object.
    Name,
    /// A name tree.
    NameTree,
    /// The null object.
    Null,
    /// A real number, or an integer where either is accepted.
    Number,
    /// A number tree.
    NumberTree,
    /// A four-element array holding a rectangle.
    Rectangle,
    /// A stream object.
    Stream,
    /// A string whose interpretation is unspecified.
    String,
    /// A string restricted to ASCII.
    StringAscii,
    /// A string of arbitrary bytes.
    StringByte,
    /// A text string, subject to the text-string encoding rules.
    StringText,
}

impl PdfType {
    /// Returns the Arlington token for this type.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Array => "array",
            Self::Bitmask => "bitmask",
            Self::Boolean => "boolean",
            Self::Date => "date",
            Self::Dictionary => "dictionary",
            Self::Integer => "integer",
            Self::Matrix => "matrix",
            Self::Name => "name",
            Self::NameTree => "name-tree",
            Self::Null => "null",
            Self::Number => "number",
            Self::NumberTree => "number-tree",
            Self::Rectangle => "rectangle",
            Self::Stream => "stream",
            Self::String => "string",
            Self::StringAscii => "string-ascii",
            Self::StringByte => "string-byte",
            Self::StringText => "string-text",
        }
    }
}

impl fmt::Display for PdfType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// What a row's `Key` column matches.
///
/// Arlington uses one column for three different things, and conflating them at lookup
/// time would silently accept a dictionary key where an array index belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyPattern {
    /// A named dictionary key.
    Name(&'static str),
    /// A fixed array element at this zero-based index.
    Index(u32),
    /// `*`: any key of a map-like dictionary, or any element of an array.
    Wildcard,
    /// `<digit>*`: a member of a repeating group beginning at this index.
    ///
    /// The trailing rows of a repeating array all carry this, and together they describe
    /// one period of the repeat. An array of `0` then `1*` `2*` therefore holds one fixed
    /// element followed by any number of pairs.
    Repeating(u32),
}

/// Whether a key must be present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// Always required.
    Always,
    /// Never required.
    Never,
    /// Required only when a condition holds.
    ///
    /// Carries Arlington's predicate verbatim, unevaluated. A validator that cannot
    /// evaluate it must report that it could not decide, rather than guessing either way:
    /// treating the key as optional would accept invalid files, and as required would
    /// reject valid ones.
    Conditional(&'static str),
}

/// Whether a value must be an indirect reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indirectness {
    /// Must be an indirect reference.
    Required,
    /// Must be a direct object.
    Forbidden,
    /// Either is acceptable.
    Either,
    /// Constrained by a condition, carried verbatim and unevaluated.
    Conditional(&'static str),
}

/// From which version a key is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Present from this version onwards.
    Since(Version),
    /// Present only when an ISO extension is in use.
    Extension {
        /// The extension's Arlington name.
        name: &'static str,
        /// The version the extension itself dates from, where Arlington records one.
        since: Option<Version>,
    },
    /// Standard from `since`, and available earlier under an extension.
    ///
    /// The common shape in Arlington: a feature that shipped as a vendor extension and
    /// was later folded into the standard.
    SinceOrExtension {
        /// Version from which it is standard.
        since: Version,
        /// The extension that provided it earlier.
        extension: &'static str,
        /// The version the extension dates from, where recorded.
        extension_since: Option<Version>,
    },
}

impl Availability {
    /// Returns the version from which this key is available *without* any extension.
    ///
    /// `None` for extension-only keys.
    #[must_use]
    pub fn standard_since(self) -> Option<Version> {
        match self {
            Self::Since(version) | Self::SinceOrExtension { since: version, .. } => Some(version),
            Self::Extension { .. } => None,
        }
    }
}

/// A condition gating one type alternative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeGate {
    /// The type always applies.
    Always,
    /// The type applies only under an ISO extension.
    Extension(&'static str),
    /// The type was deprecated in this version.
    DeprecatedIn(Version),
}

/// One of the types a key may hold, with the constraints belonging to that type.
///
/// Arlington aligns `Type`, `Link`, `PossibleValues` and bracketed `IndirectReference`
/// positionally by `;`, so the constraints for the third type alternative are the third
/// group of each column. That alignment is verified at generation time across the whole
/// model, and grouping them here means a consumer cannot mismatch them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeAlternative {
    /// The value type.
    pub kind: PdfType,
    /// Object types this may link to, for `dictionary`, `stream` and `array` values.
    pub links: &'static [&'static str],
    /// Permitted values, verbatim from Arlington, if constrained.
    ///
    /// Left as text because the syntax spans name sets, numeric ranges and predicates;
    /// parsing it is the next increment, and half-parsing it would be worse than not
    /// starting.
    pub possible_values: Option<&'static str>,
    /// Whether this alternative must be an indirect reference.
    pub indirect: Indirectness,
    /// A condition restricting when this alternative applies.
    pub gate: TypeGate,
}

/// One row of an Arlington object definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeySpec {
    /// What this row matches.
    pub pattern: KeyPattern,
    /// The permitted types, in Arlington's order.
    pub types: &'static [TypeAlternative],
    /// From which version the key exists.
    pub availability: Availability,
    /// The version the key was deprecated in, if any.
    pub deprecated_in: Option<Version>,
    /// Whether the key must be present.
    pub required: Requirement,
    /// Whether the key's value is inherited from ancestor nodes when absent.
    pub inheritable: bool,
    /// The default value, verbatim, if the specification gives one.
    pub default_value: Option<&'static str>,
    /// Arlington's `SpecialCase` predicate, verbatim and unevaluated.
    pub special_case: Option<&'static str>,
}

/// An Arlington object definition: one TSV file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectSpec {
    /// The object's name, which is its TSV filename without extension.
    pub name: &'static str,
    /// Its rows, in file order.
    pub keys: &'static [KeySpec],
}

impl ObjectSpec {
    /// Looks up a dictionary key by name.
    ///
    /// Falls back to a [`KeyPattern::Wildcard`] row when no named row matches, which is
    /// how map-like objects permit arbitrary keys. Returns `None` when the key is neither
    /// named nor covered by a wildcard — which for a non-map object means the key does not
    /// belong in this dictionary.
    #[must_use]
    pub fn key(&self, name: &str) -> Option<&'static KeySpec> {
        self.keys
            .iter()
            .find(|spec| matches!(spec.pattern, KeyPattern::Name(candidate) if candidate == name))
            .or_else(|| {
                self.keys
                    .iter()
                    .find(|spec| matches!(spec.pattern, KeyPattern::Wildcard))
            })
    }

    /// Looks up an array element by zero-based index.
    ///
    /// Resolves fixed elements first, then a repeating group, then a wildcard. For a
    /// repeating group the index is reduced modulo the group's period, so element 7 of an
    /// array of pairs beginning at index 1 resolves to the same row as element 5.
    #[must_use]
    pub fn element(&self, index: u32) -> Option<&'static KeySpec> {
        if let Some(fixed) = self
            .keys
            .iter()
            .find(|spec| spec.pattern == KeyPattern::Index(index))
        {
            return Some(fixed);
        }

        let group: Vec<&'static KeySpec> = self
            .keys
            .iter()
            .filter(|spec| matches!(spec.pattern, KeyPattern::Repeating(_)))
            .collect();

        if let Some(first) = group.first() {
            let KeyPattern::Repeating(start) = first.pattern else {
                unreachable!("filtered to repeating patterns above")
            };
            if index >= start {
                // The group describes one period of the repeat, so an index beyond the
                // first period wraps back into it. `start` is <= `index` here, and the
                // period is at least one, so neither operation can fail.
                let period = u32::try_from(group.len()).unwrap_or(1).max(1);
                let offset = index.saturating_sub(start).checked_rem(period).unwrap_or(0);
                return group.get(usize::try_from(offset).unwrap_or(0)).copied();
            }
        }

        self.keys
            .iter()
            .find(|spec| spec.pattern == KeyPattern::Wildcard)
    }

    /// Returns every key that is unconditionally required.
    ///
    /// Excludes conditionally-required keys: those need a predicate evaluator, and
    /// including them here would misreport them as mandatory.
    pub fn always_required(&self) -> impl Iterator<Item = &'static KeySpec> {
        self.keys
            .iter()
            .filter(|spec| spec.required == Requirement::Always)
    }

    /// Returns `true` if this object accepts arbitrary keys.
    #[must_use]
    pub fn is_map_like(&self) -> bool {
        self.keys
            .iter()
            .any(|spec| spec.pattern == KeyPattern::Wildcard)
    }
}
