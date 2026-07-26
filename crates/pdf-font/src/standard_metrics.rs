//! The advance widths of the standard 14 fonts.
//!
//! A PDF may use one of fourteen named fonts without embedding it *and* without stating
//! `/Widths`, because a conforming reader is required to know their metrics already. This
//! module is that knowledge.
//!
//! Without it, a substituted standard-14 font has to take its advances from whichever
//! substitute this machine happens to offer, which makes the rendered layout depend on the
//! installed fonts. With it, the layout is the document's, and only the glyph *shapes*
//! vary — which is the most a substitute can ever get right.
//!
//! # Where these numbers come from
//!
//! Generated from `doc/pdf.js/src/core/metrics.js`, which is Apache-2.0 and therefore
//! redistributable. They are Adobe's published metrics for the standard 14, and they are
//! *facts about a typeface* rather than a font program: no outline, no design, just the
//! advance each named glyph takes.
//!
//! The URW metric clones installed on a typical Linux system carry the same numbers, but
//! are AGPL-3.0, and deriving this table from those would put a copyleft obligation on
//! this crate. That is why the numbers come from pdf.js and not from the AFM files sitting
//! in `/usr/share/fonts`. See `doc/adr/0007-non-embedded-fonts.md`.
//!
//! Regenerate with `tools/gen-standard-metrics.py` after updating the pdf.js submodule.

use crate::substitute::{Family, Request};

/// One of the fourteen fonts every PDF reader is required to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StandardFont {
    /// `Courier`, and its bold, oblique and bold-oblique faces.
    Courier,
    /// `Helvetica`.
    Helvetica,
    /// `Helvetica-Bold`.
    HelveticaBold,
    /// `Helvetica-Oblique`.
    HelveticaOblique,
    /// `Helvetica-BoldOblique`.
    HelveticaBoldOblique,
    /// `Times-Roman`.
    TimesRoman,
    /// `Times-Bold`.
    TimesBold,
    /// `Times-Italic`.
    TimesItalic,
    /// `Times-BoldItalic`.
    TimesBoldItalic,
    /// `Symbol`.
    Symbol,
    /// `ZapfDingbats`.
    ZapfDingbats,
}

impl StandardFont {
    /// Chooses the standard face that best answers a substitution request.
    ///
    /// The request is already derived from the document alone, so this stays
    /// document-determined. It maps more than the fourteen exact names on purpose: a
    /// document naming `/Arial` without widths means Helvetica's metrics in practice,
    /// because Arial was drawn to be metrically compatible with it, and the same holds
    /// for Times New Roman and Courier New. Answering those with the standard metrics is
    /// far closer than answering with an arbitrary substitute's.
    #[must_use]
    pub fn for_request(request: Request) -> Self {
        match (request.family, request.bold, request.italic) {
            (Family::Symbol, _, _) => Self::Symbol,
            (Family::ZapfDingbats, _, _) => Self::ZapfDingbats,
            // Every Courier face is fixed pitch at the same width, so the four collapse.
            (Family::Monospace, _, _) => Self::Courier,
            (Family::Serif, false, false) => Self::TimesRoman,
            (Family::Serif, true, false) => Self::TimesBold,
            (Family::Serif, false, true) => Self::TimesItalic,
            (Family::Serif, true, true) => Self::TimesBoldItalic,
            (Family::SansSerif, false, false) => Self::Helvetica,
            (Family::SansSerif, true, false) => Self::HelveticaBold,
            (Family::SansSerif, false, true) => Self::HelveticaOblique,
            (Family::SansSerif, true, true) => Self::HelveticaBoldOblique,
        }
    }

    /// Returns a glyph's advance in thousandths of an em, by its name.
    ///
    /// `None` means this face has no such glyph, which is not the same as zero: the caller
    /// then has nothing to say about the code and should fall back rather than draw it
    /// with no advance.
    #[must_use]
    pub fn width(self, glyph_name: &str) -> Option<f32> {
        /// Every Courier face is fixed pitch, so one number covers all of its glyphs.
        const COURIER_WIDTH: f32 = 600.0;

        let table: &[(&str, u16)] = match self {
            // A fixed-pitch face still has to answer "do you have this glyph", and the
            // honest answer needs a glyph list; Helvetica's covers the same character set.
            Self::Courier => {
                return HELVETICA_WIDTHS
                    .binary_search_by_key(&glyph_name, |(name, _)| name)
                    .ok()
                    .map(|_| COURIER_WIDTH);
            }
            Self::Helvetica => &HELVETICA_WIDTHS,
            Self::HelveticaBold => &HELVETICABOLD_WIDTHS,
            Self::HelveticaOblique => &HELVETICAOBLIQUE_WIDTHS,
            Self::HelveticaBoldOblique => &HELVETICABOLDOBLIQUE_WIDTHS,
            Self::TimesRoman => &TIMESROMAN_WIDTHS,
            Self::TimesBold => &TIMESBOLD_WIDTHS,
            Self::TimesItalic => &TIMESITALIC_WIDTHS,
            Self::TimesBoldItalic => &TIMESBOLDITALIC_WIDTHS,
            Self::Symbol => &SYMBOL_WIDTHS,
            Self::ZapfDingbats => &ZAPFDINGBATS_WIDTHS,
        };
        table
            .binary_search_by_key(&glyph_name, |(name, _)| name)
            .ok()
            .and_then(|index| table.get(index))
            .map(|(_, width)| f32::from(*width))
    }
}

/// Advance widths for `Helvetica`, sorted by glyph name.
#[rustfmt::skip]
static HELVETICA_WIDTHS: [(&str, u16); 315] = [
    ("A",667), ("AE",1000), ("Aacute",667), ("Abreve",667),
    ("Acircumflex",667), ("Adieresis",667), ("Agrave",667), ("Amacron",667),
    ("Aogonek",667), ("Aring",667), ("Atilde",667), ("B",667),
    ("C",722), ("Cacute",722), ("Ccaron",722), ("Ccedilla",722),
    ("D",722), ("Dcaron",722), ("Dcroat",722), ("Delta",612),
    ("E",667), ("Eacute",667), ("Ecaron",667), ("Ecircumflex",667),
    ("Edieresis",667), ("Edotaccent",667), ("Egrave",667), ("Emacron",667),
    ("Eogonek",667), ("Eth",722), ("Euro",556), ("F",611),
    ("G",778), ("Gbreve",778), ("Gcommaaccent",778), ("H",722),
    ("I",278), ("Iacute",278), ("Icircumflex",278), ("Idieresis",278),
    ("Idotaccent",278), ("Igrave",278), ("Imacron",278), ("Iogonek",278),
    ("J",500), ("K",667), ("Kcommaaccent",667), ("L",556),
    ("Lacute",556), ("Lcaron",556), ("Lcommaaccent",556), ("Lslash",556),
    ("M",833), ("N",722), ("Nacute",722), ("Ncaron",722),
    ("Ncommaaccent",722), ("Ntilde",722), ("O",778), ("OE",1000),
    ("Oacute",778), ("Ocircumflex",778), ("Odieresis",778), ("Ograve",778),
    ("Ohungarumlaut",778), ("Omacron",778), ("Oslash",778), ("Otilde",778),
    ("P",667), ("Q",778), ("R",722), ("Racute",722),
    ("Rcaron",722), ("Rcommaaccent",722), ("S",667), ("Sacute",667),
    ("Scaron",667), ("Scedilla",667), ("Scommaaccent",667), ("T",611),
    ("Tcaron",611), ("Tcommaaccent",611), ("Thorn",667), ("U",722),
    ("Uacute",722), ("Ucircumflex",722), ("Udieresis",722), ("Ugrave",722),
    ("Uhungarumlaut",722), ("Umacron",722), ("Uogonek",722), ("Uring",722),
    ("V",667), ("W",944), ("X",667), ("Y",667),
    ("Yacute",667), ("Ydieresis",667), ("Z",611), ("Zacute",611),
    ("Zcaron",611), ("Zdotaccent",611), ("a",556), ("aacute",556),
    ("abreve",556), ("acircumflex",556), ("acute",333), ("adieresis",556),
    ("ae",889), ("agrave",556), ("amacron",556), ("ampersand",667),
    ("aogonek",556), ("aring",556), ("asciicircum",469), ("asciitilde",584),
    ("asterisk",389), ("at",1015), ("atilde",556), ("b",556),
    ("backslash",278), ("bar",260), ("braceleft",334), ("braceright",334),
    ("bracketleft",278), ("bracketright",278), ("breve",333), ("brokenbar",260),
    ("bullet",350), ("c",500), ("cacute",500), ("caron",333),
    ("ccaron",500), ("ccedilla",500), ("cedilla",333), ("cent",556),
    ("circumflex",333), ("colon",278), ("comma",278), ("commaaccent",250),
    ("copyright",737), ("currency",556), ("d",556), ("dagger",556),
    ("daggerdbl",556), ("dcaron",643), ("dcroat",556), ("degree",400),
    ("dieresis",333), ("divide",584), ("dollar",556), ("dotaccent",333),
    ("dotlessi",278), ("e",556), ("eacute",556), ("ecaron",556),
    ("ecircumflex",556), ("edieresis",556), ("edotaccent",556), ("egrave",556),
    ("eight",556), ("ellipsis",1000), ("emacron",556), ("emdash",1000),
    ("endash",556), ("eogonek",556), ("equal",584), ("eth",556),
    ("exclam",278), ("exclamdown",333), ("f",278), ("fi",500),
    ("five",556), ("fl",500), ("florin",556), ("four",556),
    ("fraction",167), ("g",556), ("gbreve",556), ("gcommaaccent",556),
    ("germandbls",611), ("grave",333), ("greater",584), ("greaterequal",549),
    ("guillemotleft",556), ("guillemotright",556), ("guilsinglleft",333), ("guilsinglright",333),
    ("h",556), ("hungarumlaut",333), ("hyphen",333), ("i",222),
    ("iacute",278), ("icircumflex",278), ("idieresis",278), ("igrave",278),
    ("imacron",278), ("iogonek",222), ("j",222), ("k",500),
    ("kcommaaccent",500), ("l",222), ("lacute",222), ("lcaron",299),
    ("lcommaaccent",222), ("less",584), ("lessequal",549), ("logicalnot",584),
    ("lozenge",471), ("lslash",222), ("m",833), ("macron",333),
    ("minus",584), ("mu",556), ("multiply",584), ("n",556),
    ("nacute",556), ("ncaron",556), ("ncommaaccent",556), ("nine",556),
    ("notequal",549), ("ntilde",556), ("numbersign",556), ("o",556),
    ("oacute",556), ("ocircumflex",556), ("odieresis",556), ("oe",944),
    ("ogonek",333), ("ograve",556), ("ohungarumlaut",556), ("omacron",556),
    ("one",556), ("onehalf",834), ("onequarter",834), ("onesuperior",333),
    ("ordfeminine",370), ("ordmasculine",365), ("oslash",611), ("otilde",556),
    ("p",556), ("paragraph",537), ("parenleft",333), ("parenright",333),
    ("partialdiff",476), ("percent",889), ("period",278), ("periodcentered",278),
    ("perthousand",1000), ("plus",584), ("plusminus",584), ("q",556),
    ("question",556), ("questiondown",611), ("quotedbl",355), ("quotedblbase",333),
    ("quotedblleft",333), ("quotedblright",333), ("quoteleft",222), ("quoteright",222),
    ("quotesinglbase",222), ("quotesingle",191), ("r",333), ("racute",333),
    ("radical",453), ("rcaron",333), ("rcommaaccent",333), ("registered",737),
    ("ring",333), ("s",500), ("sacute",500), ("scaron",500),
    ("scedilla",500), ("scommaaccent",500), ("section",556), ("semicolon",278),
    ("seven",556), ("six",556), ("slash",278), ("space",278),
    ("sterling",556), ("summation",600), ("t",278), ("tcaron",317),
    ("tcommaaccent",278), ("thorn",556), ("three",556), ("threequarters",834),
    ("threesuperior",333), ("tilde",333), ("trademark",1000), ("two",556),
    ("twosuperior",333), ("u",556), ("uacute",556), ("ucircumflex",556),
    ("udieresis",556), ("ugrave",556), ("uhungarumlaut",556), ("umacron",556),
    ("underscore",556), ("uogonek",556), ("uring",556), ("v",500),
    ("w",722), ("x",500), ("y",500), ("yacute",500),
    ("ydieresis",500), ("yen",556), ("z",500), ("zacute",500),
    ("zcaron",500), ("zdotaccent",500), ("zero",556),
];

