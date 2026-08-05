//! The base encodings a PDF font dictionary can name.
//!
//! A simple font maps a one-byte character code to a *glyph name*, and the font program
//! maps that name to an outline. PDF supplies the first half through `/Encoding`: either a
//! base encoding named here, or a dictionary layering `/Differences` over one.
//!
//! # Where these tables come from
//!
//! Both are transcribed from Table D.2 of ISO 32000-2, which is in this repository as
//! `doc/md/ISO_32000-2_sponsored_EC3.md`, together with that table's notes — which carry
//! assignments the table body does not show, and which are the entries most easily got
//! wrong.
//!
//! They are deliberately *not* copies of the platform encodings they are named after.
//! PDF's `MacRomanEncoding` omits the mathematical and symbol glyphs of Mac OS Roman, and
//! keeps code 219 as `currency` where Apple later reassigned it to the euro. Transcribing
//! the platform tables instead would map real documents to real — but wrong — glyphs,
//! which is the silent failure this crate exists to avoid.
//!
//! `StandardEncoding` is not transcribed at all. It is the same table as the CFF
//! specification's standard encoding, which `read-fonts` already carries and which this
//! module defers to, because one table is easier to keep right than two.
//!
//! `MacExpertEncoding` is absent. It is valid but rare, and a font naming it is refused
//! rather than quietly given Latin glyph names.

use skrifa::raw::ps::encoding::PredefinedEncoding;

/// One of the base encodings a PDF font dictionary can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BaseEncoding {
    /// `StandardEncoding`, which is also the CFF standard encoding.
    Standard,
    /// `WinAnsiEncoding`.
    WinAnsi,
    /// `MacRomanEncoding`.
    MacRoman,
}

impl BaseEncoding {
    /// Looks up a base encoding by the name `/Encoding` or `/BaseEncoding` uses.
    ///
    /// Returns `None` for a name with no table here, including the valid-but-absent
    /// `MacExpertEncoding`, so the caller reports it rather than substituting.
    #[must_use]
    pub fn by_name(name: &[u8]) -> Option<Self> {
        match name {
            b"StandardEncoding" => Some(Self::Standard),
            b"WinAnsiEncoding" => Some(Self::WinAnsi),
            b"MacRomanEncoding" => Some(Self::MacRoman),
            _ => None,
        }
    }

    /// Returns the glyph name a character code selects.
    ///
    /// An empty name means the code is unencoded, which is not the same as selecting
    /// `.notdef`: an unencoded code has no glyph to look up, so the caller falls back to
    /// the font's own encoding rather than drawing a missing-glyph box.
    #[must_use]
    pub fn glyph_name(self, code: u8) -> &'static str {
        match self {
            // `read-fonts` spells an unencoded code `.notdef`; the tables here spell it as
            // an empty name, and the two must agree at this boundary.
            Self::Standard => match PredefinedEncoding::Standard.name(code) {
                ".notdef" => "",
                name => name,
            },
            // A `u8` cannot index outside a 256-element array.
            Self::WinAnsi => WIN_ANSI[usize::from(code)],
            Self::MacRoman => MAC_ROMAN[usize::from(code)],
        }
    }
}

