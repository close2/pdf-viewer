//! XFA 3.3's picture clauses, read for input parsing: a value a person typed, taken apart by the
//! field's picture and written in [`super::canonical`]'s form.
//!
//! Chapter 26, *Picture Clause Specification* (pages 1150–1186), is the grammar: alternates
//! separated by `|` (page 1159), each a run of literals and of `category(locale){symbols}`
//! clauses or one bare clause whose category is inferred from its symbols (page 1157), and
//! predefined clauses such as `date.long{}` read from the locale's patterns (pages 1155–1156).
//! The symbols read are the standard ones: the date symbols of page 1167, the time symbols of
//! page 1173, the numeric symbols of pages 1177–1178, the text symbols of page 1183, the null and
//! zero categories of page 1184, and the global `?`, `*` and `+` of page 1152.
//!
//! **What this module does not read is said, never guessed** (trap 5): the full-width and
//! ideographic symbols of pages 1161–1166, a week number (`w`, `WW`), a time zone (`Z`, `z`, whose
//! parsed value page 1176 makes depend on the platform's own zone), a numeric exponent (`E`), and
//! the `datetime` category, whose body XFA 3.3 gives no symbol rules — each is an `Err` naming the
//! symbol, which the save reports. A picture XFA 3.3 itself calls unacceptable for parsing — a
//! repeated kind of date or time symbol (page 1171), an hour of the meridiem without its meridiem
//! (page 1175), a reserved character (page 1153) — is an `Err` too. An input that no alternative
//! matches is `Ok(None)`: chapter 4's Rule 2 (page 163) then takes the text as already canonical.

use std::collections::HashSet;

use super::canonical::{self, Date, Time};
use super::locale::{LocaleSet, Names, Patterns};

/// A picture clause's category (page 1157's keywords).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Category {
    Date,
    Time,
    Num,
    Text,
    Null,
    Zero,
}

/// How the field's own value type breaks a tie between categories its picture's symbols allow
/// (page 1157 has the processor infer the category from the symbols the clause holds).
pub(crate) type Hint = Option<Category>;

/// The most picture text this module reads; a template's picture is a few dozen characters, so
/// this bounds a hostile file and is not a reading (principle 3).
const MAX_PICTURE: usize = 512;

/// The most variable-width date and time symbols one clause may hold. Page 1171 lets each kind
/// appear once, which makes seven a generous ceiling; past it the backtracking would be the file's
/// to choose.
const MAX_VARIABLE: usize = 12;

/// Parses `input` under `picture` in the prevailing `locale`: the canonical value of the first
/// alternative that matches, `Ok(None)` where none does.
///
/// # Errors
///
/// A sentence saying which part of the picture this module does not read, or why XFA 3.3 calls
/// the picture unacceptable for parsing.
pub(crate) fn canonicalize(
    picture: &str,
    hint: Hint,
    locale: &str,
    locales: &LocaleSet,
    input: &str,
) -> Result<Option<String>, String> {
    if picture.len() > MAX_PICTURE {
        return Err(format!(
            "its picture clause is {} bytes, past this reader's {MAX_PICTURE}",
            picture.len()
        ));
    }
    for alternative in split_unquoted(picture, '|') {
        let clauses = clauses(alternative.trim(), hint, locale, locales)?;
        if let Some(canonical) = parse_clauses(&clauses, input) {
            return Ok(Some(canonical));
        }
    }
    Ok(None)
}

/// One piece of an alternative: a clause of one category in one locale, or literal text between
/// clauses (page 1157's EXAMPLE 26.9 puts literals either side of the braces alike).
#[derive(Debug)]
enum Piece {
    Literal(String),
    Clause(Category, Vec<Token>),
}

/// The text split at every `separator` outside single quotes.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every offset is a position inside the picture text, bounded by MAX_PICTURE bytes"
)]
fn split_unquoted(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut quoted = false;
    let mut start = 0;
    for (at, c) in text.char_indices() {
        if c == '\'' {
            quoted = !quoted;
        } else if c == separator && !quoted {
            parts.push(&text[start..at]);
            start = at + c.len_utf8();
        }
    }
    parts.push(&text[start..]);
    parts
}

const KEYWORDS: [(&str, Option<Category>); 7] = [
    ("datetime", None),
    ("date", Some(Category::Date)),
    ("time", Some(Category::Time)),
    ("num", Some(Category::Num)),
    ("text", Some(Category::Text)),
    ("null", Some(Category::Null)),
    ("zero", Some(Category::Zero)),
];

/// A clause opener at the start of `rest`: its keyword's category (`None` for `datetime`), its
/// subcategory, its locale, and the length of the opener up to and including `{`.
type Opener<'a> = (
    Option<Option<Category>>,
    Option<&'a str>,
    Option<&'a str>,
    usize,
);

#[expect(
    clippy::arithmetic_side_effects,
    reason = "every offset is a position inside the picture text, bounded by MAX_PICTURE bytes"
)]
fn opener(rest: &str) -> Option<Opener<'_>> {
    let (category, mut at) = KEYWORDS
        .iter()
        .find(|(word, _)| rest.starts_with(word))
        .map_or((None, 0), |(word, category)| (Some(*category), word.len()));
    let mut subcategory = None;
    if category.is_some() && rest[at..].starts_with('.') {
        let tail = &rest[at + 1..];
        let length = tail
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(tail.len());
        subcategory = Some(&tail[..length]);
        at += 1 + length;
    }
    let mut locale = None;
    if rest[at..].starts_with('(') {
        let close = rest[at..].find(')')?;
        locale = Some(&rest[at + 1..at + close]);
        at += close + 1;
    }
    (rest[at..].starts_with('{') && (category.is_some() || locale.is_some())).then_some((
        category,
        subcategory,
        locale,
        at + 1,
    ))
}