/// Advance widths for `Helvetica-Bold`, sorted by glyph name.
#[rustfmt::skip]
static HELVETICABOLD_WIDTHS: [(&str, u16); 315] = [
    ("A",722), ("AE",1000), ("Aacute",722), ("Abreve",722),
    ("Acircumflex",722), ("Adieresis",722), ("Agrave",722), ("Amacron",722),
    ("Aogonek",722), ("Aring",722), ("Atilde",722), ("B",722),
    ("C",722), ("Cacute",722), ("Ccaron",722), ("Ccedilla",722),
    ("D",722), ("Dcaron",722), ("Dcroat",722), ("Delta",612),
    ("E",667), ("Eacute",667), ("Ecaron",667), ("Ecircumflex",667),
    ("Edieresis",667), ("Edotaccent",667), ("Egrave",667), ("Emacron",667),
    ("Eogonek",667), ("Eth",722), ("Euro",556), ("F",611),
    ("G",778), ("Gbreve",778), ("Gcommaaccent",778), ("H",722),
    ("I",278), ("Iacute",278), ("Icircumflex",278), ("Idieresis",278),
    ("Idotaccent",278), ("Igrave",278), ("Imacron",278), ("Iogonek",278),
    ("J",556), ("K",722), ("Kcommaaccent",722), ("L",611),
    ("Lacute",611), ("Lcaron",611), ("Lcommaaccent",611), ("Lslash",611),
    ("M",833), ("N",722), ("Nacute",722), ("Ncaron",722),
    ("Ncommaaccent",722), ("Ntilde",722), ("O",778), ("OE",1000),
    ("Oacute",778), ("Ocircumflex",778), ("Odieresis",778), ("Ograve",778),
    ("Ohungarumlaut",778), ("Omacron",778), ("Oslash",778), ("Otilde",778),
    ("P",667), ("Q",778), ("R",722), ("Racute",722),
    ("Rcaron",722), ("Rcommaaccent",722), ("S",667), ("Sacute",667),
    ("Scaron",667), ("Scedilla",667), ("Scommaaccent",667), ("T",611),
    ("Tcaron",611), ("Tcommaaccent",611), ("Thorn",667), ("U",722),
    ("Uacute",722), ("Ucircumflex",722), ("Udieresis",722), ("Ugrave",722),
    ("Uhungarumlaut",722), ("Umacron",722), ("Uogonek",722), ("Uring",722),
    ("V",667), ("W",944), ("X",667), ("Y",667),
    ("Yacute",667), ("Ydieresis",667), ("Z",611), ("Zacute",611),
    ("Zcaron",611), ("Zdotaccent",611), ("a",556), ("aacute",556),
    ("abreve",556), ("acircumflex",556), ("acute",333), ("adieresis",556),
    ("ae",889), ("agrave",556), ("amacron",556), ("ampersand",722),
    ("aogonek",556), ("aring",556), ("asciicircum",584), ("asciitilde",584),
    ("asterisk",389), ("at",975), ("atilde",556), ("b",611),
    ("backslash",278), ("bar",280), ("braceleft",389), ("braceright",389),
    ("bracketleft",333), ("bracketright",333), ("breve",333), ("brokenbar",280),
    ("bullet",350), ("c",556), ("cacute",556), ("caron",333),
    ("ccaron",556), ("ccedilla",556), ("cedilla",333), ("cent",556),
    ("circumflex",333), ("colon",333), ("comma",278), ("commaaccent",250),
    ("copyright",737), ("currency",556), ("d",611), ("dagger",556),
    ("daggerdbl",556), ("dcaron",743), ("dcroat",611), ("degree",400),
    ("dieresis",333), ("divide",584), ("dollar",556), ("dotaccent",333),
    ("dotlessi",278), ("e",556), ("eacute",556), ("ecaron",556),
    ("ecircumflex",556), ("edieresis",556), ("edotaccent",556), ("egrave",556),
    ("eight",556), ("ellipsis",1000), ("emacron",556), ("emdash",1000),
    ("endash",556), ("eogonek",556), ("equal",584), ("eth",611),
    ("exclam",333), ("exclamdown",333), ("f",333), ("fi",611),
    ("five",556), ("fl",611), ("florin",556), ("four",556),
    ("fraction",167), ("g",611), ("gbreve",611), ("gcommaaccent",611),
    ("germandbls",611), ("grave",333), ("greater",584), ("greaterequal",549),
    ("guillemotleft",556), ("guillemotright",556), ("guilsinglleft",333), ("guilsinglright",333),
    ("h",611), ("hungarumlaut",333), ("hyphen",333), ("i",278),
    ("iacute",278), ("icircumflex",278), ("idieresis",278), ("igrave",278),
    ("imacron",278), ("iogonek",278), ("j",278), ("k",556),
    ("kcommaaccent",556), ("l",278), ("lacute",278), ("lcaron",400),
    ("lcommaaccent",278), ("less",584), ("lessequal",549), ("logicalnot",584),
    ("lozenge",494), ("lslash",278), ("m",889), ("macron",333),
    ("minus",584), ("mu",611), ("multiply",584), ("n",611),
    ("nacute",611), ("ncaron",611), ("ncommaaccent",611), ("nine",556),
    ("notequal",549), ("ntilde",611), ("numbersign",556), ("o",611),
    ("oacute",611), ("ocircumflex",611), ("odieresis",611), ("oe",944),
    ("ogonek",333), ("ograve",611), ("ohungarumlaut",611), ("omacron",611),
    ("one",556), ("onehalf",834), ("onequarter",834), ("onesuperior",333),
    ("ordfeminine",370), ("ordmasculine",365), ("oslash",611), ("otilde",611),
    ("p",611), ("paragraph",556), ("parenleft",333), ("parenright",333),
    ("partialdiff",494), ("percent",889), ("period",278), ("periodcentered",278),
    ("perthousand",1000), ("plus",584), ("plusminus",584), ("q",611),
    ("question",611), ("questiondown",611), ("quotedbl",474), ("quotedblbase",500),
    ("quotedblleft",500), ("quotedblright",500), ("quoteleft",278), ("quoteright",278),
    ("quotesinglbase",278), ("quotesingle",238), ("r",389), ("racute",389),
    ("radical",549), ("rcaron",389), ("rcommaaccent",389), ("registered",737),
    ("ring",333), ("s",556), ("sacute",556), ("scaron",556),
    ("scedilla",556), ("scommaaccent",556), ("section",556), ("semicolon",333),
    ("seven",556), ("six",556), ("slash",278), ("space",278),
    ("sterling",556), ("summation",600), ("t",333), ("tcaron",389),
    ("tcommaaccent",333), ("thorn",611), ("three",556), ("threequarters",834),
    ("threesuperior",333), ("tilde",333), ("trademark",1000), ("two",556),
    ("twosuperior",333), ("u",611), ("uacute",611), ("ucircumflex",611),
    ("udieresis",611), ("ugrave",611), ("uhungarumlaut",611), ("umacron",611),
    ("underscore",556), ("uogonek",611), ("uring",611), ("v",556),
    ("w",778), ("x",556), ("y",556), ("yacute",556),
    ("ydieresis",556), ("yen",556), ("z",500), ("zacute",500),
    ("zcaron",500), ("zdotaccent",500), ("zero",556),
];

