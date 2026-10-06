//! Well Known Text, read as ISO 19162 section 6 states its string form.
//!
//! §12.10.3 hands the format of a coordinate system's `/WKT` entry to another standard: "[t]he WKT
//! (Well Known Text) format is specified in ISO 19162". This module is that string form and no
//! more — keywords, delimiters, quoted text, numbers and bare enumerations, nested to any depth —
//! and [`super::system`] is what the tree *means*. The two are separate because the grammar is
//! shared by every dialect a file carries: ISO 19162's own keywords and the older ones its Annex C
//! documents for backward compatibility parse to the same tree.
//!
//! What ISO 19162 section 6 fixes, each paraphrased because the text is not ISO 32000-2's:
//!
//! - a token is a keyword followed by its attributes between delimiters, and attributes are
//!   separated by commas (section 6.1);
//! - the delimiters are brackets, or parentheses in their place, and a string uses one form
//!   throughout — a closing delimiter must match its opening one (section 6.4);
//! - keywords and enumerations are case-insensitive, quoted text is not (section 6.5);
//! - a double quote inside quoted text is written as two (section 6.3.5);
//! - white space outside quoted text is padding (section 6.1).
//!
//! # The one tolerance, and why it is the standard's own example
//!
//! Between two bracketed attributes a missing comma is accepted. ISO 32000-2's own EXAMPLE 2 in
//! §12.10.4 writes `PARAMETER["Standard_Parallel_2",60.0] PARAMETER["Latitude_Of_Origin",40.0]`
//! with no separator, and a reader that refused the clause's own example would be reading a
//! stricter grammar than the clause's author did. Nothing else is accepted that section 6 does
//! not state; ADR 1587 records the choice.

use std::fmt;

/// How deeply tokens may nest before the parse refuses.
///
/// ISO 19162 section 6.1 puts no limit on nesting; a real coordinate system nests
/// five or six levels (a projected system, its base, its datum, its ellipsoid, an identifier),
/// and a recursive parser over a file's bytes needs a bound a hostile string cannot pass.
const MAX_DEPTH: usize = 32;

/// One token: a keyword and its attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The keyword, upper-cased: ISO 19162 section 6.5 makes keywords case-insensitive.
    pub keyword: String,
    /// The attributes, in the order the string states them.
    pub attributes: Vec<Attribute>,
}

/// One attribute of a token.
#[derive(Debug, Clone, PartialEq)]
pub enum Attribute {
    /// Quoted text, with a doubled double quote read as one.
    Text(String),
    /// A number.
    Number(f64),
    /// A bare word that is not followed by a delimiter — an enumeration such as `NORTH`,
    /// upper-cased because enumerations are case-insensitive.
    Word(String),
    /// A nested token.
    Node(Node),
}

/// Why a string is not Well Known Text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WktError {
    /// The string ended inside a token or a quoted text.
    #[error("the Well Known Text ends before its last token closes")]
    Truncated,
    /// A character that no production of ISO 19162 section 6 allows at this place.
    #[error("unexpected {found:?} at byte {at} of the Well Known Text")]
    Unexpected {
        /// The character found.
        found: char,
        /// Its byte offset.
        at: usize,
    },
    /// A closing delimiter of the other form — ISO 19162 section 6.4 requires a string to use
    /// one form throughout.
    #[error("a {found:?} closes a token opened with {opened:?}")]
    MismatchedDelimiter {
        /// The opening delimiter.
        opened: char,
        /// The closing delimiter found.
        found: char,
    },
    /// Nesting past [`MAX_DEPTH`].
    #[error("the Well Known Text nests more than {MAX_DEPTH} tokens deep")]
    TooDeep,
    /// A number the grammar of ISO 19162 section 6.3.2 does not produce.
    #[error("{0:?} is not a number")]
    Number(String),
    /// Text after the outermost token closes.
    #[error("text follows the outermost token at byte {0}")]
    Trailing(usize),
}

impl Node {
    /// The nested tokens with this keyword (upper case), in order.
    pub fn children<'a>(&'a self, keyword: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.attributes
            .iter()
            .filter_map(move |attribute| match attribute {
                Attribute::Node(node) if node.keyword == keyword => Some(node),
                _ => None,
            })
    }

    /// The first nested token with any of these keywords.
    #[must_use]
    pub fn child(&self, keywords: &[&str]) -> Option<&Node> {
        self.attributes
            .iter()
            .find_map(|attribute| match attribute {
                Attribute::Node(node) if keywords.contains(&node.keyword.as_str()) => Some(node),
                _ => None,
            })
    }

    /// The first attribute, where it is quoted text — the name every CRS component opens with.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self.attributes.first() {
            Some(Attribute::Text(text)) => Some(text),
            _ => None,
        }
    }

    /// The `index`th attribute, where it is a number.
    #[must_use]
    pub fn number(&self, index: usize) -> Option<f64> {
        match self.attributes.get(index) {
            Some(Attribute::Number(value)) => Some(*value),
            _ => None,
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.keyword)?;
        if let Some(name) = self.name() {
            write!(f, "[\"{name}\"]")?;
        }
        Ok(())
    }
}