/// Whether an opener stands anywhere in `text` outside single quotes.
fn has_opener(text: &str) -> bool {
    let mut quoted = false;
    text.char_indices().any(|(at, c)| {
        if c == '\'' {
            quoted = !quoted;
        }
        !quoted && opener(&text[at..]).is_some()
    })
}

/// An alternative read into its pieces: one bare clause, or clauses with literals between.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every offset is a position inside the picture text, bounded by MAX_PICTURE bytes"
)]
fn clauses(
    alternative: &str,
    hint: Hint,
    locale: &str,
    locales: &LocaleSet,
) -> Result<Vec<Piece>, String> {
    if !has_opener(alternative) {
        return Ok(vec![clause(alternative, None, hint, locale, locales)?]);
    }
    let mut pieces = Vec::new();
    let mut literal = String::new();
    let mut rest = alternative;
    while let Some(c) = rest.chars().next() {
        if c == '\'' {
            let (text, used) = quoted(rest)?;
            literal.push_str(&text);
            rest = &rest[used..];
            continue;
        }
        if let Some((category, subcategory, named, used)) = opener(rest) {
            let close = rest[used..]
                .find('}')
                .ok_or("its picture clause opens a brace it does not close")?;
            let clause_locale = named.unwrap_or(locale);
            if !literal.is_empty() {
                pieces.push(Piece::Literal(std::mem::take(&mut literal)));
            }
            let category = match category {
                Some(Some(category)) => Some(category),
                Some(None) => {
                    return Err(
                        "its picture clause uses the datetime category, whose symbols XFA 3.3 \
                         page 1157 lists without rules, so this program does not read it"
                            .to_owned(),
                    );
                }
                None => None,
            };
            let body = match subcategory {
                Some(sub) => predefined(category, sub, clause_locale, locales)?,
                None => rest[used..used + close].to_owned(),
            };
            pieces.push(clause(&body, category, hint, clause_locale, locales)?);
            rest = &rest[used + close + 1..];
            continue;
        }
        if c.is_whitespace() || matches!(c, ',' | '-' | ':' | '/' | '.') {
            literal.push(c);
            rest = &rest[c.len_utf8()..];
            continue;
        }
        return Err(format!(
            "its picture clause {alternative:?} holds {c:?} outside its braces, which XFA 3.3 \
             page 1153 reserves"
        ));
    }
    if !literal.is_empty() {
        pieces.push(Piece::Literal(literal));
    }
    Ok(pieces)
}

/// A quoted literal at the start of `text`: its content and the bytes it used. Page 1152: two
/// quotes stand for one.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every offset is a position inside the picture text, bounded by MAX_PICTURE bytes"
)]
fn quoted(text: &str) -> Result<(String, usize), String> {
    let mut out = String::new();
    let mut chars = text.char_indices().skip(1).peekable();
    while let Some((at, c)) = chars.next() {
        if c == '\'' {
            if chars.peek().is_some_and(|&(_, next)| next == '\'') {
                out.push('\'');
                chars.next();
            } else {
                return Ok((out, at + 1));
            }
        } else {
            out.push(c);
        }
    }
    if text == "''" {
        return Ok(("'".to_owned(), 2));
    }
    Err("its picture clause opens a quoted literal it does not close".to_owned())
}

/// A predefined clause's pattern from the locale (pages 1155–1156). `medium` is the locale set's
/// `med`; `default` is implementation-defined there, and this program takes `med`; `num.integer`
/// is the integer part of the `numeric` pattern and `num.decimal` the whole of it.
fn predefined(
    category: Option<Category>,
    sub: &str,
    locale: &str,
    locales: &LocaleSet,
) -> Result<String, String> {
    let date_or_time = |which| -> Result<String, String> {
        let name = match sub {
            "short" | "long" | "full" => sub,
            "medium" | "default" => "med",
            _ => {
                return Err(format!(
                    "its picture clause names no predefined {sub} picture"
                ));
            }
        };
        locales.pattern(locale, which, name).map(str::to_owned)
    };
    match category {
        Some(Category::Date) => date_or_time(Patterns::Date),
        Some(Category::Time) => date_or_time(Patterns::Time),
        Some(Category::Num) => {
            let (name, integer) = match sub {
                "integer" => ("numeric", true),
                "decimal" => ("numeric", false),
                "currency" | "percent" => (sub, false),
                _ => return Err(format!("its picture clause names no predefined num.{sub}")),
            };
            let pattern = locales.pattern(locale, Patterns::Number, name)?;
            Ok(if integer {
                pattern.split('.').next().unwrap_or(pattern).to_owned()
            } else {
                pattern.to_owned()
            })
        }
        _ => Err(format!(
            "its picture clause names a predefined {sub} picture of a category that has none"
        )),
    }
}

/// One matcher of a clause's body.
#[derive(Debug, Clone)]
enum Token {
    /// Text matched exactly and contributing nothing (page 1152).
    Literal(String),
    /// `?`: any one character.
    Any,
    /// `*` (zero or more) and `+` (one or more) whitespace characters.
    Spaces(usize),
    /// A date or time number of `min..=max` ASCII digits, into `field`, within `range`.
    Digits(Field, usize, usize, (u16, u16)),
    /// A name from a locale list, its index into `field` (offset by `base`).
    Name(Field, Vec<String>, u16),
    /// A numeric digit: `9` required; `z` and `8` optional; `Z` a digit or a space.
    NumDigit { required: bool, space: bool },
    /// The grouping separator, optional until a digit has been read.
    Grouping(String),
    /// The radix: the locale's for `.`; implied as well for `V` and `v` (page 1178).
    Radix(String, bool),
    /// `S` and `s`: a minus or a plus, `S` a space as well, either nothing (page 1182's `S9999`).
    Sign(String, bool),
    /// `CR`, `DB` (or spaces) and `cr`, `db` (or nothing): the text that marks a negative.
    Negative(&'static str, bool),
    /// `(` and `)`: a negative's brackets, or a space, or nothing.
    Bracket(bool),
    /// `$`, `$$` and `%`: a locale symbol; `%` makes the value a percentage.
    Symbol(String, bool),
    /// A text symbol: `A` alphabetic, `X` any, `O` and `0` alphanumeric, `9` a digit.
    Character(fn(char) -> bool),
}

/// Which part of a date or time a symbol supplies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Field {
    Day,
    DayOfYear,
    Month,
    Weekday,
    IsoWeekday,
    Year2,
    Year4,
    Era,
    Hour12,
    Hour11,
    Hour23,
    Hour24,
    Minute,
    Second,
    Millisecond,
    Meridiem,
}