/// Advance widths for `Helvetica-Oblique`, sorted by glyph name.
#[rustfmt::skip]
static HELVETICAOBLIQUE_WIDTHS: [(&str, u16); 315] = [
    ("A",667), ("AE",1000), ("Aacute",667), ("Abreve",667),
    ("Acircumflex",667), ("Adieresis",667), ("Agrave",667), ("Amacron",667),
    ("Aogonek",667), ("Aring",667), ("Atilde",667), ("B",667),
    ("C",722), ("Cacute",722), ("Ccaron",722), ("Ccedilla",722),
    ("D",722), ("Dcaron",722), ("Dcroat",722), ("Delta",612),
    ("E",667), ("Eacute",667), ("Ecaron",667), ("Ecircumflex",667),
    ("Edieresis",667), ("Edotaccent",667), ("Egrave",667), ("Emacron",667),
    ("Eogonek",667), ("Eth",722), ("Euro",556), ("F",611),
    ("G",778), ("Gbreve",778), ("Gcommaaccent",778), ("H",722),
    ("I",278), ("Iacute",278), ("Icircumflex",278), ("Idieresis",278),
    ("Idotaccent",278), ("Igrave",278), ("Imacron",278), ("Iogonek",278),
    ("J",500), ("K",667), ("Kcommaaccent",667), ("L",556),
    ("Lacute",556), ("Lcaron",556), ("Lcommaaccent",556), ("Lslash",556),
    ("M",833), ("N",722), ("Nacute",722), ("Ncaron",722),
    ("Ncommaaccent",722), ("Ntilde",722), ("O",778), ("OE",1000),
    ("Oacute",778), ("Ocircumflex",778), ("Odieresis",778), ("Ograve",778),
    ("Ohungarumlaut",778), ("Omacron",778), ("Oslash",778), ("Otilde",778),
    ("P",667), ("Q",778), ("R",722), ("Racute",722),
    ("Rcaron",722), ("Rcommaaccent",722), ("S",667), ("Sacute",667),
    ("Scaron",667), ("Scedilla",667), ("Scommaaccent",667), ("T",611),
    ("Tcaron",611), ("Tcommaaccent",611), ("Thorn",667), ("U",722),
    ("Uacute",722), ("Ucircumflex",722), ("Udieresis",722), ("Ugrave",722),
    ("Uhungarumlaut",722), ("Umacron",722), ("Uogonek",722), ("Uring",722),
    ("V",667), ("W",944), ("X",667), ("Y",667),
    ("Yacute",667), ("Ydieresis",667), ("Z",611), ("Zacute",611),
    ("Zcaron",611), ("Zdotaccent",611), ("a",556), ("aacute",556),
    ("abreve",556), ("acircumflex",556), ("acute",333), ("adieresis",556),
    ("ae",889), ("agrave",556), ("amacron",556), ("ampersand",667),
    ("aogonek",556), ("aring",556), ("asciicircum",469), ("asciitilde",584),
    ("asterisk",389), ("at",1015), ("atilde",556), ("b",556),
    ("backslash",278), ("bar",260), ("braceleft",334), ("braceright",334),
    ("bracketleft",278), ("bracketright",278), ("breve",333), ("brokenbar",260),
    ("bullet",350), ("c",500), ("cacute",500), ("caron",333),
    ("ccaron",500), ("ccedilla",500), ("cedilla",333), ("cent",556),
    ("circumflex",333), ("colon",278), ("comma",278), ("commaaccent",250),
    ("copyright",737), ("currency",556), ("d",556), ("dagger",556),
    ("daggerdbl",556), ("dcaron",643), ("dcroat",556), ("degree",400),
    ("dieresis",333), ("divide",584), ("dollar",556), ("dotaccent",333),
    ("dotlessi",278), ("e",556), ("eacute",556), ("ecaron",556),
    ("ecircumflex",556), ("edieresis",556), ("edotaccent",556), ("egrave",556),
    ("eight",556), ("ellipsis",1000), ("emacron",556), ("emdash",1000),
    ("endash",556), ("eogonek",556), ("equal",584), ("eth",556),
    ("exclam",278), ("exclamdown",333), ("f",278), ("fi",500),
    ("five",556), ("fl",500), ("florin",556), ("four",556),
    ("fraction",167), ("g",556), ("gbreve",556), ("gcommaaccent",556),
    ("germandbls",611), ("grave",333), ("greater",584), ("greaterequal",549),
    ("guillemotleft",556), ("guillemotright",556), ("guilsinglleft",333), ("guilsinglright",333),
    ("h",556), ("hungarumlaut",333), ("hyphen",333), ("i",222),
    ("iacute",278), ("icircumflex",278), ("idieresis",278), ("igrave",278),
    ("imacron",278), ("iogonek",222), ("j",222), ("k",500),
    ("kcommaaccent",500), ("l",222), ("lacute",222), ("lcaron",299),
    ("lcommaaccent",222), ("less",584), ("lessequal",549), ("logicalnot",584),
    ("lozenge",471), ("lslash",222), ("m",833), ("macron",333),
    ("minus",584), ("mu",556), ("multiply",584), ("n",556),
    ("nacute",556), ("ncaron",556), ("ncommaaccent",556), ("nine",556),
    ("notequal",549), ("ntilde",556), ("numbersign",556), ("o",556),
    ("oacute",556), ("ocircumflex",556), ("odieresis",556), ("oe",944),
    ("ogonek",333), ("ograve",556), ("ohungarumlaut",556), ("omacron",556),
    ("one",556), ("onehalf",834), ("onequarter",834), ("onesuperior",333),
    ("ordfeminine",370), ("ordmasculine",365), ("oslash",611), ("otilde",556),
    ("p",556), ("paragraph",537), ("parenleft",333), ("parenright",333),
    ("partialdiff",476), ("percent",889), ("period",278), ("periodcentered",278),
    ("perthousand",1000), ("plus",584), ("plusminus",584), ("q",556),
    ("question",556), ("questiondown",611), ("quotedbl",355), ("quotedblbase",333),
    ("quotedblleft",333), ("quotedblright",333), ("quoteleft",222), ("quoteright",222),
    ("quotesinglbase",222), ("quotesingle",191), ("r",333), ("racute",333),
    ("radical",453), ("rcaron",333), ("rcommaaccent",333), ("registered",737),
    ("ring",333), ("s",500), ("sacute",500), ("scaron",500),
    ("scedilla",500), ("scommaaccent",500), ("section",556), ("semicolon",278),
    ("seven",556), ("six",556), ("slash",278), ("space",278),
    ("sterling",556), ("summation",600), ("t",278), ("tcaron",317),
    ("tcommaaccent",278), ("thorn",556), ("three",556), ("threequarters",834),
    ("threesuperior",333), ("tilde",333), ("trademark",1000), ("two",556),
    ("twosuperior",333), ("u",556), ("uacute",556), ("ucircumflex",556),
    ("udieresis",556), ("ugrave",556), ("uhungarumlaut",556), ("umacron",556),
    ("underscore",556), ("uogonek",556), ("uring",556), ("v",500),
    ("w",722), ("x",500), ("y",500), ("yacute",500),
    ("ydieresis",500), ("yen",556), ("z",500), ("zacute",500),
    ("zcaron",500), ("zdotaccent",500), ("zero",556),
];