/// The `WinAnsiEncoding` base encoding.
/// 
/// Note 3 of Table D.2 assigns every otherwise unused code above 32 to `bullet`,
/// and notes 5 and 6 add `hyphen` at 173 and `space` at 160 — none of which appear
/// in the table body itself.
#[rustfmt::skip]
static WIN_ANSI: [&str; 256] = [
    "",               "",               "",               "", // 0
    "",               "",               "",               "", // 4
    "",               "",               "",               "", // 8
    "",               "",               "",               "", // 12
    "",               "",               "",               "", // 16
    "",               "",               "",               "", // 20
    "",               "",               "",               "", // 24
    "",               "",               "",               "", // 28
    "space",          "exclam",         "quotedbl",       "numbersign", // 32
    "dollar",         "percent",        "ampersand",      "quotesingle", // 36
    "parenleft",      "parenright",     "asterisk",       "plus", // 40
    "comma",          "hyphen",         "period",         "slash", // 44
    "zero",           "one",            "two",            "three", // 48
    "four",           "five",           "six",            "seven", // 52
    "eight",          "nine",           "colon",          "semicolon", // 56
    "less",           "equal",          "greater",        "question", // 60
    "at",             "A",              "B",              "C", // 64
    "D",              "E",              "F",              "G", // 68
    "H",              "I",              "J",              "K", // 72
    "L",              "M",              "N",              "O", // 76
    "P",              "Q",              "R",              "S", // 80
    "T",              "U",              "V",              "W", // 84
    "X",              "Y",              "Z",              "bracketleft", // 88
    "backslash",      "bracketright",   "asciicircum",    "underscore", // 92
    "grave",          "a",              "b",              "c", // 96
    "d",              "e",              "f",              "g", // 100
    "h",              "i",              "j",              "k", // 104
    "l",              "m",              "n",              "o", // 108
    "p",              "q",              "r",              "s", // 112
    "t",              "u",              "v",              "w", // 116
    "x",              "y",              "z",              "braceleft", // 120
    "bar",            "braceright",     "asciitilde",     "bullet", // 124
    "Euro",           "bullet",         "quotesinglbase", "florin", // 128
    "quotedblbase",   "ellipsis",       "dagger",         "daggerdbl", // 132
    "circumflex",     "perthousand",    "Scaron",         "guilsinglleft", // 136
    "OE",             "bullet",         "Zcaron",         "bullet", // 140
    "bullet",         "quoteleft",      "quoteright",     "quotedblleft", // 144
    "quotedblright",  "bullet",         "endash",         "emdash", // 148
    "tilde",          "trademark",      "scaron",         "guilsinglright", // 152
    "oe",             "bullet",         "zcaron",         "Ydieresis", // 156
    "space",          "exclamdown",     "cent",           "sterling", // 160
    "currency",       "yen",            "brokenbar",      "section", // 164
    "dieresis",       "copyright",      "ordfeminine",    "guillemotleft", // 168
    "logicalnot",     "hyphen",         "registered",     "macron", // 172
    "degree",         "plusminus",      "twosuperior",    "threesuperior", // 176
    "acute",          "mu",             "paragraph",      "periodcentered", // 180
    "cedilla",        "onesuperior",    "ordmasculine",   "guillemotright", // 184
    "onequarter",     "onehalf",        "threequarters",  "questiondown", // 188
    "Agrave",         "Aacute",         "Acircumflex",    "Atilde", // 192
    "Adieresis",      "Aring",          "AE",             "Ccedilla", // 196
    "Egrave",         "Eacute",         "Ecircumflex",    "Edieresis", // 200
    "Igrave",         "Iacute",         "Icircumflex",    "Idieresis", // 204
    "Eth",            "Ntilde",         "Ograve",         "Oacute", // 208
    "Ocircumflex",    "Otilde",         "Odieresis",      "multiply", // 212
    "Oslash",         "Ugrave",         "Uacute",         "Ucircumflex", // 216
    "Udieresis",      "Yacute",         "Thorn",          "germandbls", // 220
    "agrave",         "aacute",         "acircumflex",    "atilde", // 224
    "adieresis",      "aring",          "ae",             "ccedilla", // 228
    "egrave",         "eacute",         "ecircumflex",    "edieresis", // 232
    "igrave",         "iacute",         "icircumflex",    "idieresis", // 236
    "eth",            "ntilde",         "ograve",         "oacute", // 240
    "ocircumflex",    "otilde",         "odieresis",      "divide", // 244
    "oslash",         "ugrave",         "uacute",         "ucircumflex", // 248
    "udieresis",      "yacute",         "thorn",          "ydieresis", // 252
];