impl Field {
    /// The kind page 1171 lets appear once in a clause read for parsing.
    const fn kind(self) -> u8 {
        match self {
            Self::Day | Self::DayOfYear => 0,
            Self::Month => 1,
            Self::Weekday | Self::IsoWeekday => 2,
            Self::Year2 | Self::Year4 => 3,
            Self::Era => 4,
            Self::Hour12 | Self::Hour11 | Self::Hour23 | Self::Hour24 => 5,
            Self::Minute => 6,
            Self::Second => 7,
            Self::Millisecond => 8,
            Self::Meridiem => 9,
        }
    }
}

/// A clause's body read under its category, or under the category its symbols allow.
fn clause(
    body: &str,
    category: Option<Category>,
    hint: Hint,
    locale: &str,
    locales: &LocaleSet,
) -> Result<Piece, String> {
    if let Some(category) = category {
        return Ok(Piece::Clause(
            category,
            tokens(body, category, locale, locales)?,
        ));
    }
    let mut readings = Vec::new();
    let mut errors = Vec::new();
    for category in [
        Category::Date,
        Category::Time,
        Category::Num,
        Category::Text,
    ] {
        match tokens(body, category, locale, locales) {
            Ok(tokens) if tokens.iter().any(is_symbol) => readings.push((category, tokens)),
            Ok(_) => {}
            Err(error) => errors.push((category, error)),
        }
    }
    let chosen = if readings.len() == 1 {
        readings.pop()
    } else {
        readings
            .iter()
            .position(|(c, _)| Some(*c) == hint)
            .map(|at| readings.swap_remove(at))
    };
    match chosen {
        Some((category, tokens)) => Ok(Piece::Clause(category, tokens)),
        // The field's own category says best why its picture does not read; without one, each
        // category's reason is given.
        None if readings.is_empty() => Err(match errors.iter().find(|(c, _)| Some(*c) == hint) {
            Some((_, error)) => error.clone(),
            None if errors.is_empty() => {
                format!("its picture clause {body:?} holds no picture symbol")
            }
            None => errors
                .iter()
                .map(|(_, error)| error.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        }),
        None => Err(format!(
            "its picture clause {body:?} reads as more than one category and states none, which \
             XFA 3.3 page 1157 makes an error"
        )),
    }
}

const fn is_symbol(token: &Token) -> bool {
    !matches!(token, Token::Literal(_) | Token::Any | Token::Spaces(_))
}

/// The separators page 1168 (and its kin for each category) treat as literals.
fn separator(c: char, category: Category) -> bool {
    match c {
        '-' | ':' | '/' | ' ' => true,
        ',' | '.' => category != Category::Num,
        _ => false,
    }
}

/// A clause's body as tokens of `category`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every offset is a position inside the picture text, bounded by MAX_PICTURE bytes"
)]
fn tokens(
    body: &str,
    category: Category,
    locale: &str,
    locales: &LocaleSet,
) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = body.chars().collect();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut at = 0;
    while let Some(&c) = chars.get(at) {
        let run = chars[at..].iter().take_while(|&&d| d == c).count();
        let mut used = 1;
        let token = match c {
            '\'' => {
                let text: String = chars[at..].iter().collect();
                let (literal, bytes) = quoted(&text)?;
                used = text[..bytes].chars().count();
                Token::Literal(literal)
            }
            '?' => Token::Any,
            '*' => Token::Spaces(0),
            '+' => Token::Spaces(1),
            c if separator(c, category) => Token::Literal(c.to_string()),
            _ => {
                let (token, length) = symbol(&chars[at..], run, category, locale, locales)?;
                used = length;
                if let Token::Digits(field, ..) | Token::Name(field, ..) = &token
                    && !seen.insert(field.kind())
                {
                    return Err(format!(
                        "its picture clause states the same kind of date or time symbol twice, \
                         which XFA 3.3 page 1171 makes unacceptable for parsing ({body:?})"
                    ));
                }
                token
            }
        };
        out.push(token);
        at += used;
    }
    if category == Category::Time {
        let meridiem = seen.contains(&Field::Meridiem.kind());
        let twelve = out
            .iter()
            .any(|t| matches!(t, Token::Digits(Field::Hour12 | Field::Hour11, ..)));
        if twelve && !meridiem {
            return Err(
                "its time picture states an hour of the meridiem without the meridiem, which \
                 XFA 3.3 page 1175 makes invalid for parsing"
                    .to_owned(),
            );
        }
    }
    let variable = out
        .iter()
        .filter(|t| {
            matches!(t, Token::Digits(_, min, max, _) if min != max)
                || matches!(t, Token::Name(..) | Token::Spaces(_))
        })
        .count();
    if variable > MAX_VARIABLE {
        return Err(format!(
            "its picture clause holds {variable} symbols of variable width, past this reader's \
             {MAX_VARIABLE}"
        ));
    }
    Ok(out)
}

fn unread(c: char, category: Category) -> String {
    format!(
        "its {} picture uses {c:?}, which this program's picture reader does not read",
        match category {
            Category::Date => "date",
            Category::Time => "time",
            Category::Num => "numeric",
            Category::Text => "text",
            Category::Null => "null",
            Category::Zero => "zero",
        }
    )
}