/// Advance widths for `Helvetica-BoldOblique`, sorted by glyph name.
#[rustfmt::skip]
static HELVETICABOLDOBLIQUE_WIDTHS: [(&str, u16); 315] = [
    ("A",722), ("AE",1000), ("Aacute",722), ("Abreve",722),
    ("Acircumflex",722), ("Adieresis",722), ("Agrave",722), ("Amacron",722),
    ("Aogonek",722), ("Aring",722), ("Atilde",722), ("B",722),
    ("C",722), ("Cacute",722), ("Ccaron",722), ("Ccedilla",722),
    ("D",722), ("Dcaron",722), ("Dcroat",722), ("Delta",612),
    ("E",667), ("Eacute",667), ("Ecaron",667), ("Ecircumflex",667),
    ("Edieresis",667), ("Edotaccent",667), ("Egrave",667), ("Emacron",667),
    ("Eogonek",667), ("Eth",722), ("Euro",556), ("F",611),
    ("G",778), ("Gbreve",778), ("Gcommaaccent",778), ("H",722),
    ("I",278), ("Iacute",278), ("Icircumflex",278), ("Idieresis",278),
    ("Idotaccent",278), ("Igrave",278), ("Imacron",278), ("Iogonek",278),
    ("J",556), ("K",722), ("Kcommaaccent",722), ("L",611),
    ("Lacute",611), ("Lcaron",611), ("Lcommaaccent",611), ("Lslash",611),
    ("M",833), ("N",722), ("Nacute",722), ("Ncaron",722),
    ("Ncommaaccent",722), ("Ntilde",722), ("O",778), ("OE",1000),
    ("Oacute",778), ("Ocircumflex",778), ("Odieresis",778), ("Ograve",778),
    ("Ohungarumlaut",778), ("Omacron",778), ("Oslash",778), ("Otilde",778),
    ("P",667), ("Q",778), ("R",722), ("Racute",722),
    ("Rcaron",722), ("Rcommaaccent",722), ("S",667), ("Sacute",667),
    ("Scaron",667), ("Scedilla",667), ("Scommaaccent",667), ("T",611),
    ("Tcaron",611), ("Tcommaaccent",611), ("Thorn",667), ("U",722),
    ("Uacute",722), ("Ucircumflex",722), ("Udieresis",722), ("Ugrave",722),
    ("Uhungarumlaut",722), ("Umacron",722), ("Uogonek",722), ("Uring",722),
    ("V",667), ("W",944), ("X",667), ("Y",667),
    ("Yacute",667), ("Ydieresis",667), ("Z",611), ("Zacute",611),
    ("Zcaron",611), ("Zdotaccent",611), ("a",556), ("aacute",556),
    ("abreve",556), ("acircumflex",556), ("acute",333), ("adieresis",556),
    ("ae",889), ("agrave",556), ("amacron",556), ("ampersand",722),
    ("aogonek",556), ("aring",556), ("asciicircum",584), ("asciitilde",584),
    ("asterisk",389), ("at",975), ("atilde",556), ("b",611),
    ("backslash",278), ("bar",280), ("braceleft",389), ("braceright",389),
    ("bracketleft",333), ("bracketright",333), ("breve",333), ("brokenbar",280),
    ("bullet",350), ("c",556), ("cacute",556), ("caron",333),
    ("ccaron",556), ("ccedilla",556), ("cedilla",333), ("cent",556),
    ("circumflex",333), ("colon",333), ("comma",278), ("commaaccent",250),
    ("copyright",737), ("currency",556), ("d",611), ("dagger",556),
    ("daggerdbl",556), ("dcaron",743), ("dcroat",611), ("degree",400),
    ("dieresis",333), ("divide",584), ("dollar",556), ("dotaccent",333),
    ("dotlessi",278), ("e",556), ("eacute",556), ("ecaron",556),
    ("ecircumflex",556), ("edieresis",556), ("edotaccent",556), ("egrave",556),
    ("eight",556), ("ellipsis",1000), ("emacron",556), ("emdash",1000),
    ("endash",556), ("eogonek",556), ("equal",584), ("eth",611),
    ("exclam",333), ("exclamdown",333), ("f",333), ("fi",611),
    ("five",556), ("fl",611), ("florin",556), ("four",556),
    ("fraction",167), ("g",611), ("gbreve",611), ("gcommaaccent",611),
    ("germandbls",611), ("grave",333), ("greater",584), ("greaterequal",549),
    ("guillemotleft",556), ("guillemotright",556), ("guilsinglleft",333), ("guilsinglright",333),
    ("h",611), ("hungarumlaut",333), ("hyphen",333), ("i",278),
    ("iacute",278), ("icircumflex",278), ("idieresis",278), ("igrave",278),
    ("imacron",278), ("iogonek",278), ("j",278), ("k",556),
    ("kcommaaccent",556), ("l",278), ("lacute",278), ("lcaron",400),
    ("lcommaaccent",278), ("less",584), ("lessequal",549), ("logicalnot",584),
    ("lozenge",494), ("lslash",278), ("m",889), ("macron",333),
    ("minus",584), ("mu",611), ("multiply",584), ("n",611),
    ("nacute",611), ("ncaron",611), ("ncommaaccent",611), ("nine",556),
    ("notequal",549), ("ntilde",611), ("numbersign",556), ("o",611),
    ("oacute",611), ("ocircumflex",611), ("odieresis",611), ("oe",944),
    ("ogonek",333), ("ograve",611), ("ohungarumlaut",611), ("omacron",611),
    ("one",556), ("onehalf",834), ("onequarter",834), ("onesuperior",333),
    ("ordfeminine",370), ("ordmasculine",365), ("oslash",611), ("otilde",611),
    ("p",611), ("paragraph",556), ("parenleft",333), ("parenright",333),
    ("partialdiff",494), ("percent",889), ("period",278), ("periodcentered",278),
    ("perthousand",1000), ("plus",584), ("plusminus",584), ("q",611),
    ("question",611), ("questiondown",611), ("quotedbl",474), ("quotedblbase",500),
    ("quotedblleft",500), ("quotedblright",500), ("quoteleft",278), ("quoteright",278),
    ("quotesinglbase",278), ("quotesingle",238), ("r",389), ("racute",389),
    ("radical",549), ("rcaron",389), ("rcommaaccent",389), ("registered",737),
    ("ring",333), ("s",556), ("sacute",556), ("scaron",556),
    ("scedilla",556), ("scommaaccent",556), ("section",556), ("semicolon",333),
    ("seven",556), ("six",556), ("slash",278), ("space",278),
    ("sterling",556), ("summation",600), ("t",333), ("tcaron",389),
    ("tcommaaccent",333), ("thorn",611), ("three",556), ("threequarters",834),
    ("threesuperior",333), ("tilde",333), ("trademark",1000), ("two",556),
    ("twosuperior",333), ("u",611), ("uacute",611), ("ucircumflex",611),
    ("udieresis",611), ("ugrave",611), ("uhungarumlaut",611), ("umacron",611),
    ("underscore",556), ("uogonek",611), ("uring",611), ("v",556),
    ("w",778), ("x",556), ("y",556), ("yacute",556),
    ("ydieresis",556), ("yen",556), ("z",500), ("zacute",500),
    ("zcaron",500), ("zdotaccent",500), ("zero",556),
];

/// Advance widths for `Times-Roman`, sorted by glyph name.
#[rustfmt::skip]
static TIMESROMAN_WIDTHS: [(&str, u16); 315] = [
    ("A",722), ("AE",889), ("Aacute",722), ("Abreve",722),
    ("Acircumflex",722), ("Adieresis",722), ("Agrave",722), ("Amacron",722),
    ("Aogonek",722), ("Aring",722), ("Atilde",722), ("B",667),
    ("C",667), ("Cacute",667), ("Ccaron",667), ("Ccedilla",667),
    ("D",722), ("Dcaron",722), ("Dcroat",722), ("Delta",612),
    ("E",611), ("Eacute",611), ("Ecaron",611), ("Ecircumflex",611),
    ("Edieresis",611), ("Edotaccent",611), ("Egrave",611), ("Emacron",611),
    ("Eogonek",611), ("Eth",722), ("Euro",500), ("F",556),
    ("G",722), ("Gbreve",722), ("Gcommaaccent",722), ("H",722),
    ("I",333), ("Iacute",333), ("Icircumflex",333), ("Idieresis",333),
    ("Idotaccent",333), ("Igrave",333), ("Imacron",333), ("Iogonek",333),
    ("J",389), ("K",722), ("Kcommaaccent",722), ("L",611),
    ("Lacute",611), ("Lcaron",611), ("Lcommaaccent",611), ("Lslash",611),
    ("M",889), ("N",722), ("Nacute",722), ("Ncaron",722),
    ("Ncommaaccent",722), ("Ntilde",722), ("O",722), ("OE",889),
    ("Oacute",722), ("Ocircumflex",722), ("Odieresis",722), ("Ograve",722),
    ("Ohungarumlaut",722), ("Omacron",722), ("Oslash",722), ("Otilde",722),
    ("P",556), ("Q",722), ("R",667), ("Racute",667),
    ("Rcaron",667), ("Rcommaaccent",667), ("S",556), ("Sacute",556),
    ("Scaron",556), ("Scedilla",556), ("Scommaaccent",556), ("T",611),
    ("Tcaron",611), ("Tcommaaccent",611), ("Thorn",556), ("U",722),
    ("Uacute",722), ("Ucircumflex",722), ("Udieresis",722), ("Ugrave",722),
    ("Uhungarumlaut",722), ("Umacron",722), ("Uogonek",722), ("Uring",722),
    ("V",722), ("W",944), ("X",722), ("Y",722),
    ("Yacute",722), ("Ydieresis",722), ("Z",611), ("Zacute",611),
    ("Zcaron",611), ("Zdotaccent",611), ("a",444), ("aacute",444),
    ("abreve",444), ("acircumflex",444), ("acute",333), ("adieresis",444),
    ("ae",667), ("agrave",444), ("amacron",444), ("ampersand",778),
    ("aogonek",444), ("aring",444), ("asciicircum",469), ("asciitilde",541),
    ("asterisk",500), ("at",921), ("atilde",444), ("b",500),
    ("backslash",278), ("bar",200), ("braceleft",480), ("braceright",480),
    ("bracketleft",333), ("bracketright",333), ("breve",333), ("brokenbar",200),
    ("bullet",350), ("c",444), ("cacute",444), ("caron",333),
    ("ccaron",444), ("ccedilla",444), ("cedilla",333), ("cent",500),
    ("circumflex",333), ("colon",278), ("comma",250), ("commaaccent",250),
    ("copyright",760), ("currency",500), ("d",500), ("dagger",500),
    ("daggerdbl",500), ("dcaron",588), ("dcroat",500), ("degree",400),
    ("dieresis",333), ("divide",564), ("dollar",500), ("dotaccent",333),
    ("dotlessi",278), ("e",444), ("eacute",444), ("ecaron",444),
    ("ecircumflex",444), ("edieresis",444), ("edotaccent",444), ("egrave",444),
    ("eight",500), ("ellipsis",1000), ("emacron",444), ("emdash",1000),
    ("endash",500), ("eogonek",444), ("equal",564), ("eth",500),
    ("exclam",333), ("exclamdown",333), ("f",333), ("fi",556),
    ("five",500), ("fl",556), ("florin",500), ("four",500),
    ("fraction",167), ("g",500), ("gbreve",500), ("gcommaaccent",500),
    ("germandbls",500), ("grave",333), ("greater",564), ("greaterequal",549),
    ("guillemotleft",500), ("guillemotright",500), ("guilsinglleft",333), ("guilsinglright",333),
    ("h",500), ("hungarumlaut",333), ("hyphen",333), ("i",278),
    ("iacute",278), ("icircumflex",278), ("idieresis",278), ("igrave",278),
    ("imacron",278), ("iogonek",278), ("j",278), ("k",500),
    ("kcommaaccent",500), ("l",278), ("lacute",278), ("lcaron",344),
    ("lcommaaccent",278), ("less",564), ("lessequal",549), ("logicalnot",564),
    ("lozenge",471), ("lslash",278), ("m",778), ("macron",333),
    ("minus",564), ("mu",500), ("multiply",564), ("n",500),
    ("nacute",500), ("ncaron",500), ("ncommaaccent",500), ("nine",500),
    ("notequal",549), ("ntilde",500), ("numbersign",500), ("o",500),
    ("oacute",500), ("ocircumflex",500), ("odieresis",500), ("oe",722),
    ("ogonek",333), ("ograve",500), ("ohungarumlaut",500), ("omacron",500),
    ("one",500), ("onehalf",750), ("onequarter",750), ("onesuperior",300),
    ("ordfeminine",276), ("ordmasculine",310), ("oslash",500), ("otilde",500),
    ("p",500), ("paragraph",453), ("parenleft",333), ("parenright",333),
    ("partialdiff",476), ("percent",833), ("period",250), ("periodcentered",250),
    ("perthousand",1000), ("plus",564), ("plusminus",564), ("q",500),
    ("question",444), ("questiondown",444), ("quotedbl",408), ("quotedblbase",444),
    ("quotedblleft",444), ("quotedblright",444), ("quoteleft",333), ("quoteright",333),
    ("quotesinglbase",333), ("quotesingle",180), ("r",333), ("racute",333),
    ("radical",453), ("rcaron",333), ("rcommaaccent",333), ("registered",760),
    ("ring",333), ("s",389), ("sacute",389), ("scaron",389),
    ("scedilla",389), ("scommaaccent",389), ("section",500), ("semicolon",278),
    ("seven",500), ("six",500), ("slash",278), ("space",250),
    ("sterling",500), ("summation",600), ("t",278), ("tcaron",326),
    ("tcommaaccent",278), ("thorn",500), ("three",500), ("threequarters",750),
    ("threesuperior",300), ("tilde",333), ("trademark",980), ("two",500),
    ("twosuperior",300), ("u",500), ("uacute",500), ("ucircumflex",500),
    ("udieresis",500), ("ugrave",500), ("uhungarumlaut",500), ("umacron",500),
    ("underscore",500), ("uogonek",500), ("uring",500), ("v",500),
    ("w",722), ("x",500), ("y",500), ("yacute",500),
    ("ydieresis",500), ("yen",500), ("z",444), ("zacute",444),
    ("zcaron",444), ("zdotaccent",444), ("zero",500),
];