/// The `MacRomanEncoding` base encoding.
/// 
/// This is PDF's encoding, not Mac OS Roman: the mathematical and symbol glyphs
/// outside the standard Latin set are deliberately unencoded, and code 219 stays
/// `currency` where Mac OS Roman later reassigned it to the euro (note 1).
/// Note 6 adds `space` at 202.
#[rustfmt::skip]
static MAC_ROMAN: [&str; 256] = [
    "",               "",               "",               "", // 0
    "",               "",               "",               "", // 4
    "",               "",               "",               "", // 8
    "",               "",               "",               "", // 12
    "",               "",               "",               "", // 16
    "",               "",               "",               "", // 20
    "",               "",               "",               "", // 24
    "",               "",               "",               "", // 28
    "space",          "exclam",         "quotedbl",       "numbersign", // 32
    "dollar",         "percent",        "ampersand",      "quotesingle", // 36
    "parenleft",      "parenright",     "asterisk",       "plus", // 40
    "comma",          "hyphen",         "period",         "slash", // 44
    "zero",           "one",            "two",            "three", // 48
    "four",           "five",           "six",            "seven", // 52
    "eight",          "nine",           "colon",          "semicolon", // 56
    "less",           "equal",          "greater",        "question", // 60
    "at",             "A",              "B",              "C", // 64
    "D",              "E",              "F",              "G", // 68
    "H",              "I",              "J",              "K", // 72
    "L",              "M",              "N",              "O", // 76
    "P",              "Q",              "R",              "S", // 80
    "T",              "U",              "V",              "W", // 84
    "X",              "Y",              "Z",              "bracketleft", // 88
    "backslash",      "bracketright",   "asciicircum",    "underscore", // 92
    "grave",          "a",              "b",              "c", // 96
    "d",              "e",              "f",              "g", // 100
    "h",              "i",              "j",              "k", // 104
    "l",              "m",              "n",              "o", // 108
    "p",              "q",              "r",              "s", // 112
    "t",              "u",              "v",              "w", // 116
    "x",              "y",              "z",              "braceleft", // 120
    "bar",            "braceright",     "asciitilde",     "", // 124
    "Adieresis",      "Aring",          "Ccedilla",       "Eacute", // 128
    "Ntilde",         "Odieresis",      "Udieresis",      "aacute", // 132
    "agrave",         "acircumflex",    "adieresis",      "atilde", // 136
    "aring",          "ccedilla",       "eacute",         "egrave", // 140
    "ecircumflex",    "edieresis",      "iacute",         "igrave", // 144
    "icircumflex",    "idieresis",      "ntilde",         "oacute", // 148
    "ograve",         "ocircumflex",    "odieresis",      "otilde", // 152
    "uacute",         "ugrave",         "ucircumflex",    "udieresis", // 156
    "dagger",         "degree",         "cent",           "sterling", // 160
    "section",        "bullet",         "paragraph",      "germandbls", // 164
    "registered",     "copyright",      "trademark",      "acute", // 168
    "dieresis",       "",               "AE",             "Oslash", // 172
    "",               "plusminus",      "",               "", // 176
    "yen",            "mu",             "",               "", // 180
    "",               "",               "",               "ordfeminine", // 184
    "ordmasculine",   "",               "ae",             "oslash", // 188
    "questiondown",   "exclamdown",     "logicalnot",     "", // 192
    "florin",         "",               "",               "guillemotleft", // 196
    "guillemotright", "ellipsis",       "space",          "Agrave", // 200
    "Atilde",         "Otilde",         "OE",             "oe", // 204
    "endash",         "emdash",         "quotedblleft",   "quotedblright", // 208
    "quoteleft",      "quoteright",     "divide",         "", // 212
    "ydieresis",      "Ydieresis",      "fraction",       "currency", // 216
    "guilsinglleft",  "guilsinglright", "fi",             "fl", // 220
    "daggerdbl",      "periodcentered", "quotesinglbase", "quotedblbase", // 224
    "perthousand",    "Acircumflex",    "Ecircumflex",    "Aacute", // 228
    "Edieresis",      "Egrave",         "Iacute",         "Icircumflex", // 232
    "Idieresis",      "Igrave",         "Oacute",         "Ocircumflex", // 236
    "",               "Ograve",         "Uacute",         "Ucircumflex", // 240
    "Ugrave",         "dotlessi",       "circumflex",     "tilde", // 244
    "macron",         "breve",          "dotaccent",      "ring", // 248
    "cedilla",        "hungarumlaut",   "ogonek",         "caron", // 252
];

/// Returns the Mac OS Roman code a glyph name occupies, if it occupies one.
///
/// The glyph name a `/Differences` array would use for a character.
///
/// §9.6.5.1 makes an encoding dictionary's `/Differences` an array of "glyph names", and Annex D's
/// tables are written in the same names — so a program that wants a character a base encoding has
/// no code for has to say which glyph it means, by name. This is that step, and it is the only
/// direction §9.6.5 does not state a table for: the annexes go from code to name.
///
/// The Adobe Glyph List answers it, and it is already in this tree — `read-fonts` carries the
/// generated table and [`crate::LoadedFont::text`]'s last resort uses it in the other direction
/// (`name_to_char`, §9.10.2's third method). Nothing is vendored for this.
///
/// A `String` rather than a borrow because the AGL's own API writes into a caller's buffer: the
/// names it computes for characters outside its table are `uniXXXX` forms built on the spot.
#[must_use]
pub fn glyph_name(character: char) -> Option<String> {
    let mut buffer = [0u8; 32];
    read_fonts::ps::agl::char_to_name(character, &mut buffer).map(str::to_owned)
}