/// The token a run of `run` letters `c` makes in `category`, and the characters it used.
fn symbol(
    chars: &[char],
    run: usize,
    category: Category,
    locale: &str,
    locales: &LocaleSet,
) -> Result<(Token, usize), String> {
    let c = chars.first().copied().unwrap_or(' ');
    let digits = |field, min, max, range| Ok((Token::Digits(field, min, max, range), run));
    let names = |field, list, base| -> Result<(Token, usize), String> {
        Ok((
            Token::Name(field, locales.names(locale, list)?.to_vec(), base),
            run,
        ))
    };
    match (category, c, run) {
        (Category::Date, 'D', 1) => digits(Field::Day, 1, 2, (1, 31)),
        (Category::Date, 'D', 2) => digits(Field::Day, 2, 2, (1, 31)),
        (Category::Date, 'J', 1) => digits(Field::DayOfYear, 1, 3, (1, 366)),
        (Category::Date, 'J', 3) => digits(Field::DayOfYear, 3, 3, (1, 366)),
        (Category::Date, 'M', 1) => digits(Field::Month, 1, 2, (1, 12)),
        (Category::Date, 'M', 2) => digits(Field::Month, 2, 2, (1, 12)),
        (Category::Date, 'M', 3) => names(Field::Month, Names::MonthsAbbr, 1),
        (Category::Date, 'M', 4) => names(Field::Month, Names::Months, 1),
        (Category::Date, 'E', 1) => digits(Field::Weekday, 1, 1, (1, 7)),
        (Category::Date, 'E', 3) => names(Field::Weekday, Names::DaysAbbr, 1),
        (Category::Date, 'E', 4) => names(Field::Weekday, Names::Days, 1),
        (Category::Date, 'e', 1) => digits(Field::IsoWeekday, 1, 1, (1, 7)),
        (Category::Date, 'G', 1) => names(Field::Era, Names::Eras, 0),
        (Category::Date, 'Y', 2) => digits(Field::Year2, 2, 2, (0, 99)),
        (Category::Date, 'Y', 4) => digits(Field::Year4, 4, 4, (0, 9999)),
        (Category::Time, 'h', 1) => digits(Field::Hour12, 1, 2, (1, 12)),
        (Category::Time, 'h', 2) => digits(Field::Hour12, 2, 2, (1, 12)),
        (Category::Time, 'k', 1) => digits(Field::Hour11, 1, 2, (0, 11)),
        (Category::Time, 'k', 2) => digits(Field::Hour11, 2, 2, (0, 11)),
        (Category::Time, 'H', 1) => digits(Field::Hour23, 1, 2, (0, 23)),
        (Category::Time, 'H', 2) => digits(Field::Hour23, 2, 2, (0, 23)),
        (Category::Time, 'K', 1) => digits(Field::Hour24, 1, 2, (1, 24)),
        (Category::Time, 'K', 2) => digits(Field::Hour24, 2, 2, (1, 24)),
        (Category::Time, 'M', 1) => digits(Field::Minute, 1, 2, (0, 59)),
        (Category::Time, 'M', 2) => digits(Field::Minute, 2, 2, (0, 59)),
        (Category::Time, 'S', 1) => digits(Field::Second, 1, 2, (0, 59)),
        (Category::Time, 'S', 2) => digits(Field::Second, 2, 2, (0, 59)),
        (Category::Time, 'F', 3) => digits(Field::Millisecond, 3, 3, (0, 999)),
        (Category::Time, 'A', 1) => names(Field::Meridiem, Names::Meridiems, 0),
        (Category::Num, _, _) => num_symbol(chars, locale, locales),
        (Category::Text, 'A', _) => Ok((Token::Character(char::is_alphabetic), 1)),
        (Category::Text, 'X', _) => Ok((Token::Character(|_| true), 1)),
        (Category::Text, 'O' | '0', _) => Ok((Token::Character(char::is_alphanumeric), 1)),
        (Category::Text, '9', _) => Ok((Token::Character(|c| c.is_ascii_digit()), 1)),
        _ => Err(unread(c, category)),
    }
}

/// One numeric symbol of pages 1177–1178 at the start of `chars`.
fn num_symbol(chars: &[char], locale: &str, locales: &LocaleSet) -> Result<(Token, usize), String> {
    let c = chars.first().copied().unwrap_or(' ');
    let next = chars.get(1).copied();
    let symbol = |name| locales.symbol(locale, false, name).map(str::to_owned);
    Ok(match (c, next) {
        ('9', _) => (
            Token::NumDigit {
                required: true,
                space: false,
            },
            1,
        ),
        ('z' | '8', _) => (
            Token::NumDigit {
                required: false,
                space: false,
            },
            1,
        ),
        ('Z', _) => (
            Token::NumDigit {
                required: false,
                space: true,
            },
            1,
        ),
        (',', _) => (Token::Grouping(symbol("grouping")?), 1),
        ('.', _) => (Token::Radix(symbol("decimal")?, false), 1),
        ('V' | 'v', _) => (Token::Radix(symbol("decimal")?, true), 1),
        ('S', _) => (Token::Sign(symbol("minus")?, true), 1),
        ('s', _) => (Token::Sign(symbol("minus")?, false), 1),
        ('C', Some('R')) => (Token::Negative("CR", true), 2),
        ('c', Some('r')) => (Token::Negative("CR", false), 2),
        ('D', Some('B')) => (Token::Negative("DB", true), 2),
        ('d', Some('b')) => (Token::Negative("DB", false), 2),
        ('(', _) => (Token::Bracket(true), 1),
        (')', _) => (Token::Bracket(false), 1),
        ('$', Some('$')) => (
            Token::Symbol(locales.symbol(locale, true, "isoname")?.to_owned(), false),
            2,
        ),
        ('$', _) => (
            Token::Symbol(locales.symbol(locale, true, "symbol")?.to_owned(), false),
            1,
        ),
        ('%', _) => (Token::Symbol(symbol("percent")?, true), 1),
        _ => return Err(unread(c, Category::Num)),
    })
}