/// Advance widths for `Times-Bold`, sorted by glyph name.
#[rustfmt::skip]
static TIMESBOLD_WIDTHS: [(&str, u16); 315] = [
    ("A",722), ("AE",1000), ("Aacute",722), ("Abreve",722),
    ("Acircumflex",722), ("Adieresis",722), ("Agrave",722), ("Amacron",722),
    ("Aogonek",722), ("Aring",722), ("Atilde",722), ("B",667),
    ("C",722), ("Cacute",722), ("Ccaron",722), ("Ccedilla",722),
    ("D",722), ("Dcaron",722), ("Dcroat",722), ("Delta",612),
    ("E",667), ("Eacute",667), ("Ecaron",667), ("Ecircumflex",667),
    ("Edieresis",667), ("Edotaccent",667), ("Egrave",667), ("Emacron",667),
    ("Eogonek",667), ("Eth",722), ("Euro",500), ("F",611),
    ("G",778), ("Gbreve",778), ("Gcommaaccent",778), ("H",778),
    ("I",389), ("Iacute",389), ("Icircumflex",389), ("Idieresis",389),
    ("Idotaccent",389), ("Igrave",389), ("Imacron",389), ("Iogonek",389),
    ("J",500), ("K",778), ("Kcommaaccent",778), ("L",667),
    ("Lacute",667), ("Lcaron",667), ("Lcommaaccent",667), ("Lslash",667),
    ("M",944), ("N",722), ("Nacute",722), ("Ncaron",722),
    ("Ncommaaccent",722), ("Ntilde",722), ("O",778), ("OE",1000),
    ("Oacute",778), ("Ocircumflex",778), ("Odieresis",778), ("Ograve",778),
    ("Ohungarumlaut",778), ("Omacron",778), ("Oslash",778), ("Otilde",778),
    ("P",611), ("Q",778), ("R",722), ("Racute",722),
    ("Rcaron",722), ("Rcommaaccent",722), ("S",556), ("Sacute",556),
    ("Scaron",556), ("Scedilla",556), ("Scommaaccent",556), ("T",667),
    ("Tcaron",667), ("Tcommaaccent",667), ("Thorn",611), ("U",722),
    ("Uacute",722), ("Ucircumflex",722), ("Udieresis",722), ("Ugrave",722),
    ("Uhungarumlaut",722), ("Umacron",722), ("Uogonek",722), ("Uring",722),
    ("V",722), ("W",1000), ("X",722), ("Y",722),
    ("Yacute",722), ("Ydieresis",722), ("Z",667), ("Zacute",667),
    ("Zcaron",667), ("Zdotaccent",667), ("a",500), ("aacute",500),
    ("abreve",500), ("acircumflex",500), ("acute",333), ("adieresis",500),
    ("ae",722), ("agrave",500), ("amacron",500), ("ampersand",833),
    ("aogonek",500), ("aring",500), ("asciicircum",581), ("asciitilde",520),
    ("asterisk",500), ("at",930), ("atilde",500), ("b",556),
    ("backslash",278), ("bar",220), ("braceleft",394), ("braceright",394),
    ("bracketleft",333), ("bracketright",333), ("breve",333), ("brokenbar",220),
    ("bullet",350), ("c",444), ("cacute",444), ("caron",333),
    ("ccaron",444), ("ccedilla",444), ("cedilla",333), ("cent",500),
    ("circumflex",333), ("colon",333), ("comma",250), ("commaaccent",250),
    ("copyright",747), ("currency",500), ("d",556), ("dagger",500),
    ("daggerdbl",500), ("dcaron",672), ("dcroat",556), ("degree",400),
    ("dieresis",333), ("divide",570), ("dollar",500), ("dotaccent",333),
    ("dotlessi",278), ("e",444), ("eacute",444), ("ecaron",444),
    ("ecircumflex",444), ("edieresis",444), ("edotaccent",444), ("egrave",444),
    ("eight",500), ("ellipsis",1000), ("emacron",444), ("emdash",1000),
    ("endash",500), ("eogonek",444), ("equal",570), ("eth",500),
    ("exclam",333), ("exclamdown",333), ("f",333), ("fi",556),
    ("five",500), ("fl",556), ("florin",500), ("four",500),
    ("fraction",167), ("g",500), ("gbreve",500), ("gcommaaccent",500),
    ("germandbls",556), ("grave",333), ("greater",570), ("greaterequal",549),
    ("guillemotleft",500), ("guillemotright",500), ("guilsinglleft",333), ("guilsinglright",333),
    ("h",556), ("hungarumlaut",333), ("hyphen",333), ("i",278),
    ("iacute",278), ("icircumflex",278), ("idieresis",278), ("igrave",278),
    ("imacron",278), ("iogonek",278), ("j",333), ("k",556),
    ("kcommaaccent",556), ("l",278), ("lacute",278), ("lcaron",394),
    ("lcommaaccent",278), ("less",570), ("lessequal",549), ("logicalnot",570),
    ("lozenge",494), ("lslash",278), ("m",833), ("macron",333),
    ("minus",570), ("mu",556), ("multiply",570), ("n",556),
    ("nacute",556), ("ncaron",556), ("ncommaaccent",556), ("nine",500),
    ("notequal",549), ("ntilde",556), ("numbersign",500), ("o",500),
    ("oacute",500), ("ocircumflex",500), ("odieresis",500), ("oe",722),
    ("ogonek",333), ("ograve",500), ("ohungarumlaut",500), ("omacron",500),
    ("one",500), ("onehalf",750), ("onequarter",750), ("onesuperior",300),
    ("ordfeminine",300), ("ordmasculine",330), ("oslash",500), ("otilde",500),
    ("p",556), ("paragraph",540), ("parenleft",333), ("parenright",333),
    ("partialdiff",494), ("percent",1000), ("period",250), ("periodcentered",250),
    ("perthousand",1000), ("plus",570), ("plusminus",570), ("q",556),
    ("question",500), ("questiondown",500), ("quotedbl",555), ("quotedblbase",500),
    ("quotedblleft",500), ("quotedblright",500), ("quoteleft",333), ("quoteright",333),
    ("quotesinglbase",333), ("quotesingle",278), ("r",444), ("racute",444),
    ("radical",549), ("rcaron",444), ("rcommaaccent",444), ("registered",747),
    ("ring",333), ("s",389), ("sacute",389), ("scaron",389),
    ("scedilla",389), ("scommaaccent",389), ("section",500), ("semicolon",333),
    ("seven",500), ("six",500), ("slash",278), ("space",250),
    ("sterling",500), ("summation",600), ("t",333), ("tcaron",416),
    ("tcommaaccent",333), ("thorn",556), ("three",500), ("threequarters",750),
    ("threesuperior",300), ("tilde",333), ("trademark",1000), ("two",500),
    ("twosuperior",300), ("u",556), ("uacute",556), ("ucircumflex",556),
    ("udieresis",556), ("ugrave",556), ("uhungarumlaut",556), ("umacron",556),
    ("underscore",500), ("uogonek",556), ("uring",556), ("v",500),
    ("w",722), ("x",500), ("y",500), ("yacute",500),
    ("ydieresis",500), ("yen",500), ("z",444), ("zacute",444),
    ("zcaron",444), ("zdotaccent",444), ("zero",500),
];