/// The character a glyph name stands for, by the Adobe Glyph List.
///
/// §9.10.2's second method in one call: "that name can be looked up in the Adobe Glyph List and
/// Adobe Glyph List for New Fonts to obtain the corresponding Unicode value". `read-fonts` holds
/// both tables and the `uniXXXX` and `uXXXX` forms with them, so nothing is vendored here.
///
/// Public because §9.6.5's simple fonts are not the only place a name is selected by a code:
/// §9.6.4's Type 3 glyph selection is a name too, and its module has no `LoadedFont` to ask.
#[must_use]
pub fn character_for(name: &str) -> Option<char> {
    read_fonts::ps::agl::name_to_char(name)
}

/// The *characters* a glyph name stands for, by the Adobe Glyph List's own algorithm.
///
/// §9.10.2's second method names two lists — "the Adobe Glyph List **and Adobe Glyph List for New
/// Fonts**" — and the second brings a convention with it that a one-character lookup cannot
/// express: a name may be *composed*, and it may carry a variant suffix.
///
/// ```text
/// f_f_i     → "ffi"    three components, each a name the list knows
/// oacute.sc → "ó"      the small-capital variant of a name the list knows
/// ```
///
/// So the algorithm is the Adobe Glyph List Specification's, in three steps: drop everything from
/// the first period, split on underscores, and map each component. A component the list cannot
/// answer for makes the whole name unanswerable — a half-mapped ligature would be worse than
/// none, because it would put `ff` where the page shows `ffi`.
///
/// **A suffix is dropped rather than read**, and that is the specification's own instruction:
/// the part after the period names a *variant* — `.sc` for small capitals, `.alt` for an
/// alternate — and the variant of a character is that character. It is the same reading ADR 0050
/// gave the suffix in the other direction, where a program's own name for a glyph keeps it.
///
/// `issue15516_reduced.pdf` is the corpus witness: a Minion subset whose eight codes are
/// `/f_f_i`, `/f_i`, `/f_f_l`, `/f_f`, `/f_l`, `/f_t`, `/T_h` and `/f_h`, and which read back as
/// nothing at all until the three-hundred-and-twenty-seventh session.
#[must_use]
pub fn text_for(name: &str) -> Option<String> {
    let stem = name.split('.').next().unwrap_or(name);
    if stem.is_empty() {
        return None;
    }
    // The common case is one component and a name the list knows, which is every ordinary
    // encoding: answered without splitting or allocating a `Vec`.
    if let Some(character) = character_for(stem) {
        return Some(character.to_string());
    }
    let mut out = String::new();
    for component in stem.split('_') {
        out.push(character_for(component)?);
    }
    (!out.is_empty()).then_some(out)
}

/// This is the reverse of the encoding §9.6.5.4 calls "the standard Roman encoding that is
/// used on Mac OS", which it needs in exactly one place: a `TrueType` font with a (1, 0)
/// `cmap` subtable is addressed by *code*, so a glyph name has to be turned back into one.
///
/// It is not [`BaseEncoding::MacRoman`]. Table 113 lists the 16 codes where the two differ
/// — 15 mathematical and symbol glyphs PDF leaves unencoded, and code 219, which is
/// `currency` in PDF's table and `Euro` here. Using PDF's table instead would reach no
/// glyph for those 16 names, which is a smaller error than reaching the wrong one but is
/// still an error.
///
/// The lowest code wins where a name appears twice, which is what makes `space` resolve to
/// 32 rather than to the non-breaking space at 202.
#[must_use]
pub fn mac_os_roman_code(name: &str) -> Option<u8> {
    if let Some((code, _)) = MAC_OS_ROMAN_ADDITIONS
        .iter()
        .find(|(_, added)| *added == name)
    {
        return Some(*code);
    }
    // Code 219 is `currency` in PDF's MacRomanEncoding and `Euro` in Mac OS Roman, so the
    // name it holds in the table below no longer names this code.
    if name == "currency" {
        return None;
    }
    (0..=u8::MAX).find(|code| MAC_ROMAN[usize::from(*code)] == name)
}

/// The entries Table 113 adds to `MacRomanEncoding` to make Mac OS Roman.
///
/// Transcribed from Table 113 of ISO 32000-2. `Euro` at 219 is the one *replacement* —
/// every other entry fills a code PDF's table leaves unencoded — which is why
/// [`mac_os_roman_code`] has to refuse `currency` separately.
static MAC_OS_ROMAN_ADDITIONS: [(u8, &str); 16] = [
    (173, "notequal"),
    (176, "infinity"),
    (178, "lessequal"),
    (179, "greaterequal"),
    (182, "partialdiff"),
    (183, "summation"),
    (184, "product"),
    (185, "pi"),
    (186, "integral"),
    (189, "Omega"),
    (195, "radical"),
    (197, "approxequal"),
    (198, "Delta"),
    (215, "lozenge"),
    (219, "Euro"),
    (240, "apple"),
];