/// What a match has read so far.
#[derive(Debug, Clone, Default)]
struct Read {
    fields: Vec<(Field, u16)>,
    negative: bool,
    whole: String,
    fraction: Option<String>,
    percent: bool,
    bracket_open: bool,
    text: String,
    /// The canonical values of the clauses already read, by category.
    done: Vec<(Category, String)>,
}

impl Read {
    fn field(&self, wanted: Field) -> Option<u16> {
        self.fields
            .iter()
            .find(|(f, _)| *f == wanted)
            .map(|&(_, v)| v)
    }
}

/// Matches `input` against an alternative's pieces, in order, and writes its canonical value.
fn parse_clauses(pieces: &[Piece], input: &str) -> Option<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut walker = Walker {
        pieces,
        input: &chars,
        failed: HashSet::new(),
    };
    let read = walker.walk(0, 0, 0, Read::default())?;
    let of = |wanted| read.done.iter().find(|(c, _)| *c == wanted).map(|(_, v)| v);
    match (of(Category::Date), of(Category::Time)) {
        // Page 1157: a date clause and a time clause compounded read a date-time (page 1047).
        (Some(date), Some(time)) => Some(format!("{date}T{time}")),
        _ => read.done.first().map(|(_, value)| value.clone()),
    }
}

/// A failed position of a numeric or text clause: the piece, the token, the character, and the
/// state the rest of the match depends on — whether an integer digit and a fraction digit have
/// been read, and whether a bracket is open.
type Position = (usize, usize, usize, bool, bool, bool);

/// One way a token may match: the characters it takes, and how it changes what is read.
type Choice<'a> = (usize, Box<dyn Fn(&mut Read) + 'a>);

/// The backtracking match over one alternative. A numeric or text clause's failures are
/// remembered by [`Position`], so a run of optional digits costs the product of its length and
/// the input's rather than a power; a date or time clause's are not, because its validity is
/// decided once the whole clause is read, and [`MAX_VARIABLE`] bounds it.
struct Walker<'a> {
    pieces: &'a [Piece],
    input: &'a [char],
    failed: HashSet<Position>,
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "positions are inside the input, piece and token indices inside their slices, and \
              a name's index is under the thirteen a locale list holds"
)]
impl Walker<'_> {
    /// The input's next `length` characters as text, where there are that many.
    fn ahead(&self, at: usize, length: usize) -> Option<String> {
        self.input.get(at..at + length).map(|s| s.iter().collect())
    }

    /// Whether `text` stands at `at`; the characters it takes.
    fn stands(&self, at: usize, text: &str) -> Option<usize> {
        let length = text.chars().count();
        (length > 0 && self.ahead(at, length).as_deref() == Some(text)).then_some(length)
    }

    /// Piece `p`, token `t` of it, at character `at`.
    fn walk(&mut self, p: usize, t: usize, at: usize, read: Read) -> Option<Read> {
        let Some(piece) = self.pieces.get(p) else {
            return (at == self.input.len()).then_some(read);
        };
        match piece {
            Piece::Literal(text) => {
                let length = self.stands(at, text)?;
                self.walk(p + 1, 0, at + length, read)
            }
            Piece::Clause(category, tokens) => {
                let memo = matches!(category, Category::Num | Category::Text);
                let key = (
                    p,
                    t,
                    at,
                    read.whole.is_empty(),
                    read.fraction.as_deref().is_some_and(|f| !f.is_empty()),
                    read.bracket_open,
                );
                if memo && self.failed.contains(&key) {
                    return None;
                }
                let result = self.step(p, *category, tokens, t, at, read);
                if memo && result.is_none() {
                    self.failed.insert(key);
                }
                result
            }
        }
    }

    /// Each choice of `options` — the characters a token takes, and how it changes what is
    /// read — tried in order.
    fn first(
        &mut self,
        p: usize,
        t: usize,
        at: usize,
        read: &Read,
        options: Vec<Choice<'_>>,
    ) -> Option<Read> {
        for (width, change) in options {
            let mut read = read.clone();
            change(&mut read);
            if let Some(found) = self.walk(p, t + 1, at + width, read) {
                return Some(found);
            }
        }
        None
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one arm per token of pages 1152, 1167, 1173, 1177–1178 and 1183, read together"
    )]
    fn step(
        &mut self,
        p: usize,
        category: Category,
        tokens: &[Token],
        t: usize,
        at: usize,
        read: Read,
    ) -> Option<Read> {
        let Some(token) = tokens.get(t) else {
            // The clause is read; its value is decided and the next piece begins.
            let value = finish(category, &read)?;
            let mut done = read.done;
            done.push((category, value));
            return self.walk(
                p + 1,
                0,
                at,
                Read {
                    done,
                    ..Read::default()
                },
            );
        };
        let here = self.input.get(at).copied();
        let keep = || -> Box<dyn Fn(&mut Read)> { Box::new(|_| {}) };
        let mut options: Vec<Choice<'_>> = Vec::new();
        match token {
            Token::Literal(text) => {
                options.push((self.stands(at, text)?, keep()));
            }
            Token::Any => {
                here?;
                options.push((1, keep()));
            }
            Token::Spaces(min) => {
                let available = self.input[at..]
                    .iter()
                    .take_while(|c| c.is_whitespace())
                    .count();
                for n in *min..=available {
                    options.push((n, keep()));
                }
            }
            Token::Digits(field, min, max, (low, high)) => {
                for n in (*min..=*max).rev() {
                    let Some(digits) = self.ahead(at, n) else {
                        continue;
                    };
                    if !digits.chars().all(|c| c.is_ascii_digit()) {
                        continue;
                    }
                    let Ok(value) = digits.parse::<u16>() else {
                        continue;
                    };
                    if (*low..=*high).contains(&value) {
                        let field = *field;
                        options.push((
                            n,
                            Box::new(move |r: &mut Read| r.fields.push((field, value))),
                        ));
                    }
                }
            }
            Token::Name(field, names, base) => {
                for (index, name) in names.iter().enumerate() {
                    let length = name.chars().count();
                    let matches = self.ahead(at, length).is_some_and(|slice| {
                        length > 0 && slice.to_lowercase() == name.to_lowercase()
                    });
                    let Ok(index) = u16::try_from(index) else {
                        continue;
                    };
                    if matches {
                        let (field, value) = (*field, index + *base);
                        options.push((
                            length,
                            Box::new(move |r: &mut Read| r.fields.push((field, value))),
                        ));
                    }
                }
            }
            Token::NumDigit { required, space } => {
                if let Some(digit) = here.filter(char::is_ascii_digit) {
                    options.push((
                        1,
                        Box::new(move |r: &mut Read| match &mut r.fraction {
                            Some(fraction) => fraction.push(digit),
                            None => r.whole.push(digit),
                        }),
                    ));
                }
                if *space && here == Some(' ') {
                    options.push((1, keep()));
                }
                if !required {
                    options.push((0, keep()));
                }
            }
            Token::Grouping(symbol) => {
                if let Some(length) = self.stands(at, symbol) {
                    options.push((length, keep()));
                }
                if read.whole.is_empty() {
                    options.push((0, keep()));
                }
            }
            Token::Radix(symbol, implied) => {
                let start = || -> Box<dyn Fn(&mut Read)> {
                    Box::new(|r: &mut Read| r.fraction = Some(String::new()))
                };
                if let Some(length) = self.stands(at, symbol) {
                    options.push((length, start()));
                }
                if *implied {
                    options.push((0, start()));
                }
                // A radix may be absent where every fraction symbol after it is optional —
                // page 1179's `z` that leaves the radix out of a number that has none, read
                // for parsing as for formatting.
                if tokens[t + 1..]
                    .iter()
                    .all(|t| !matches!(t, Token::NumDigit { required: true, .. }))
                {
                    options.push((0, keep()));
                }
            }
            Token::Sign(minus, space) => {
                if let Some(length) = self.stands(at, minus) {
                    options.push((length, Box::new(|r: &mut Read| r.negative = true)));
                }
                if here == Some('+') || (*space && here == Some(' ')) {
                    options.push((1, keep()));
                }
                options.push((0, keep()));
            }
            Token::Negative(text, spaces) => {
                if self.stands(at, text).is_some() {
                    options.push((2, Box::new(|r: &mut Read| r.negative = true)));
                }
                if *spaces && self.stands(at, "  ").is_some() {
                    options.push((2, keep()));
                }
                options.push((0, keep()));
            }
            Token::Bracket(true) => {
                if here == Some('(') {
                    options.push((
                        1,
                        Box::new(|r: &mut Read| {
                            r.bracket_open = true;
                            r.negative = true;
                        }),
                    ));
                }
                if here == Some(' ') {
                    options.push((1, keep()));
                }
                options.push((0, keep()));
            }
            Token::Bracket(false) => {
                if read.bracket_open {
                    if here == Some(')') {
                        options.push((1, Box::new(|r: &mut Read| r.bracket_open = false)));
                    }
                } else {
                    if here == Some(' ') {
                        options.push((1, keep()));
                    }
                    options.push((0, keep()));
                }
            }
            Token::Symbol(symbol, percent) => {
                let length = self.stands(at, symbol)?;
                let percent = *percent;
                options.push((length, Box::new(move |r: &mut Read| r.percent |= percent)));
            }
            Token::Character(accepts) => {
                let c = here.filter(|&c| accepts(c))?;
                options.push((1, Box::new(move |r: &mut Read| r.text.push(c))));
            }
        }
        self.first(p, t, at, &read, options)
    }
}