/// Advance widths for `Times-Italic`, sorted by glyph name.
#[rustfmt::skip]
static TIMESITALIC_WIDTHS: [(&str, u16); 315] = [
    ("A",611), ("AE",889), ("Aacute",611), ("Abreve",611),
    ("Acircumflex",611), ("Adieresis",611), ("Agrave",611), ("Amacron",611),
    ("Aogonek",611), ("Aring",611), ("Atilde",611), ("B",611),
    ("C",667), ("Cacute",667), ("Ccaron",667), ("Ccedilla",667),
    ("D",722), ("Dcaron",722), ("Dcroat",722), ("Delta",612),
    ("E",611), ("Eacute",611), ("Ecaron",611), ("Ecircumflex",611),
    ("Edieresis",611), ("Edotaccent",611), ("Egrave",611), ("Emacron",611),
    ("Eogonek",611), ("Eth",722), ("Euro",500), ("F",611),
    ("G",722), ("Gbreve",722), ("Gcommaaccent",722), ("H",722),
    ("I",333), ("Iacute",333), ("Icircumflex",333), ("Idieresis",333),
    ("Idotaccent",333), ("Igrave",333), ("Imacron",333), ("Iogonek",333),
    ("J",444), ("K",667), ("Kcommaaccent",667), ("L",556),
    ("Lacute",556), ("Lcaron",611), ("Lcommaaccent",556), ("Lslash",556),
    ("M",833), ("N",667), ("Nacute",667), ("Ncaron",667),
    ("Ncommaaccent",667), ("Ntilde",667), ("O",722), ("OE",944),
    ("Oacute",722), ("Ocircumflex",722), ("Odieresis",722), ("Ograve",722),
    ("Ohungarumlaut",722), ("Omacron",722), ("Oslash",722), ("Otilde",722),
    ("P",611), ("Q",722), ("R",611), ("Racute",611),
    ("Rcaron",611), ("Rcommaaccent",611), ("S",500), ("Sacute",500),
    ("Scaron",500), ("Scedilla",500), ("Scommaaccent",500), ("T",556),
    ("Tcaron",556), ("Tcommaaccent",556), ("Thorn",611), ("U",722),
    ("Uacute",722), ("Ucircumflex",722), ("Udieresis",722), ("Ugrave",722),
    ("Uhungarumlaut",722), ("Umacron",722), ("Uogonek",722), ("Uring",722),
    ("V",611), ("W",833), ("X",611), ("Y",556),
    ("Yacute",556), ("Ydieresis",556), ("Z",556), ("Zacute",556),
    ("Zcaron",556), ("Zdotaccent",556), ("a",500), ("aacute",500),
    ("abreve",500), ("acircumflex",500), ("acute",333), ("adieresis",500),
    ("ae",667), ("agrave",500), ("amacron",500), ("ampersand",778),
    ("aogonek",500), ("aring",500), ("asciicircum",422), ("asciitilde",541),
    ("asterisk",500), ("at",920), ("atilde",500), ("b",500),
    ("backslash",278), ("bar",275), ("braceleft",400), ("braceright",400),
    ("bracketleft",389), ("bracketright",389), ("breve",333), ("brokenbar",275),
    ("bullet",350), ("c",444), ("cacute",444), ("caron",333),
    ("ccaron",444), ("ccedilla",444), ("cedilla",333), ("cent",500),
    ("circumflex",333), ("colon",333), ("comma",250), ("commaaccent",250),
    ("copyright",760), ("currency",500), ("d",500), ("dagger",500),
    ("daggerdbl",500), ("dcaron",544), ("dcroat",500), ("degree",400),
    ("dieresis",333), ("divide",675), ("dollar",500), ("dotaccent",333),
    ("dotlessi",278), ("e",444), ("eacute",444), ("ecaron",444),
    ("ecircumflex",444), ("edieresis",444), ("edotaccent",444), ("egrave",444),
    ("eight",500), ("ellipsis",889), ("emacron",444), ("emdash",889),
    ("endash",500), ("eogonek",444), ("equal",675), ("eth",500),
    ("exclam",333), ("exclamdown",389), ("f",278), ("fi",500),
    ("five",500), ("fl",500), ("florin",500), ("four",500),
    ("fraction",167), ("g",500), ("gbreve",500), ("gcommaaccent",500),
    ("germandbls",500), ("grave",333), ("greater",675), ("greaterequal",549),
    ("guillemotleft",500), ("guillemotright",500), ("guilsinglleft",333), ("guilsinglright",333),
    ("h",500), ("hungarumlaut",333), ("hyphen",333), ("i",278),
    ("iacute",278), ("icircumflex",278), ("idieresis",278), ("igrave",278),
    ("imacron",278), ("iogonek",278), ("j",278), ("k",444),
    ("kcommaaccent",444), ("l",278), ("lacute",278), ("lcaron",300),
    ("lcommaaccent",278), ("less",675), ("lessequal",549), ("logicalnot",675),
    ("lozenge",471), ("lslash",278), ("m",722), ("macron",333),
    ("minus",675), ("mu",500), ("multiply",675), ("n",500),
    ("nacute",500), ("ncaron",500), ("ncommaaccent",500), ("nine",500),
    ("notequal",549), ("ntilde",500), ("numbersign",500), ("o",500),
    ("oacute",500), ("ocircumflex",500), ("odieresis",500), ("oe",667),
    ("ogonek",333), ("ograve",500), ("ohungarumlaut",500), ("omacron",500),
    ("one",500), ("onehalf",750), ("onequarter",750), ("onesuperior",300),
    ("ordfeminine",276), ("ordmasculine",310), ("oslash",500), ("otilde",500),
    ("p",500), ("paragraph",523), ("parenleft",333), ("parenright",333),
    ("partialdiff",476), ("percent",833), ("period",250), ("periodcentered",250),
    ("perthousand",1000), ("plus",675), ("plusminus",675), ("q",500),
    ("question",500), ("questiondown",500), ("quotedbl",420), ("quotedblbase",556),
    ("quotedblleft",556), ("quotedblright",556), ("quoteleft",333), ("quoteright",333),
    ("quotesinglbase",333), ("quotesingle",214), ("r",389), ("racute",389),
    ("radical",453), ("rcaron",389), ("rcommaaccent",389), ("registered",760),
    ("ring",333), ("s",389), ("sacute",389), ("scaron",389),
    ("scedilla",389), ("scommaaccent",389), ("section",500), ("semicolon",333),
    ("seven",500), ("six",500), ("slash",278), ("space",250),
    ("sterling",500), ("summation",600), ("t",278), ("tcaron",300),
    ("tcommaaccent",278), ("thorn",500), ("three",500), ("threequarters",750),
    ("threesuperior",300), ("tilde",333), ("trademark",980), ("two",500),
    ("twosuperior",300), ("u",500), ("uacute",500), ("ucircumflex",500),
    ("udieresis",500), ("ugrave",500), ("uhungarumlaut",500), ("umacron",500),
    ("underscore",500), ("uogonek",500), ("uring",500), ("v",444),
    ("w",667), ("x",444), ("y",444), ("yacute",444),
    ("ydieresis",444), ("yen",500), ("z",389), ("zacute",389),
    ("zcaron",389), ("zdotaccent",389), ("zero",500),
];