/// The built-in encoding of a standard-14 font that has no Latin character set.
///
/// `Symbol` and `ZapfDingbats` are the two symbolic fonts among the standard 14. Neither
/// can be addressed by a base encoding — their glyphs are not Latin letters and the names
/// in Table D.2 do not reach them — so each carries its own encoding, and these are it.
///
/// A `/Differences` array still applies on top, which is why this is a table rather than
/// something the substitute font resolves internally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SymbolicEncoding {
    /// The `Symbol` font's encoding.
    Symbol,
    /// The `ZapfDingbats` font's encoding.
    ZapfDingbats,
}

impl SymbolicEncoding {
    /// Returns the glyph name a character code selects, empty when unencoded.
    #[must_use]
    pub fn glyph_name(self, code: u8) -> &'static str {
        // A `u8` cannot index outside a 256-element array.
        match self {
            Self::Symbol => SYMBOL[usize::from(code)],
            Self::ZapfDingbats => ZAPF_DINGBATS[usize::from(code)],
        }
    }
}

/// The built-in encoding of the standard-14 `Symbol` font.
/// 
/// Transcribed from Table D.5 of ISO 32000-2. `Symbol` is symbolic: it has no
/// Latin base encoding, so this table *is* its encoding.
#[rustfmt::skip]
static SYMBOL: [&str; 256] = [
    "",               "",               "",               "", // 0
    "",               "",               "",               "", // 4
    "",               "",               "",               "", // 8
    "",               "",               "",               "", // 12
    "",               "",               "",               "", // 16
    "",               "",               "",               "", // 20
    "",               "",               "",               "", // 24
    "",               "",               "",               "", // 28
    "space",          "exclam",         "universal",      "numbersign", // 32
    "existential",    "percent",        "ampersand",      "suchthat", // 36
    "parenleft",      "parenright",     "asteriskmath",   "plus", // 40
    "comma",          "minus",          "period",         "slash", // 44
    "zero",           "one",            "two",            "three", // 48
    "four",           "five",           "six",            "seven", // 52
    "eight",          "nine",           "colon",          "semicolon", // 56
    "less",           "equal",          "greater",        "question", // 60
    "congruent",      "Alpha",          "Beta",           "Chi", // 64
    "Delta",          "Epsilon",        "Phi",            "Gamma", // 68
    "Eta",            "Iota",           "theta1",         "Kappa", // 72
    "Lambda",         "Mu",             "Nu",             "Omicron", // 76
    "Pi",             "Theta",          "Rho",            "Sigma", // 80
    "Tau",            "Upsilon",        "sigma1",         "Omega", // 84
    "Xi",             "Psi",            "Zeta",           "bracketleft", // 88
    "therefore",      "bracketright",   "perpendicular",  "underscore", // 92
    "radicalex",      "alpha",          "beta",           "chi", // 96
    "delta",          "epsilon",        "phi",            "gamma", // 100
    "eta",            "iota",           "phi1",           "kappa", // 104
    "lambda",         "mu",             "nu",             "omicron", // 108
    "pi",             "theta",          "rho",            "sigma", // 112
    "tau",            "upsilon",        "omega1",         "omega", // 116
    "xi",             "psi",            "zeta",           "braceleft", // 120
    "bar",            "braceright",     "similar",        "", // 124
    "",               "",               "",               "", // 128
    "",               "",               "",               "", // 132
    "",               "",               "",               "", // 136
    "",               "",               "",               "", // 140
    "",               "",               "",               "", // 144
    "",               "",               "",               "", // 148
    "",               "",               "",               "", // 152
    "",               "",               "",               "", // 156
    "Euro",           "Upsilon1",       "minute",         "lessequal", // 160
    "fraction",       "infinity",       "florin",         "club", // 164
    "diamond",        "heart",          "spade",          "arrowboth", // 168
    "arrowleft",      "arrowup",        "arrowright",     "arrowdown", // 172
    "degree",         "plusminus",      "second",         "greaterequal", // 176
    "multiply",       "proportional",   "partialdiff",    "bullet", // 180
    "divide",         "notequal",       "equivalence",    "approxequal", // 184
    "ellipsis",       "arrowvertex",    "arrowhorizex",   "carriagereturn", // 188
    "aleph",          "Ifraktur",       "Rfraktur",       "weierstrass", // 192
    "circlemultiply", "circleplus",     "emptyset",       "intersection", // 196
    "union",          "propersuperset", "reflexsuperset", "notsubset", // 200
    "propersubset",   "reflexsubset",   "element",        "notelement", // 204
    "angle",          "gradient",       "registerserif",  "copyrightserif", // 208
    "trademarkserif", "product",        "radical",        "dotmath", // 212
    "logicalnot",     "logicaland",     "logicalor",      "arrowdblboth", // 216
    "arrowdblleft",   "arrowdblup",     "arrowdblright",  "arrowdbldown", // 220
    "lozenge",        "angleleft",      "registersans",   "copyrightsans", // 224
    "trademarksans",  "summation",      "parenlefttp",    "parenleftex", // 228
    "parenleftbt",    "bracketlefttp",  "bracketleftex",  "bracketleftbt", // 232
    "bracelefttp",    "braceleftmid",   "braceleftbt",    "braceex", // 236
    "",               "angleright",     "integral",       "integraltp", // 240
    "integralex",     "integralbt",     "parenrighttp",   "parenrightex", // 244
    "parenrightbt",   "bracketrighttp", "bracketrightex", "bracketrightbt", // 248
    "bracerighttp",   "bracerightmid",  "bracerightbt",   "", // 252
];