/// Parses one Well Known Text string into its outermost token.
///
/// # Errors
///
/// [`WktError`] names the first place the string departs from ISO 19162 section 6.
pub fn parse(text: &str) -> Result<Node, WktError> {
    let mut parser = Parser {
        text,
        bytes: text.as_bytes(),
        at: 0,
    };
    parser.skip_space();
    let node = parser.node(0)?;
    parser.skip_space();
    if parser.at < parser.bytes.len() {
        return Err(WktError::Trailing(parser.at));
    }
    Ok(node)
}

/// A cursor over the string.
struct Parser<'a> {
    /// The string.
    text: &'a str,
    /// Its bytes, which every delimiter and keyword character is one of.
    bytes: &'a [u8],
    /// The byte offset of the next unread character.
    at: usize,
}

impl Parser<'_> {
    /// Moves the cursor on by `by` bytes; the cursor never passes the string's length, so the
    /// saturation is never reached.
    fn advance(&mut self, by: usize) {
        self.at = self.at.saturating_add(by);
    }

    /// Skips padding: ISO 19162 section 6.1 says white space outside quoted text is not part of
    /// the string.
    fn skip_space(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.advance(1);
        }
    }

    /// The character at the cursor, for an error.
    fn here(&self) -> WktError {
        match self
            .text
            .get(self.at..)
            .and_then(|rest| rest.chars().next())
        {
            Some(found) => WktError::Unexpected { found, at: self.at },
            None => WktError::Truncated,
        }
    }

    /// A keyword: letters, digits and underscores, the underscore significant (section 6.5).
    fn word(&mut self) -> String {
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            self.advance(1);
        }
        self.text[start..self.at].to_ascii_uppercase()
    }

    /// One token, the cursor on its keyword.
    fn node(&mut self, depth: usize) -> Result<Node, WktError> {
        if depth >= MAX_DEPTH {
            return Err(WktError::TooDeep);
        }
        if !self.bytes.get(self.at).is_some_and(u8::is_ascii_alphabetic) {
            return Err(self.here());
        }
        let keyword = self.word();
        self.skip_space();
        let opened = match self.bytes.get(self.at) {
            Some(b'[') => ']',
            Some(b'(') => ')',
            _ => return Err(self.here()),
        };
        self.advance(1);
        let mut attributes = Vec::new();
        loop {
            self.skip_space();
            match self.bytes.get(self.at) {
                None => return Err(WktError::Truncated),
                Some(&close @ (b']' | b')')) => {
                    self.advance(1);
                    if char::from(close) != opened {
                        return Err(WktError::MismatchedDelimiter {
                            opened: if opened == ']' { '[' } else { '(' },
                            found: char::from(close),
                        });
                    }
                    return Ok(Node {
                        keyword,
                        attributes,
                    });
                }
                Some(_) => {}
            }
            attributes.push(self.attribute(depth)?);
            self.skip_space();
            match self.bytes.get(self.at) {
                Some(b',') => self.advance(1),
                Some(b']' | b')') => {}
                // The tolerance this module's documentation states: a bracketed attribute
                // followed by another token with no comma between, as §12.10.4's EXAMPLE 2 is
                // written. Anything else missing its separator is still refused.
                Some(b)
                    if b.is_ascii_alphabetic()
                        && matches!(attributes.last(), Some(Attribute::Node(_))) => {}
                None => return Err(WktError::Truncated),
                Some(_) => return Err(self.here()),
            }
        }
    }

    /// One attribute, the cursor on its first character.
    fn attribute(&mut self, depth: usize) -> Result<Attribute, WktError> {
        match self.bytes.get(self.at) {
            None => Err(WktError::Truncated),
            Some(b'"') => self.quoted().map(Attribute::Text),
            Some(b) if b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.') => {
                self.number().map(Attribute::Number)
            }
            Some(b) if b.is_ascii_alphabetic() => {
                // A word followed by a delimiter is a token; otherwise an enumeration.
                let start = self.at;
                let word = self.word();
                self.skip_space();
                if matches!(self.bytes.get(self.at), Some(b'[' | b'(')) {
                    self.at = start;
                    self.node(depth.saturating_add(1)).map(Attribute::Node)
                } else {
                    Ok(Attribute::Word(word))
                }
            }
            Some(_) => Err(self.here()),
        }
    }

    /// Quoted text, the cursor on its opening double quote (section 6.3.5's doubling read).
    fn quoted(&mut self) -> Result<String, WktError> {
        self.advance(1);
        let mut out = String::new();
        loop {
            let rest = self.text.get(self.at..).ok_or(WktError::Truncated)?;
            let Some(end) = rest.find('"') else {
                return Err(WktError::Truncated);
            };
            out.push_str(&rest[..end]);
            self.advance(end.saturating_add(1));
            if self.bytes.get(self.at) == Some(&b'"') {
                out.push('"');
                self.advance(1);
            } else {
                return Ok(out);
            }
        }
    }

    /// A number as ISO 19162 section 6.3.2 writes one: an optional sign, digits with an optional
    /// fraction, and an optional exponent introduced by `E`.
    fn number(&mut self) -> Result<f64, WktError> {
        let start = self.at;
        let digits = |parser: &mut Self| {
            let from = parser.at;
            while parser.bytes.get(parser.at).is_some_and(u8::is_ascii_digit) {
                parser.advance(1);
            }
            parser.at > from
        };
        if matches!(self.bytes.get(self.at), Some(b'+' | b'-')) {
            self.advance(1);
        }
        let whole = digits(self);
        let fraction = if self.bytes.get(self.at) == Some(&b'.') {
            self.advance(1);
            digits(self)
        } else {
            false
        };
        if !whole && !fraction {
            return Err(WktError::Number(self.text[start..self.at].to_owned()));
        }
        if matches!(self.bytes.get(self.at), Some(b'e' | b'E')) {
            self.advance(1);
            if matches!(self.bytes.get(self.at), Some(b'+' | b'-')) {
                self.advance(1);
            }
            if !digits(self) {
                return Err(WktError::Number(self.text[start..self.at].to_owned()));
            }
        }
        let literal = &self.text[start..self.at];
        literal
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| WktError::Number(literal.to_owned()))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// ISO 32000-2 §12.10.4's EXAMPLE 2, exactly as the clause prints it — including the missing
    /// comma after the second standard parallel that this module's one tolerance exists for.
    pub(crate) const CLAUSE_EXAMPLE_2: &str = r#"PROJCS["North_America_Albers_Equal_Area_Conic", GEOGCS["GCS_North_American_1983", DATUM["D_North_American_1983", SPHEROID["GRS_1980",6378137.0, 298.257222101] ], PRIMEM["Greenwich",0.0], UNIT["Degree",0.0174532925199433] ], PROJECTION["Albers"], PARAMETER["False_Easting",0.0], PARAMETER["False_Northing",0.0], PARAMETER["Central_Meridian",-96.0], PARAMETER["Standard_Parallel_1",20.0], PARAMETER["Standard_Parallel_2",60.0] PARAMETER["Latitude_Of_Origin",40.0], UNIT["Meter",1.0] ]"#;

    #[test]
    fn the_clauses_projected_example_parses_with_its_missing_comma() {
        let node = parse(CLAUSE_EXAMPLE_2).expect("the clause's own example");
        assert_eq!(node.keyword, "PROJCS");
        assert_eq!(node.children("PARAMETER").count(), 6);
        let base = node.child(&["GEOGCS"]).expect("a base system");
        let spheroid = base
            .child(&["DATUM"])
            .and_then(|datum| datum.child(&["SPHEROID"]))
            .expect("an ellipsoid");
        assert_eq!(spheroid.number(1), Some(6_378_137.0));
        assert_eq!(spheroid.number(2), Some(298.257_222_101));
    }

    #[test]
    fn keywords_and_enumerations_are_case_insensitive_and_text_is_not() {
        // ISO 19162 section 6.5.
        let node = parse(r#"axis["Easting",east]"#).expect("a token");
        assert_eq!(node.keyword, "AXIS");
        assert_eq!(node.attributes[0], Attribute::Text("Easting".to_owned()));
        assert_eq!(node.attributes[1], Attribute::Word("EAST".to_owned()));
    }

    #[test]
    fn a_doubled_quote_is_one_quote() {
        // ISO 19162 section 6.3.5's example form.
        let node = parse(r#"REMARK["30°25'20""N"]"#).expect("a token");
        assert_eq!(node.name(), Some("30°25'20\"N"));
    }

    #[test]
    fn parentheses_may_stand_for_brackets_but_not_beside_them() {
        // ISO 19162 section 6.4.
        assert!(parse(r#"UNIT("metre",1)"#).is_ok());
        assert_eq!(
            parse(r#"UNIT["metre",1)"#),
            Err(WktError::MismatchedDelimiter {
                opened: '[',
                found: ')'
            })
        );
    }

    #[test]
    fn numbers_take_the_grammar_of_section_6_3_2() {
        let node = parse("X[-1.5E-3,+2,.5,7.]").expect("numbers");
        assert_eq!(node.number(0), Some(-0.0015));
        assert_eq!(node.number(1), Some(2.0));
        assert_eq!(node.number(2), Some(0.5));
        assert_eq!(node.number(3), Some(7.0));
        assert!(parse("X[1E]").is_err());
        assert!(parse("X[-]").is_err());
    }

    #[test]
    fn a_missing_comma_is_refused_except_after_a_token() {
        assert!(parse(r#"UNIT["metre" 1]"#).is_err());
        assert!(parse("X[1 2]").is_err());
    }

    #[test]
    fn hostile_strings_are_refused_rather_than_followed() {
        let deep = "A[".repeat(100) + &"]".repeat(100);
        assert_eq!(parse(&deep), Err(WktError::TooDeep));
        assert_eq!(parse(r#"A["unterminated]"#), Err(WktError::Truncated));
        assert_eq!(parse("A[1"), Err(WktError::Truncated));
        assert_eq!(parse("A[1]B"), Err(WktError::Trailing(4)));
        assert!(parse("").is_err());
    }
}