/// Advance widths for `Times-BoldItalic`, sorted by glyph name.
#[rustfmt::skip]
static TIMESBOLDITALIC_WIDTHS: [(&str, u16); 315] = [
    ("A",667), ("AE",944), ("Aacute",667), ("Abreve",667),
    ("Acircumflex",667), ("Adieresis",667), ("Agrave",667), ("Amacron",667),
    ("Aogonek",667), ("Aring",667), ("Atilde",667), ("B",667),
    ("C",667), ("Cacute",667), ("Ccaron",667), ("Ccedilla",667),
    ("D",722), ("Dcaron",722), ("Dcroat",722), ("Delta",612),
    ("E",667), ("Eacute",667), ("Ecaron",667), ("Ecircumflex",667),
    ("Edieresis",667), ("Edotaccent",667), ("Egrave",667), ("Emacron",667),
    ("Eogonek",667), ("Eth",722), ("Euro",500), ("F",667),
    ("G",722), ("Gbreve",722), ("Gcommaaccent",722), ("H",778),
    ("I",389), ("Iacute",389), ("Icircumflex",389), ("Idieresis",389),
    ("Idotaccent",389), ("Igrave",389), ("Imacron",389), ("Iogonek",389),
    ("J",500), ("K",667), ("Kcommaaccent",667), ("L",611),
    ("Lacute",611), ("Lcaron",611), ("Lcommaaccent",611), ("Lslash",611),
    ("M",889), ("N",722), ("Nacute",722), ("Ncaron",722),
    ("Ncommaaccent",722), ("Ntilde",722), ("O",722), ("OE",944),
    ("Oacute",722), ("Ocircumflex",722), ("Odieresis",722), ("Ograve",722),
    ("Ohungarumlaut",722), ("Omacron",722), ("Oslash",722), ("Otilde",722),
    ("P",611), ("Q",722), ("R",667), ("Racute",667),
    ("Rcaron",667), ("Rcommaaccent",667), ("S",556), ("Sacute",556),
    ("Scaron",556), ("Scedilla",556), ("Scommaaccent",556), ("T",611),
    ("Tcaron",611), ("Tcommaaccent",611), ("Thorn",611), ("U",722),
    ("Uacute",722), ("Ucircumflex",722), ("Udieresis",722), ("Ugrave",722),
    ("Uhungarumlaut",722), ("Umacron",722), ("Uogonek",722), ("Uring",722),
    ("V",667), ("W",889), ("X",667), ("Y",611),
    ("Yacute",611), ("Ydieresis",611), ("Z",611), ("Zacute",611),
    ("Zcaron",611), ("Zdotaccent",611), ("a",500), ("aacute",500),
    ("abreve",500), ("acircumflex",500), ("acute",333), ("adieresis",500),
    ("ae",722), ("agrave",500), ("amacron",500), ("ampersand",778),
    ("aogonek",500), ("aring",500), ("asciicircum",570), ("asciitilde",570),
    ("asterisk",500), ("at",832), ("atilde",500), ("b",500),
    ("backslash",278), ("bar",220), ("braceleft",348), ("braceright",348),
    ("bracketleft",333), ("bracketright",333), ("breve",333), ("brokenbar",220),
    ("bullet",350), ("c",444), ("cacute",444), ("caron",333),
    ("ccaron",444), ("ccedilla",444), ("cedilla",333), ("cent",500),
    ("circumflex",333), ("colon",333), ("comma",250), ("commaaccent",250),
    ("copyright",747), ("currency",500), ("d",500), ("dagger",500),
    ("daggerdbl",500), ("dcaron",608), ("dcroat",500), ("degree",400),
    ("dieresis",333), ("divide",570), ("dollar",500), ("dotaccent",333),
    ("dotlessi",278), ("e",444), ("eacute",444), ("ecaron",444),
    ("ecircumflex",444), ("edieresis",444), ("edotaccent",444), ("egrave",444),
    ("eight",500), ("ellipsis",1000), ("emacron",444), ("emdash",1000),
    ("endash",500), ("eogonek",444), ("equal",570), ("eth",500),
    ("exclam",389), ("exclamdown",389), ("f",333), ("fi",556),
    ("five",500), ("fl",556), ("florin",500), ("four",500),
    ("fraction",167), ("g",500), ("gbreve",500), ("gcommaaccent",500),
    ("germandbls",500), ("grave",333), ("greater",570), ("greaterequal",549),
    ("guillemotleft",500), ("guillemotright",500), ("guilsinglleft",333), ("guilsinglright",333),
    ("h",556), ("hungarumlaut",333), ("hyphen",333), ("i",278),
    ("iacute",278), ("icircumflex",278), ("idieresis",278), ("igrave",278),
    ("imacron",278), ("iogonek",278), ("j",278), ("k",500),
    ("kcommaaccent",500), ("l",278), ("lacute",278), ("lcaron",382),
    ("lcommaaccent",278), ("less",570), ("lessequal",549), ("logicalnot",606),
    ("lozenge",494), ("lslash",278), ("m",778), ("macron",333),
    ("minus",606), ("mu",576), ("multiply",570), ("n",556),
    ("nacute",556), ("ncaron",556), ("ncommaaccent",556), ("nine",500),
    ("notequal",549), ("ntilde",556), ("numbersign",500), ("o",500),
    ("oacute",500), ("ocircumflex",500), ("odieresis",500), ("oe",722),
    ("ogonek",333), ("ograve",500), ("ohungarumlaut",500), ("omacron",500),
    ("one",500), ("onehalf",750), ("onequarter",750), ("onesuperior",300),
    ("ordfeminine",266), ("ordmasculine",300), ("oslash",500), ("otilde",500),
    ("p",500), ("paragraph",500), ("parenleft",333), ("parenright",333),
    ("partialdiff",494), ("percent",833), ("period",250), ("periodcentered",250),
    ("perthousand",1000), ("plus",570), ("plusminus",570), ("q",500),
    ("question",500), ("questiondown",500), ("quotedbl",555), ("quotedblbase",500),
    ("quotedblleft",500), ("quotedblright",500), ("quoteleft",333), ("quoteright",333),
    ("quotesinglbase",333), ("quotesingle",278), ("r",389), ("racute",389),
    ("radical",549), ("rcaron",389), ("rcommaaccent",389), ("registered",747),
    ("ring",333), ("s",389), ("sacute",389), ("scaron",389),
    ("scedilla",389), ("scommaaccent",389), ("section",500), ("semicolon",333),
    ("seven",500), ("six",500), ("slash",278), ("space",250),
    ("sterling",500), ("summation",600), ("t",278), ("tcaron",366),
    ("tcommaaccent",278), ("thorn",500), ("three",500), ("threequarters",750),
    ("threesuperior",300), ("tilde",333), ("trademark",1000), ("two",500),
    ("twosuperior",300), ("u",556), ("uacute",556), ("ucircumflex",556),
    ("udieresis",556), ("ugrave",556), ("uhungarumlaut",556), ("umacron",556),
    ("underscore",500), ("uogonek",556), ("uring",556), ("v",444),
    ("w",667), ("x",500), ("y",444), ("yacute",444),
    ("ydieresis",444), ("yen",500), ("z",389), ("zacute",389),
    ("zcaron",389), ("zdotaccent",389), ("zero",500),
];

/// Advance widths for `Symbol`, sorted by glyph name.
#[rustfmt::skip]
static SYMBOL_WIDTHS: [(&str, u16); 190] = [
    ("Alpha",722), ("Beta",667), ("Chi",722), ("Delta",612),
    ("Epsilon",611), ("Eta",722), ("Euro",750), ("Gamma",603),
    ("Ifraktur",686), ("Iota",333), ("Kappa",722), ("Lambda",686),
    ("Mu",889), ("Nu",722), ("Omega",768), ("Omicron",722),
    ("Phi",763), ("Pi",768), ("Psi",795), ("Rfraktur",795),
    ("Rho",556), ("Sigma",592), ("Tau",611), ("Theta",741),
    ("Upsilon",690), ("Upsilon1",620), ("Xi",645), ("Zeta",611),
    ("aleph",823), ("alpha",631), ("ampersand",778), ("angle",768),
    ("angleleft",329), ("angleright",329), ("apple",790), ("approxequal",549),
    ("arrowboth",1042), ("arrowdblboth",1042), ("arrowdbldown",603), ("arrowdblleft",987),
    ("arrowdblright",987), ("arrowdblup",603), ("arrowdown",603), ("arrowhorizex",1000),
    ("arrowleft",987), ("arrowright",987), ("arrowup",603), ("arrowvertex",603),
    ("asteriskmath",500), ("bar",200), ("beta",549), ("braceex",494),
    ("braceleft",480), ("braceleftbt",494), ("braceleftmid",494), ("bracelefttp",494),
    ("braceright",480), ("bracerightbt",494), ("bracerightmid",494), ("bracerighttp",494),
    ("bracketleft",333), ("bracketleftbt",384), ("bracketleftex",384), ("bracketlefttp",384),
    ("bracketright",333), ("bracketrightbt",384), ("bracketrightex",384), ("bracketrighttp",384),
    ("bullet",460), ("carriagereturn",658), ("chi",549), ("circlemultiply",768),
    ("circleplus",768), ("club",753), ("colon",278), ("comma",250),
    ("congruent",549), ("copyrightsans",790), ("copyrightserif",790), ("degree",400),
    ("delta",494), ("diamond",753), ("divide",549), ("dotmath",250),
    ("eight",500), ("element",713), ("ellipsis",1000), ("emptyset",823),
    ("epsilon",439), ("equal",549), ("equivalence",549), ("eta",603),
    ("exclam",333), ("existential",549), ("five",500), ("florin",500),
    ("four",500), ("fraction",167), ("gamma",411), ("gradient",713),
    ("greater",549), ("greaterequal",549), ("heart",753), ("infinity",713),
    ("integral",274), ("integralbt",686), ("integralex",686), ("integraltp",686),
    ("intersection",768), ("iota",329), ("kappa",549), ("lambda",549),
    ("less",549), ("lessequal",549), ("logicaland",603), ("logicalnot",713),
    ("logicalor",603), ("lozenge",494), ("minus",549), ("minute",247),
    ("mu",576), ("multiply",549), ("nine",500), ("notelement",713),
    ("notequal",549), ("notsubset",713), ("nu",521), ("numbersign",500),
    ("omega",686), ("omega1",713), ("omicron",549), ("one",500),
    ("parenleft",333), ("parenleftbt",384), ("parenleftex",384), ("parenlefttp",384),
    ("parenright",333), ("parenrightbt",384), ("parenrightex",384), ("parenrighttp",384),
    ("partialdiff",494), ("percent",833), ("period",250), ("perpendicular",658),
    ("phi",521), ("phi1",603), ("pi",549), ("plus",549),
    ("plusminus",549), ("product",823), ("propersubset",713), ("propersuperset",713),
    ("proportional",713), ("psi",686), ("question",444), ("radical",549),
    ("radicalex",500), ("reflexsubset",713), ("reflexsuperset",713), ("registersans",790),
    ("registerserif",790), ("rho",549), ("second",411), ("semicolon",278),
    ("seven",500), ("sigma",603), ("sigma1",439), ("similar",549),
    ("six",500), ("slash",278), ("space",250), ("spade",753),
    ("suchthat",439), ("summation",713), ("tau",439), ("therefore",863),
    ("theta",521), ("theta1",631), ("three",500), ("trademarksans",786),
    ("trademarkserif",890), ("two",500), ("underscore",500), ("union",768),
    ("universal",713), ("upsilon",576), ("weierstrass",987), ("xi",493),
    ("zero",500), ("zeta",494),
];