/// The built-in encoding of the standard-14 `ZapfDingbats` font.
/// 
/// Transcribed from Table D.6 of ISO 32000-2. The glyph names are positional
/// (`a1`, `a2`, ...) rather than descriptive, which is why they carry no meaning
/// to read.
#[rustfmt::skip]
static ZAPF_DINGBATS: [&str; 256] = [
    "",               "",               "",               "", // 0
    "",               "",               "",               "", // 4
    "",               "",               "",               "", // 8
    "",               "",               "",               "", // 12
    "",               "",               "",               "", // 16
    "",               "",               "",               "", // 20
    "",               "",               "",               "", // 24
    "",               "",               "",               "", // 28
    "space",          "a1",             "a2",             "a202", // 32
    "a3",             "a4",             "a5",             "a119", // 36
    "a118",           "a117",           "a11",            "a12", // 40
    "a13",            "a14",            "a15",            "a16", // 44
    "a105",           "a17",            "a18",            "a19", // 48
    "a20",            "a21",            "a22",            "a23", // 52
    "a24",            "a25",            "a26",            "a27", // 56
    "a28",            "a6",             "a7",             "a8", // 60
    "a9",             "a10",            "a29",            "a30", // 64
    "a31",            "a32",            "a33",            "a34", // 68
    "a35",            "a36",            "a37",            "a38", // 72
    "a39",            "a40",            "a41",            "a42", // 76
    "a43",            "a44",            "a45",            "a46", // 80
    "a47",            "a48",            "a49",            "a50", // 84
    "a51",            "a52",            "a53",            "a54", // 88
    "a55",            "a56",            "a57",            "a58", // 92
    "a59",            "a60",            "a61",            "a62", // 96
    "a63",            "a64",            "a65",            "a66", // 100
    "a67",            "a68",            "a69",            "a70", // 104
    "a71",            "a72",            "a73",            "a74", // 108
    "a203",           "a75",            "a204",           "a76", // 112
    "a77",            "a78",            "a79",            "a81", // 116
    "a82",            "a83",            "a84",            "a97", // 120
    "a98",            "a99",            "a100",           "", // 124
    "",               "",               "",               "", // 128
    "",               "",               "",               "", // 132
    "",               "",               "",               "", // 136
    "",               "",               "",               "", // 140
    "",               "",               "",               "", // 144
    "",               "",               "",               "", // 148
    "",               "",               "",               "", // 152
    "",               "",               "",               "", // 156
    "",               "a101",           "a102",           "a103", // 160
    "a104",           "a106",           "a107",           "a108", // 164
    "a112",           "a111",           "a110",           "a109", // 168
    "a120",           "a121",           "a122",           "a123", // 172
    "a124",           "a125",           "a126",           "a127", // 176
    "a128",           "a129",           "a130",           "a131", // 180
    "a132",           "a133",           "a134",           "a135", // 184
    "a136",           "a137",           "a138",           "a139", // 188
    "a140",           "a141",           "a142",           "a143", // 192
    "a144",           "a145",           "a146",           "a147", // 196
    "a148",           "a149",           "a150",           "a151", // 200
    "a152",           "a153",           "a154",           "a155", // 204
    "a156",           "a157",           "a158",           "a159", // 208
    "a160",           "a161",           "a163",           "a164", // 212
    "a196",           "a165",           "a192",           "a166", // 216
    "a167",           "a168",           "a169",           "a170", // 220
    "a171",           "a172",           "a173",           "a162", // 224
    "a174",           "a175",           "a176",           "a177", // 228
    "a178",           "a179",           "a193",           "a180", // 232
    "a199",           "a181",           "a200",           "a182", // 236
    "",               "a201",           "a183",           "a184", // 240
    "a197",           "a185",           "a194",           "a198", // 244
    "a186",           "a195",           "a187",           "a188", // 248
    "a189",           "a190",           "a191",           "", // 252
];