/// A clause's canonical value from what it read, `None` where what it read is no value of its
/// kind: a date that does not exist, a weekday that disagrees with its date, a year before the
/// common era, a number with no digit.
fn finish(category: Category, read: &Read) -> Option<String> {
    match category {
        Category::Date => date(read).map(Date::write),
        Category::Time => time(read).map(Time::write),
        Category::Num => {
            if read.whole.is_empty() && read.fraction.as_deref().is_none_or(str::is_empty) {
                return None;
            }
            if read.bracket_open {
                return None;
            }
            let (whole, fraction) = if read.percent {
                hundredth(&read.whole, read.fraction.as_deref().unwrap_or(""))
            } else {
                (read.whole.clone(), read.fraction.clone())
            };
            Some(canonical::number(
                read.negative,
                &whole,
                fraction.as_deref(),
            ))
        }
        Category::Text => Some(read.text.clone()),
        // Page 1184: a null clause matches a null value, which the data writes as nothing, and
        // a zero clause a zero.
        Category::Null => Some(String::new()),
        Category::Zero => Some("0".to_owned()),
    }
}

/// A percentage's digits divided by a hundred. Page 1181's prose says the value is multiplied
/// on input and divided on output; its EXAMPLE 26.27 reads `12%` as `0.12` and `1,234.5%` as
/// `12.345`, which is the only reading under which `12%` means twelve hundredths, so this follows
/// the example.
fn hundredth(whole: &str, fraction: &str) -> (String, Option<String>) {
    let padded = format!("{whole:0>3}");
    // `padded` holds at least three ASCII digits.
    let split = padded.len().saturating_sub(2);
    (
        padded[..split].to_owned(),
        Some(format!("{}{fraction}", &padded[split..])),
    )
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "a two-digit year is under 100 and a weekday at most 7"
)]
fn date(read: &Read) -> Option<Date> {
    // Page 1171's century split: 00–29 are 2000–2029, 30–99 are 1930–1999.
    let year = match (read.field(Field::Year4), read.field(Field::Year2)) {
        (Some(year), _) => year,
        (None, Some(short)) if short < 30 => 2000 + short,
        (None, Some(short)) => 1900 + short,
        (None, None) => return None,
    };
    // Era 0 is the first of the locale's `eraNames`, before the common era; chapter 24 writes no
    // such year.
    if read.field(Field::Era) == Some(0) {
        return None;
    }
    let (month, day) = match (
        read.field(Field::Month),
        read.field(Field::Day),
        read.field(Field::DayOfYear),
    ) {
        (_, None, Some(ordinal)) => {
            let (month, day) = canonical::from_day_of_year(year, ordinal)?;
            (Some(month), Some(day))
        }
        (Some(month), day, None) => (
            Some(u8::try_from(month).ok()?),
            day.map(u8::try_from).transpose().ok()?,
        ),
        (None, None, None) => (None, None),
        _ => return None,
    };
    if let (Some(month), Some(day)) = (month, day) {
        if day > canonical::days_in_month(year, month) {
            return None;
        }
        let weekday = canonical::weekday(year, month, day);
        if read
            .field(Field::Weekday)
            .is_some_and(|stated| stated != u16::from(weekday))
        {
            return None;
        }
        // `e` counts from Monday (page 1168).
        if read
            .field(Field::IsoWeekday)
            .is_some_and(|stated| stated != u16::from((weekday + 5) % 7 + 1))
        {
            return None;
        }
    } else if read.field(Field::Weekday).is_some() || read.field(Field::IsoWeekday).is_some() {
        return None;
    }
    if month.is_none() && day.is_none() && read.field(Field::Day).is_some() {
        return None;
    }
    Some(Date { year, month, day })
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "an hour of the meridiem is at most 12 and of the day at most 24"
)]
fn time(read: &Read) -> Option<Time> {
    let pm = read.field(Field::Meridiem) == Some(1);
    let hour = match (
        read.field(Field::Hour23),
        read.field(Field::Hour24),
        read.field(Field::Hour12),
        read.field(Field::Hour11),
    ) {
        (Some(hour), ..) => hour,
        (None, Some(hour), ..) => hour % 24,
        (None, None, Some(hour), _) => hour % 12 + if pm { 12 } else { 0 },
        (None, None, None, Some(hour)) => hour + if pm { 12 } else { 0 },
        _ => return None,
    };
    let minute = read.field(Field::Minute);
    let second = read.field(Field::Second).filter(|_| minute.is_some());
    let millisecond = read.field(Field::Millisecond).filter(|_| second.is_some());
    Some(Time {
        hour: u8::try_from(hour).ok()?,
        minute: minute.map(u8::try_from).transpose().ok()?,
        second: second.map(u8::try_from).transpose().ok()?,
        millisecond,
    })
}