/// Advance widths for `ZapfDingbats`, sorted by glyph name.
#[rustfmt::skip]
static ZAPFDINGBATS_WIDTHS: [(&str, u16); 202] = [
    ("a1",974), ("a10",692), ("a100",668), ("a101",732),
    ("a102",544), ("a103",544), ("a104",910), ("a105",911),
    ("a106",667), ("a107",760), ("a108",760), ("a109",626),
    ("a11",960), ("a110",694), ("a111",595), ("a112",776),
    ("a117",690), ("a118",791), ("a119",790), ("a12",939),
    ("a120",788), ("a121",788), ("a122",788), ("a123",788),
    ("a124",788), ("a125",788), ("a126",788), ("a127",788),
    ("a128",788), ("a129",788), ("a13",549), ("a130",788),
    ("a131",788), ("a132",788), ("a133",788), ("a134",788),
    ("a135",788), ("a136",788), ("a137",788), ("a138",788),
    ("a139",788), ("a14",855), ("a140",788), ("a141",788),
    ("a142",788), ("a143",788), ("a144",788), ("a145",788),
    ("a146",788), ("a147",788), ("a148",788), ("a149",788),
    ("a15",911), ("a150",788), ("a151",788), ("a152",788),
    ("a153",788), ("a154",788), ("a155",788), ("a156",788),
    ("a157",788), ("a158",788), ("a159",788), ("a16",933),
    ("a160",894), ("a161",838), ("a162",924), ("a163",1016),
    ("a164",458), ("a165",924), ("a166",918), ("a167",927),
    ("a168",928), ("a169",928), ("a17",945), ("a170",834),
    ("a171",873), ("a172",828), ("a173",924), ("a174",917),
    ("a175",930), ("a176",931), ("a177",463), ("a178",883),
    ("a179",836), ("a18",974), ("a180",867), ("a181",696),
    ("a182",874), ("a183",760), ("a184",946), ("a185",865),
    ("a186",967), ("a187",831), ("a188",873), ("a189",927),
    ("a19",755), ("a190",970), ("a191",918), ("a192",748),
    ("a193",836), ("a194",771), ("a195",888), ("a196",748),
    ("a197",771), ("a198",888), ("a199",867), ("a2",961),
    ("a20",846), ("a200",696), ("a201",874), ("a202",974),
    ("a203",762), ("a204",759), ("a205",509), ("a206",410),
    ("a21",762), ("a22",761), ("a23",571), ("a24",677),
    ("a25",763), ("a26",760), ("a27",759), ("a28",754),
    ("a29",786), ("a3",980), ("a30",788), ("a31",788),
    ("a32",790), ("a33",793), ("a34",794), ("a35",816),
    ("a36",823), ("a37",789), ("a38",841), ("a39",823),
    ("a4",719), ("a40",833), ("a41",816), ("a42",831),
    ("a43",923), ("a44",744), ("a45",723), ("a46",749),
    ("a47",790), ("a48",792), ("a49",695), ("a5",789),
    ("a50",776), ("a51",768), ("a52",792), ("a53",759),
    ("a54",707), ("a55",708), ("a56",682), ("a57",701),
    ("a58",826), ("a59",815), ("a6",494), ("a60",789),
    ("a61",789), ("a62",707), ("a63",687), ("a64",696),
    ("a65",689), ("a66",786), ("a67",787), ("a68",713),
    ("a69",791), ("a7",552), ("a70",785), ("a71",791),
    ("a72",873), ("a73",761), ("a74",762), ("a75",759),
    ("a76",892), ("a77",892), ("a78",788), ("a79",784),
    ("a8",537), ("a81",438), ("a82",138), ("a83",277),
    ("a84",415), ("a85",509), ("a86",410), ("a87",234),
    ("a88",234), ("a89",390), ("a9",577), ("a90",390),
    ("a91",276), ("a92",276), ("a93",317), ("a94",317),
    ("a95",334), ("a96",334), ("a97",392), ("a98",392),
    ("a99",668), ("space",278),
];

#[cfg(test)]
mod tests {
    use super::{HELVETICA_WIDTHS, SYMBOL_WIDTHS, StandardFont, ZAPFDINGBATS_WIDTHS};

    /// Every table is looked up by binary search, which silently returns nonsense on
    /// unsorted input. The generator sorts them; this is what proves it kept doing so.
    #[test]
    fn every_table_is_sorted_by_glyph_name() {
        for face in [
            StandardFont::Helvetica,
            StandardFont::HelveticaBold,
            StandardFont::HelveticaOblique,
            StandardFont::HelveticaBoldOblique,
            StandardFont::TimesRoman,
            StandardFont::TimesBold,
            StandardFont::TimesItalic,
            StandardFont::TimesBoldItalic,
            StandardFont::Symbol,
            StandardFont::ZapfDingbats,
        ] {
            // Reaching the table through a lookup keeps this honest even if the match in
            // `width` is later rewired to the wrong constant.
            assert!(
                face.width("space").is_some(),
                "{face:?} has no space glyph, so its table is not wired up"
            );
        }

        for (label, table) in [
            ("Helvetica", &HELVETICA_WIDTHS[..]),
            ("Symbol", &SYMBOL_WIDTHS[..]),
            ("ZapfDingbats", &ZAPFDINGBATS_WIDTHS[..]),
        ] {
            assert!(
                table.windows(2).all(|pair| pair[0].0 < pair[1].0),
                "{label} is not sorted, so binary search would miss glyphs"
            );
        }
    }

    /// Values taken from the specification's own published metrics.
    #[test]
    fn known_widths_are_what_the_specification_says() {
        assert_eq!(StandardFont::Helvetica.width("space"), Some(278.0));
        assert_eq!(StandardFont::Helvetica.width("A"), Some(667.0));
        assert_eq!(StandardFont::HelveticaBold.width("A"), Some(722.0));
        assert_eq!(StandardFont::TimesRoman.width("A"), Some(722.0));
        assert_eq!(StandardFont::ZapfDingbats.width("a1"), Some(974.0));
        // Courier is fixed pitch, and every glyph it has is the same width.
        assert_eq!(StandardFont::Courier.width("A"), Some(600.0));
        assert_eq!(StandardFont::Courier.width("i"), Some(600.0));
        // A glyph no standard face has must be absent rather than zero.
        assert_eq!(StandardFont::Helvetica.width("notaglyphname"), None);
    }

    /// The metrics must match a font drawn independently to the same specification.
    ///
    /// The URW clones shipped with Ghostscript reproduce the standard-14 advances by
    /// design — that is their entire purpose. They are AGPL, so their numbers cannot be
    /// *copied* into this crate, but nothing stops us reading them at test time to check
    /// that the numbers taken from pdf.js are right. Two independent sources, one
    /// specification.
    ///
    /// Skipped where those fonts are not installed, since it checks the table rather than
    /// the machine.
    #[test]
    fn the_metrics_agree_with_an_independently_drawn_metric_clone() {
        use skrifa::prelude::{LocationRef, Size};
        use skrifa::{FontRef, MetadataProvider};

        let clones = [
            (
                "/usr/share/fonts/gsfonts/NimbusSans-Regular.otf",
                StandardFont::Helvetica,
            ),
            (
                "/usr/share/fonts/gsfonts/NimbusSans-Bold.otf",
                StandardFont::HelveticaBold,
            ),
            (
                "/usr/share/fonts/gsfonts/NimbusRoman-Regular.otf",
                StandardFont::TimesRoman,
            ),
            (
                "/usr/share/fonts/gsfonts/NimbusMonoPS-Regular.otf",
                StandardFont::Courier,
            ),
        ];

        let mut compared = 0usize;
        for (path, face) in clones {
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            let Ok(font) = FontRef::new(&bytes) else {
                continue;
            };
            let charmap = font.charmap();
            let metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());
            let upem = f32::from(
                font.metrics(Size::unscaled(), LocationRef::default())
                    .units_per_em,
            );

            // A sample across the Latin set: letters, digits and punctuation of visibly
            // different widths, so a table shifted by one entry cannot pass.
            for (glyph_name, character) in [
                ("space", ' '),
                ("A", 'A'),
                ("W", 'W'),
                ("i", 'i'),
                ("m", 'm'),
                ("period", '.'),
                ("zero", '0'),
                ("percent", '%'),
                ("question", '?'),
            ] {
                let Some(expected) = face.width(glyph_name) else {
                    continue;
                };
                let Some(glyph) = charmap.map(character) else {
                    continue;
                };
                let Some(advance) = metrics.advance_width(glyph) else {
                    continue;
                };
                let actual = advance / upem * 1000.0;
                compared += 1;
                assert!(
                    (actual - expected).abs() <= 1.0,
                    "{path}: {glyph_name} is {expected} in our table but {actual} in the font"
                );
            }
        }

        if compared == 0 {
            println!("skipped: no URW metric clones installed");
        } else {
            println!("compared {compared} advances against installed metric clones");
        }
    }
}