#[cfg(test)]
mod tests {
    use super::{BaseEncoding, MAC_ROMAN, WIN_ANSI};
    use skrifa::raw::ps::string::STANDARD_STRINGS;

    /// Every glyph name in these tables must be one the CFF specification also knows.
    ///
    /// This is the check that makes the tables trustworthy. They were extracted from
    /// ISO 32000-2 Table D.2; `STANDARD_STRINGS` comes from Adobe's CFF specification by
    /// way of `read-fonts`. The two documents are independent, so a transcription slip, a
    /// stray token picked up from the table's page furniture, or a misspelling has to
    /// survive both to get through here.
    ///
    /// `Euro` is the one legitimate exception, and Table D.2 note 1 says why: it was added
    /// to the Adobe standard Latin set in PDF 1.3, long after the CFF standard strings were
    /// fixed. A font wanting it must supply it under that name in its own charset.
    #[test]
    fn every_glyph_name_is_one_the_cff_specification_knows() {
        let known: std::collections::BTreeSet<&str> = STANDARD_STRINGS.iter().copied().collect();

        for (table, label) in [
            (&WIN_ANSI, "WinAnsiEncoding"),
            (&MAC_ROMAN, "MacRomanEncoding"),
        ] {
            for (code, name) in table.iter().enumerate() {
                assert!(
                    name.is_empty() || *name == "Euro" || known.contains(name),
                    "{label} code {code} names {name:?}, which is not a CFF standard string"
                );
            }
        }
    }

    /// The counts Table D.2 yields, pinned so an accidental edit to the tables is visible.
    ///
    /// `WinAnsiEncoding` encodes 224 codes: the 216 in the table body, plus `space` at 160
    /// and `hyphen` at 173 from notes 6 and 5, plus the six otherwise unused codes above 32
    /// that note 3 sends to `bullet`. `MacRomanEncoding` encodes 208: 207 plus `space` at
    /// 202 from note 6.
    #[test]
    fn the_tables_encode_the_number_of_codes_the_specification_defines() {
        let encoded = |table: &[&str; 256]| table.iter().filter(|name| !name.is_empty()).count();
        assert_eq!(encoded(&WIN_ANSI), 224);
        assert_eq!(encoded(&MAC_ROMAN), 208);
    }

    /// The notes under Table D.2 assign codes the table body leaves blank.
    ///
    /// These are the entries a transcription from memory reliably gets wrong, so they are
    /// asserted by hand rather than left to the counts above.
    #[test]
    fn the_assignments_made_only_in_the_notes_are_present() {
        // Note 6: the code Windows and Mac OS use for a non-breaking space is `space`.
        assert_eq!(BaseEncoding::WinAnsi.glyph_name(160), "space");
        assert_eq!(BaseEncoding::MacRoman.glyph_name(202), "space");
        // Note 5: the code Windows uses for a soft hyphen is `hyphen`.
        assert_eq!(BaseEncoding::WinAnsi.glyph_name(173), "hyphen");
        // Note 3: unused codes above 32 fall to `bullet` — but only in WinAnsi.
        assert_eq!(BaseEncoding::WinAnsi.glyph_name(127), "bullet");
        assert_eq!(BaseEncoding::MacRoman.glyph_name(127), "");
        // Note 1: PDF kept code 219 as `currency` where Mac OS Roman moved to the euro.
        assert_eq!(BaseEncoding::MacRoman.glyph_name(219), "currency");
        assert_eq!(BaseEncoding::WinAnsi.glyph_name(128), "Euro");
    }