#[cfg(test)]
mod tests {
    use super::{Category, canonicalize};
    use crate::xfa::locale::LocaleSet;
    use crate::xfa::xml::parse;

    /// An `en_US` and an `fr_FR` locale, spelled with the symbols chapter 26's examples assume.
    fn locales() -> LocaleSet {
        let text = "<localeSet>\
            <locale name=\"en_US\"><calendarSymbols>\
            <monthNames><month>January</month><month>February</month><month>March</month>\
            <month>April</month><month>May</month><month>June</month><month>July</month>\
            <month>August</month><month>September</month><month>October</month>\
            <month>November</month><month>December</month></monthNames>\
            <monthNames abbr=\"1\"><month>Jan</month><month>Feb</month><month>Mar</month>\
            <month>Apr</month><month>May</month><month>Jun</month><month>Jul</month>\
            <month>Aug</month><month>Sep</month><month>Oct</month><month>Nov</month>\
            <month>Dec</month></monthNames>\
            <dayNames><day>Sunday</day><day>Monday</day><day>Tuesday</day><day>Wednesday</day>\
            <day>Thursday</day><day>Friday</day><day>Saturday</day></dayNames>\
            <meridiemNames><meridiem>AM</meridiem><meridiem>PM</meridiem></meridiemNames>\
            <eraNames><era>BC</era><era>AD</era></eraNames></calendarSymbols>\
            <datePatterns><datePattern name=\"med\">MMM D, YYYY</datePattern></datePatterns>\
            <numberPatterns><numberPattern name=\"numeric\">z,zzz,zz9.zzz</numberPattern>\
            </numberPatterns>\
            <numberSymbols><numberSymbol name=\"decimal\">.</numberSymbol>\
            <numberSymbol name=\"grouping\">,</numberSymbol><numberSymbol name=\"percent\">%</numberSymbol>\
            <numberSymbol name=\"minus\">-</numberSymbol><numberSymbol name=\"zero\">0</numberSymbol>\
            </numberSymbols><currencySymbols><currencySymbol name=\"symbol\">$</currencySymbol>\
            <currencySymbol name=\"isoname\">USD</currencySymbol></currencySymbols></locale>\
            <locale name=\"fr_FR\"><calendarSymbols><monthNames><month>janvier</month>\
            <month>f\u{e9}vrier</month><month>mars</month><month>avril</month><month>mai</month>\
            <month>juin</month><month>juillet</month><month>ao\u{fb}t</month><month>septembre</month>\
            <month>octobre</month><month>novembre</month><month>d\u{e9}cembre</month></monthNames>\
            </calendarSymbols><numberSymbols><numberSymbol name=\"decimal\">,</numberSymbol>\
            <numberSymbol name=\"grouping\"> </numberSymbol><numberSymbol name=\"minus\">-</numberSymbol>\
            </numberSymbols></locale></localeSet>";
        let tree = parse(text.as_bytes()).expect("well formed");
        LocaleSet::read(Some((&tree, 0)), None)
    }

    fn parsed(picture: &str, locale: &str, input: &str) -> Option<String> {
        canonicalize(picture, None, locale, &locales(), input).expect("a picture this reads")
    }

    /// Chapter 26's numeric input-parsing table (EXAMPLE 26.29, page 1182), its `%` table
    /// (EXAMPLE 26.27, page 1181), its `S9999` sentence (page 1182), and chapter 4's French
    /// EXAMPLE 4.40 (page 163) whose canonical value is "1234.56".
    #[test]
    fn the_chapters_numeric_examples_parse_as_printed() {
        for (picture, input, canonical) in [
            ("99V99", "1050", "10.50"),
            ("99V99", "3125", "31.25"),
            ("z999", "150", "150"),
            ("z999", "0150", "150"),
            ("z,zz9.99", "10.50", "10.50"),
            ("z,zz9.99", "3,125.00", "3125.00"),
            ("$z,zz9.99DB", "$1,234.00", "1234.00"),
            ("$z,zz9.99DB", "$1,234.00DB", "-1234.00"),
            ("zz9%", "12%", "0.12"),
            // Page 1181's second row types "1,234.5%" against `.99`, whose two `9`s page 1177
            // makes required digits; the row is taken with both typed.
            ("zzz,zz9.99%", "1,234.50%", "12.3450"),
            ("S9999", "-5000", "-5000"),
            ("S9999", "+5000", "5000"),
            ("S9999", "5000", "5000"),
            ("zz,zz9.99", "2,157.50", "2157.50"),
        ] {
            assert_eq!(
                parsed(picture, "en_US", input).as_deref(),
                Some(canonical),
                "{picture} {input}"
            );
        }
        assert_eq!(
            parsed("z,zz9.99", "fr_FR", "1 234,56").as_deref(),
            Some("1234.56")
        );
        // Page 163: "deux" does not match, and neither does the already canonical "1234.56";
        // Rule 2 then copies each as it is, which is the caller's to do.
        assert_eq!(parsed("z,zz9.99", "fr_FR", "deux"), None);
        assert_eq!(parsed("z,zz9.99", "fr_FR", "1234.56"), None);
    }

    /// Page 1172's date input-parsing table (EXAMPLE 26.20), page 164's French EXAMPLES 4.41 and
    /// 4.42, and page 1171's century split.
    #[test]
    fn the_chapters_date_examples_parse_as_printed() {
        assert_eq!(
            parsed("MM/D/YY", "en_US", "12/2/99").as_deref(),
            Some("1999-12-02")
        );
        assert_eq!(
            parsed("MMM D, YYYY", "en_US", "Jan 10, 1999").as_deref(),
            Some("1999-01-10")
        );
        assert_eq!(
            parsed("D-MMMM-YYYY", "fr_FR", "12-janvier-2004").as_deref(),
            Some("2004-01-12")
        );
        assert_eq!(
            parsed("D-MMMM-YYYY|D-M-YYYY", "fr_FR", "12-01-2004").as_deref(),
            Some("2004-01-12")
        );
        assert_eq!(
            parsed("MM/DD/YY", "en_US", "01/02/29").as_deref(),
            Some("2029-01-02")
        );
        assert_eq!(
            parsed("date(fr_FR){D MMMM YYYY}", "en_US", "30 octobre 2004").as_deref(),
            Some("2004-10-30")
        );
        assert_eq!(
            parsed("EEEE, MMMM D, YYYY", "en_US", "Saturday, January 1, 2000").as_deref(),
            Some("2000-01-01")
        );
        // A date that does not exist, and a weekday that disagrees with its date, match nothing.
        assert_eq!(parsed("MM/DD/YYYY", "en_US", "02/30/2004"), None);
        assert_eq!(
            parsed("EEEE, MMMM D, YYYY", "en_US", "Monday, January 1, 2000"),
            None
        );
        assert_eq!(
            parsed("date.medium{}", "en_US", "Jul 1, 2006").as_deref(),
            Some("2006-07-01")
        );
    }

    /// Page 1176's time input-parsing table (EXAMPLE 26.23) where its result has no zone, the
    /// compound date and time of page 1157, and the text mask of page 1183 (EXAMPLE 26.32).
    #[test]
    fn times_compounds_and_text_masks_parse() {
        assert_eq!(parsed("HH:MM", "en_US", "18:00").as_deref(), Some("18:00"));
        assert_eq!(
            parsed("H.MM 'Uhr'", "en_US", "12.59 Uhr").as_deref(),
            Some("12:59")
        );
        assert_eq!(
            parsed("h:MM:SS A", "en_US", "1:05:10 PM").as_deref(),
            Some("13:05:10")
        );
        assert_eq!(
            parsed("date{YYYY-MM-DD} time{HH:MM}", "en_US", "1999-07-16 10:30").as_deref(),
            Some("1999-07-16T10:30")
        );
        assert_eq!(
            parsed("AAA-9999-X", "en_US", "ABC-1234-5").as_deref(),
            Some("ABC12345")
        );
        assert_eq!(
            parsed("null{}|text{999*9999}", "en_US", "555 1212").as_deref(),
            Some("5551212")
        );
        assert_eq!(
            parsed("null{}|text{999*9999}", "en_US", "").as_deref(),
            Some("")
        );
    }

    /// What is not read is said: a zone, an exponent, a week, the `datetime` category, an hour
    /// of the meridiem without its meridiem, a repeated kind of symbol, a locale the packet lacks.
    #[test]
    fn what_this_does_not_read_is_an_error() {
        let set = locales();
        for (picture, locale) in [
            ("HH:MM Z", "en_US"),
            ("99.999E", "en_US"),
            ("YYYY-'W'WW-e", "en_US"),
            ("datetime{YYYY}", "en_US"),
            ("h:MM", "en_US"),
            ("DD/MM/DD", "en_US"),
            ("MMMM D, YYYY", "de_DE"),
            ("999-9999", "en_US"),
        ] {
            assert!(
                canonicalize(picture, None, locale, &set, "x").is_err(),
                "{picture}"
            );
        }
        // A tie broken by the field's own type: text for a text field, a number for a number.
        assert_eq!(
            canonicalize("999-9999", Some(Category::Text), "en_US", &set, "555-1212"),
            Ok(Some("5551212".to_owned()))
        );
        assert_eq!(
            canonicalize("999-9999", Some(Category::Num), "en_US", &set, "555-1212"),
            Ok(Some("5551212".to_owned()))
        );
    }
}