    /// The three encodings must disagree exactly where the specification says they do.
    #[test]
    fn the_encodings_differ_where_they_are_known_to_differ() {
        // Code 39 and 96 are the classic divergence: typographic quotes in
        // StandardEncoding, straight ASCII ones everywhere else.
        assert_eq!(BaseEncoding::Standard.glyph_name(39), "quoteright");
        assert_eq!(BaseEncoding::WinAnsi.glyph_name(39), "quotesingle");
        assert_eq!(BaseEncoding::MacRoman.glyph_name(39), "quotesingle");
        assert_eq!(BaseEncoding::Standard.glyph_name(96), "quoteleft");
        assert_eq!(BaseEncoding::WinAnsi.glyph_name(96), "grave");

        // `read-fonts` must agree with this module about what "unencoded" means.
        assert_eq!(BaseEncoding::Standard.glyph_name(0), "");
        assert_eq!(BaseEncoding::Standard.glyph_name(128), "");
    }

    /// Table 113's own description of itself, checked against the table it modifies.
    ///
    /// The text says Mac OS Roman is `MacRomanEncoding` "with the addition of 15 entries
    /// and the replacement of the currency glyph with the Euro glyph". Table 113 lists 16
    /// rows, so exactly one of them must land on a code `MacRomanEncoding` already uses,
    /// and that code must be the one holding `currency`. Two independent statements in the
    /// same subclause, and a transcription slip breaks their agreement.
    #[test]
    fn table_113_adds_fifteen_codes_and_replaces_one() {
        let occupied: Vec<_> = super::MAC_OS_ROMAN_ADDITIONS
            .iter()
            .filter(|(code, _)| !MAC_ROMAN[usize::from(*code)].is_empty())
            .collect();
        assert_eq!(occupied.len(), 1, "expected exactly one replacement");
        assert_eq!(*occupied[0], (219, "Euro"));
        assert_eq!(MAC_ROMAN[219], "currency");
    }

    /// The reverse mapping §9.6.5.4 needs for a (1, 0) `cmap` subtable.
    #[test]
    fn a_glyph_name_resolves_to_its_mac_os_roman_code() {
        // The Latin range is ASCII, which is the case that matters for real documents.
        assert_eq!(super::mac_os_roman_code("H"), Some(72));
        assert_eq!(super::mac_os_roman_code("space"), Some(32));
        // Table 113's additions are reachable and PDF's `currency` is not, because the
        // code it named belongs to `Euro` in this encoding.
        assert_eq!(super::mac_os_roman_code("Euro"), Some(219));
        assert_eq!(super::mac_os_roman_code("notequal"), Some(173));
        assert_eq!(super::mac_os_roman_code("currency"), None);
        // A name no encoding lists has no code, rather than a plausible one.
        assert_eq!(super::mac_os_roman_code("gid2436"), None);
    }

    /// §9.10.2's second method names two lists, and the second brings an algorithm.
    #[test]
    fn a_composed_glyph_name_reads_back_as_its_components() {
        // `issue15516_reduced.pdf`'s eight codes, which read back as nothing until the
        // three-hundred-and-twenty-seventh session.
        assert_eq!(super::text_for("f_f_i").as_deref(), Some("ffi"));
        assert_eq!(super::text_for("T_h").as_deref(), Some("Th"));
        // A variant suffix names the same character: the part after the first period is
        // dropped, which is the Adobe Glyph List Specification's own instruction.
        assert_eq!(super::text_for("oacute.sc").as_deref(), Some("\u{f3}"));
        assert_eq!(super::text_for("a.alt.two").as_deref(), Some("a"));
        // The ordinary case is unchanged, and the `uniXXXX` form the list itself defines.
        assert_eq!(super::text_for("colon").as_deref(), Some(":"));
        assert_eq!(super::text_for("uni0041").as_deref(), Some("A"));
        // A component the list cannot answer for makes the *whole* name unanswerable: half a
        // ligature would put `ff` where the page shows `ffi`.
        assert_eq!(super::text_for("f_f_notaglyph"), None);
        assert_eq!(super::text_for("a97"), None);
        assert_eq!(super::text_for(""), None);
        assert_eq!(super::text_for(".notdef"), None);
    }

    #[test]
    fn unknown_and_unimplemented_encoding_names_are_refused() {
        assert_eq!(
            BaseEncoding::by_name(b"WinAnsiEncoding"),
            Some(BaseEncoding::WinAnsi)
        );
        // Valid in PDF, but this crate has no table for it, so it must not be guessed at.
        assert_eq!(BaseEncoding::by_name(b"MacExpertEncoding"), None);
        assert_eq!(BaseEncoding::by_name(b"NotAnEncoding"), None);
    }
}
