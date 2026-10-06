//! The `AF*` form library, re-implemented in Rust: what a field's one-call script does, with no
//! script engine.
//!
//! # Why this exists, and why it is not ECMAScript
//!
//! ISO 32000-2 §12.6.3's Table 199 gives a form field four ECMAScript triggers. Its own text, row
//! by row:
//!
//! > (Optional; PDF 1.3) An ECMAScript action that shall be performed when the user modifies a
//! > character in a text field or combo box or modifies the selection in a scrollable list box.
//! > This action may check the added text for validity and reject or modify it.
//!
//! > (Optional; PDF 1.3) An ECMAScript action that shall be performed before the field is formatted
//! > to display its value. This action may modify the field's value before formatting.
//!
//! > (Optional; PDF 1.3) An ECMAScript action that shall be performed when the field's value is
//! > changed. This action may check the new value for validity. (The name V stands for
//! > "validate.")
//!
//! What the script in such an action says is, in real documents, nearly always *one call* of a
//! library Adobe ships with its viewer — `AFNumber_Format(2, 0, 0, 0, "$", true)`,
//! `AFDate_FormatEx("mm/dd/yyyy")`, `AFSimple_Calculate("SUM", ["Line1", "Line2"])`. RFC 0008
//! section 3.5 counted them and section 4.1 drew the line this module sits behind: **Tier 0**
//! runs that library natively where a script is textually one such call with literal arguments
//! ([`Call::parse`]), constructs no engine, and reports every other script as one this tier does
//! not run. The owner accepted the RFC in `doc/questions/A193`, with the rule that the target is
//! as much of the library as it defines and never only what the census found called.
//!
//! # Where the definitions come from, and what they are
//!
//! **Not from ISO 32000-2**, which names no library, and **not from Adobe's *JavaScript for Acrobat
//! API Reference***, which the owner made the working source for the API (memory of 2026-08-28)
//! and which documents no `AF*` function at all (`doc/todo/56`, measured). Adobe's only statement
//! about them is an argument menu in a different book, the *Interapplication Communication API
//! Reference*, under the Acrobat Forms plug-in's `Field.SetJavaScriptAction` — the four special
//! formats, the fourteen date pictures, the four time formats with an example each, the six
//! `AFNumber` arguments and the four negative styles, `AFSimple_Calculate`'s five functions — and
//! the picture and mask languages the functions are built over are the *JavaScript* reference's
//! own `util.printd`, `util.scand`, `util.printx` and `util.printf`, on its "util methods" page,
//! documented with examples. Both are read at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`,
//! cited by name and section and never quoted (`doc/third-party-data.md`).
//!
//! Everything those pages leave unsaid is a **documented choice** under `CLAUDE.md` principle 5,
//! and ADR 1578 is the list: how a number is read, how it is rounded, what a negative looks like in
//! each style, which characters a keystroke accepts, how a date is read through a picture, what an
//! empty field calculates as. pdf.js's `src/scripting_api/aform.js` is evidence about the size of
//! the work and was read as such; no rule here is taken from it because it is there.
//!
//! # Shape
//!
//! [`Call`] is one parsed script. Its four operations are Table 199's four triggers, each taking
//! the event the trigger raises and answering what the script did — [`Call::format`],
//! [`Call::keystroke`], [`Call::validate`], [`Call::calculate`] — and a function called at a
//! trigger it does not serve is a [`Refusal`] with a sentence rather than a silent nothing. The
//! library's own helpers are public functions beside them: [`make_number`] (`AFMakeNumber`),
//! [`extract_nums`] (`AFExtractNums`), [`merge_change`] (`AFMergeChange`), [`parse_date`]
//! (`AFParseDateEx`), and the three `util` methods the formats are written with, [`print_date`],
//! [`print_mask`] and [`number_text`]. Nothing here reads a document: the field's script and its
//! values are handed in by `crate::view`, which is where Table 199's triggers are raised (ADR
//! 1579).

mod call;
mod date;
mod mask;
mod number;
pub mod site;

pub use call::{Call, Literal, NotOneCall};
pub use date::{DateTime, parse_date, print_date};
pub use mask::print_mask;
pub use number::{extract_nums, make_number, number_text};

use number::{Arithmetic, MAX_PLACES, Negative, Separators};

/// Table 199's four triggers, which the library's functions each serve one of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Trigger {
    /// `/K`: a character typed, or the value committed.
    Keystroke,
    /// `/F`: the value about to be displayed.
    Format,
    /// `/V`: the value changed.
    Validate,
    /// `/C`: another field's value changed, so this one is recalculated.
    Calculate,
}

impl Trigger {
    /// Table 199's key for this trigger.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Keystroke => "K",
            Self::Format => "F",
            Self::Validate => "V",
            Self::Calculate => "C",
        }
    }

    /// What the trigger is called in a sentence a person reads.
    #[must_use]
    pub fn noun(self) -> &'static str {
        match self {
            Self::Keystroke => "keystroke",
            Self::Format => "format",
            Self::Validate => "validate",
            Self::Calculate => "calculate",
        }
    }
}

/// Every function of the library this module defines, by its ECMAScript name.
///
/// Twenty-one: the four format families' format and keystroke functions, the two `Ex` variants of
/// each date and time function, the special formats' arbitrary mask, the one calculation, the one
/// validation, and the four helpers the rest are built over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Function {
    /// `AFNumber_Format(nDec, sepStyle, negStyle, currStyle, strCurrency, bCurrencyPrepend)`.
    NumberFormat,
    /// `AFNumber_Keystroke`, with the same six arguments.
    NumberKeystroke,
    /// `AFPercent_Format(nDec, sepStyle)`, and the optional third argument that puts the sign in
    /// front.
    PercentFormat,
    /// `AFPercent_Keystroke(nDec, sepStyle)`.
    PercentKeystroke,
    /// `AFDate_Format(cFormat)`: one of the fourteen pictures, by index or by text.
    DateFormat,
    /// `AFDate_FormatEx(cFormat)`: any `printd` picture.
    DateFormatEx,
    /// `AFDate_Keystroke(cFormat)`.
    DateKeystroke,
    /// `AFDate_KeystrokeEx(cFormat)`.
    DateKeystrokeEx,
    /// `AFTime_Format(ptf)`: one of the four time formats.
    TimeFormat,
    /// `AFTime_FormatEx(cFormat)`: any `printd` picture.
    TimeFormatEx,
    /// `AFTime_Keystroke(ptf)`.
    TimeKeystroke,
    /// `AFTime_KeystrokeEx(cFormat)`.
    TimeKeystrokeEx,
    /// `AFSpecial_Format(psf)`: zip, zip + 4, phone, SSN.
    SpecialFormat,
    /// `AFSpecial_Keystroke(psf)`.
    SpecialKeystroke,
    /// `AFSpecial_KeystrokeEx(cMask)`: an arbitrary mask.
    SpecialKeystrokeEx,
    /// `AFSimple_Calculate(cFunction, cFields)`.
    SimpleCalculate,
    /// `AFRange_Validate(bGreaterThan, nGreaterThan, bLessThan, nLessThan)`.
    RangeValidate,
    /// `AFMakeNumber(cString)`.
    MakeNumber,
    /// `AFExtractNums(cString)`.
    ExtractNums,
    /// `AFMergeChange(event)`.
    MergeChange,
    /// `AFParseDateEx(cString, cOrder)`.
    ParseDateEx,
}

impl Function {
    /// All twenty-one, in declaration order.
    pub const ALL: [Self; 21] = [
        Self::NumberFormat,
        Self::NumberKeystroke,
        Self::PercentFormat,
        Self::PercentKeystroke,
        Self::DateFormat,
        Self::DateFormatEx,
        Self::DateKeystroke,
        Self::DateKeystrokeEx,
        Self::TimeFormat,
        Self::TimeFormatEx,
        Self::TimeKeystroke,
        Self::TimeKeystrokeEx,
        Self::SpecialFormat,
        Self::SpecialKeystroke,
        Self::SpecialKeystrokeEx,
        Self::SimpleCalculate,
        Self::RangeValidate,
        Self::MakeNumber,
        Self::ExtractNums,
        Self::MergeChange,
        Self::ParseDateEx,
    ];

    /// The function's ECMAScript name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::NumberFormat => "AFNumber_Format",
            Self::NumberKeystroke => "AFNumber_Keystroke",
            Self::PercentFormat => "AFPercent_Format",
            Self::PercentKeystroke => "AFPercent_Keystroke",
            Self::DateFormat => "AFDate_Format",
            Self::DateFormatEx => "AFDate_FormatEx",
            Self::DateKeystroke => "AFDate_Keystroke",
            Self::DateKeystrokeEx => "AFDate_KeystrokeEx",
            Self::TimeFormat => "AFTime_Format",
            Self::TimeFormatEx => "AFTime_FormatEx",
            Self::TimeKeystroke => "AFTime_Keystroke",
            Self::TimeKeystrokeEx => "AFTime_KeystrokeEx",
            Self::SpecialFormat => "AFSpecial_Format",
            Self::SpecialKeystroke => "AFSpecial_Keystroke",
            Self::SpecialKeystrokeEx => "AFSpecial_KeystrokeEx",
            Self::SimpleCalculate => "AFSimple_Calculate",
            Self::RangeValidate => "AFRange_Validate",
            Self::MakeNumber => "AFMakeNumber",
            Self::ExtractNums => "AFExtractNums",
            Self::MergeChange => "AFMergeChange",
            Self::ParseDateEx => "AFParseDateEx",
        }
    }

    /// The function an ECMAScript name names, or `None`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|function| function.name() == name)
    }

    /// The trigger the function serves, or `None` for the four helpers, which change nothing about
    /// the event and serve none.
    #[must_use]
    pub fn trigger(self) -> Option<Trigger> {
        Some(match self {
            Self::NumberFormat
            | Self::PercentFormat
            | Self::DateFormat
            | Self::DateFormatEx
            | Self::TimeFormat
            | Self::TimeFormatEx
            | Self::SpecialFormat => Trigger::Format,
            Self::NumberKeystroke
            | Self::PercentKeystroke
            | Self::DateKeystroke
            | Self::DateKeystrokeEx
            | Self::TimeKeystroke
            | Self::TimeKeystrokeEx
            | Self::SpecialKeystroke
            | Self::SpecialKeystrokeEx => Trigger::Keystroke,
            Self::RangeValidate => Trigger::Validate,
            Self::SimpleCalculate => Trigger::Calculate,
            Self::MakeNumber | Self::ExtractNums | Self::MergeChange | Self::ParseDateEx => {
                return None;
            }
        })
    }
}

/// The moment [`Call::accepted_example`] writes a date or time picture with.
const EXAMPLE_MOMENT: DateTime = DateTime {
    year: 2024,
    month: 1,
    day: 5,
    hour: 14,
    minute: 30,
    second: 15,
};

/// Why a call did not run: one sentence, for the report a person reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(String);

impl Refusal {
    /// A refusal saying `why`.
    pub(crate) fn new(why: impl Into<String>) -> Self {
        Self(why.into())
    }

    /// The sentence.
    #[must_use]
    pub fn sentence(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refusal {}

/// What Table 199's `/K` hands its script: Adobe's `event.value`, `event.change`,
/// `event.selStart`, `event.selEnd` and `event.willCommit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keystroke<'a> {
    /// The field's text before the keystroke.
    pub value: &'a str,
    /// What the keystroke inserts, replacing the selection.
    pub change: &'a str,
    /// The selection the change replaces, as byte offsets into `value`.
    pub selection: (usize, usize),
    /// Whether this is the commit rather than a character.
    pub will_commit: bool,
}

/// `AFMergeChange`: the field's text as a keystroke would leave it.
///
/// At commit the value is already whole; otherwise it is the value with its selection replaced by
/// the change. An offset past the end, or inside a character, is moved to the nearest boundary
/// below it, so a host's stale selection cannot split a character.
#[must_use]
pub fn merge_change(event: &Keystroke<'_>) -> String {
    if event.will_commit {
        return event.value.to_owned();
    }
    let boundary = |at: usize| {
        let mut at = at.min(event.value.len());
        while !event.value.is_char_boundary(at) {
            at = at.saturating_sub(1);
        }
        at
    };
    let from = boundary(event.selection.0.min(event.selection.1));
    let to = boundary(event.selection.0.max(event.selection.1));
    let mut out = String::with_capacity(event.value.len().saturating_add(event.change.len()));
    out.push_str(event.value.get(..from).unwrap_or_default());
    out.push_str(event.change);
    out.push_str(event.value.get(to..).unwrap_or_default());
    out
}

/// What a format script made of a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formatted {
    /// The text to display, in place of the value.
    pub text: String,
    /// Whether the text is to be drawn red: `AFNumber_Format`'s two red negative styles.
    pub red: bool,
}

/// What a keystroke script made of a keystroke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keyed {
    /// The keystroke stands. At commit the value may have been rewritten, which `value` carries.
    Accepted {
        /// The committed text, where the script rewrote it.
        value: Option<String>,
    },
    /// The keystroke is refused: Adobe's `event.rc` set false.
    Rejected {
        /// What a person is told, at commit; a refused character is refused in silence.
        message: Option<String>,
    },
}

/// The arguments of one call, read as ECMAScript would convert them.
struct Arguments<'a>(&'a [Literal]);

impl Arguments<'_> {
    /// Argument `index` as a number: ECMAScript's `ToNumber` over a literal, `None` for an absent
    /// argument or a string that states no number.
    fn number(&self, index: usize) -> Option<f64> {
        match self.0.get(index)? {
            Literal::Number(value) => Some(*value),
            Literal::Boolean(value) => Some(if *value { 1.0 } else { 0.0 }),
            Literal::String(text) => make_number(text),
            Literal::Array(_) => None,
        }
    }

    /// Argument `index` as a boolean: ECMAScript's `ToBoolean`, `false` for an absent argument.
    fn boolean(&self, index: usize) -> bool {
        match self.0.get(index) {
            Some(Literal::Boolean(value)) => *value,
            Some(Literal::Number(value)) => *value != 0.0 && !value.is_nan(),
            Some(Literal::String(text)) => !text.is_empty(),
            Some(Literal::Array(_)) => true,
            None => false,
        }
    }

    /// Argument `index` as a string: ECMAScript's `ToString`, `""` for an absent argument.
    fn string(&self, index: usize) -> String {
        match self.0.get(index) {
            Some(Literal::String(text)) => text.clone(),
            Some(Literal::Number(value)) => number_text(*value),
            Some(Literal::Boolean(value)) => value.to_string(),
            Some(Literal::Array(items)) => items.join(","),
            None => String::new(),
        }
    }

    /// Argument `index` as a list of names: an array literal, or one string of comma-separated
    /// names with white space around each taken off.
    fn names(&self, index: usize) -> Vec<String> {
        let listed: Vec<String> = match self.0.get(index) {
            Some(Literal::Array(items)) => items.clone(),
            Some(Literal::String(text)) => text.split(',').map(str::to_owned).collect(),
            _ => Vec::new(),
        };
        listed
            .into_iter()
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect()
    }
}

/// The decimal places argument of a number or percent function.
fn places(arguments: &Arguments<'_>, function: Function) -> Result<u32, Refusal> {
    let value = arguments.number(0).unwrap_or(0.0);
    number::whole(value)
        .filter(|places| *places <= MAX_PLACES)
        .ok_or_else(|| {
            Refusal::new(format!(
                "{}'s nDec is {}, and a number of decimal places is a whole number from 0 to \
                 {MAX_PLACES}",
                function.name(),
                number_text(value)
            ))
        })
}

/// The separator style argument of a number or percent function.
fn separators(arguments: &Arguments<'_>, function: Function) -> Result<Separators, Refusal> {
    let value = arguments.number(1).unwrap_or(0.0);
    Separators::of(value).ok_or_else(|| {
        Refusal::new(format!(
            "{}'s sepStyle is {}, and the styles are 0 to 4",
            function.name(),
            number_text(value)
        ))
    })
}

/// The picture a date or time function names: an index into its menu, or a picture's text.
fn picture(
    arguments: &Arguments<'_>,
    menu: &[&'static str],
    function: Function,
) -> Result<String, Refusal> {
    match arguments.0.first() {
        Some(Literal::String(text)) => Ok(text.clone()),
        Some(Literal::Number(index)) => number::whole(*index)
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| menu.get(index))
            .map(|picture| (*picture).to_owned())
            .ok_or_else(|| {
                Refusal::new(format!(
                    "{} names picture {} of a menu of {}",
                    function.name(),
                    number_text(*index),
                    menu.len()
                ))
            }),
        _ => Err(Refusal::new(format!(
            "{} names no picture",
            function.name()
        ))),
    }
}

impl Call {
    /// Runs a call at the trigger it was found at, refusing one that serves another.
    fn serving(&self, trigger: Trigger) -> Result<Arguments<'_>, Refusal> {
        match self.function.trigger() {
            Some(served) if served == trigger => Ok(Arguments(&self.arguments)),
            Some(served) => Err(Refusal::new(format!(
                "{} is a {} function and Table 199 raised it as the field's {} script",
                self.function.name(),
                served.noun(),
                trigger.noun()
            ))),
            None => Err(Refusal::new(format!(
                "{} returns a value and changes nothing about the event, so as the whole of a {} \
                 script it does nothing",
                self.function.name(),
                trigger.noun()
            ))),
        }
    }

    /// Table 199's `/F`: what the field displays for `value`.
    ///
    /// An empty value displays empty under every format. A value the format cannot read — a
    /// number format over text that states no number, a date format over text its picture does not
    /// read — displays **as it stands**: the format describes how to show a value of its kind,
    /// and a value of another kind is shown as the person or the producer wrote it rather than
    /// blanked (ADR 1578 section 1).
    ///
    /// # Errors
    ///
    /// A [`Refusal`] where the call is not a format function, or where its arguments name a style,
    /// a picture or a mask this library does not define.
    pub fn format(&self, value: &str) -> Result<Formatted, Refusal> {
        let arguments = self.serving(Trigger::Format)?;
        let plain = |text: String| Formatted { text, red: false };
        if value.is_empty() {
            return Ok(plain(String::new()));
        }
        match self.function {
            Function::NumberFormat => {
                let places = places(&arguments, self.function)?;
                let style = separators(&arguments, self.function)?;
                let negative_value = arguments.number(2).unwrap_or(0.0);
                let negative = Negative::of(negative_value).ok_or_else(|| {
                    Refusal::new(format!(
                        "AFNumber_Format's negStyle is {}, and the styles are 0 to 3",
                        number_text(negative_value)
                    ))
                })?;
                let currency = arguments.string(4);
                let prepend = arguments.boolean(5);
                let Some(number) = make_number(value) else {
                    return Ok(plain(value.to_owned()));
                };
                let (text, red) =
                    number::number_format(number, places, style, negative, &currency, prepend);
                Ok(Formatted { text, red })
            }
            Function::PercentFormat => {
                let places = places(&arguments, self.function)?;
                let style = separators(&arguments, self.function)?;
                let Some(number) = make_number(value) else {
                    return Ok(plain(value.to_owned()));
                };
                let (negative, digits) = number::digits_with(number * 100.0, places, style);
                let signed = if negative {
                    format!("-{digits}")
                } else {
                    digits
                };
                Ok(plain(if arguments.boolean(2) {
                    format!("%{signed}")
                } else {
                    format!("{signed}%")
                }))
            }
            Function::DateFormat | Function::DateFormatEx | Function::TimeFormatEx => {
                let picture = picture(&arguments, &date::DATE_PICTURES, self.function)?;
                match parse_date(value, &picture) {
                    Some(moment) => Ok(plain(print_date(&picture, &moment)?)),
                    None => Ok(plain(value.to_owned())),
                }
            }
            Function::TimeFormat => {
                let picture = picture(&arguments, &date::TIME_PICTURES, self.function)?;
                let Some(moment) = parse_date(value, &picture) else {
                    return Ok(plain(value.to_owned()));
                };
                let menu = date::TIME_PICTURES.iter().position(|item| *item == picture);
                Ok(plain(
                    match menu.and_then(|index| date::print_time_menu(index, moment)) {
                        Some(text) => text,
                        None => print_date(&picture, &moment)?,
                    },
                ))
            }
            Function::SpecialFormat => {
                let psf = arguments.number(0).unwrap_or(-1.0);
                let special = mask::Special::of(psf).ok_or_else(|| {
                    Refusal::new(format!(
                        "AFSpecial_Format's psf is {}, and the formats are 0 to 3",
                        number_text(psf)
                    ))
                })?;
                Ok(plain(print_mask(special.mask(value), value)))
            }
            _ => Err(Refusal::new(format!(
                "{} is not a format function",
                self.function.name()
            ))),
        }
    }

    /// Table 199's `/K`: whether a keystroke stands, and at commit what it commits.
    ///
    /// # Errors
    ///
    /// A [`Refusal`] where the call is not a keystroke function, or names a style or a picture
    /// this library does not define.
    pub fn keystroke(&self, event: &Keystroke<'_>) -> Result<Keyed, Refusal> {
        let arguments = self.serving(Trigger::Keystroke)?;
        let merged = merge_change(event);
        let accepted = Keyed::Accepted { value: None };
        let refused = |message: String| Keyed::Rejected {
            message: event.will_commit.then_some(message),
        };
        match self.function {
            Function::NumberKeystroke | Function::PercentKeystroke => {
                let style = separators(&arguments, self.function)?;
                if !number::number_keystroke(&merged, style.point, event.will_commit) {
                    return Ok(refused(format!(
                        "the value entered does not match the format of the field, a number \
                         written with {} as its decimal point",
                        style.point
                    )));
                }
                // A comma style stores the number the person typed with a period, so that every
                // reader of the value — the format, a calculation, a submission — reads one
                // spelling of a decimal point (ADR 1578 section 3).
                if event.will_commit && style.point == ',' && merged.contains(',') {
                    return Ok(Keyed::Accepted {
                        value: Some(merged.trim().replace(',', ".")),
                    });
                }
                Ok(accepted)
            }
            Function::DateKeystroke
            | Function::DateKeystrokeEx
            | Function::TimeKeystroke
            | Function::TimeKeystrokeEx => {
                let menu: &[&'static str] = match self.function {
                    Function::TimeKeystroke | Function::TimeKeystrokeEx => &date::TIME_PICTURES,
                    _ => &date::DATE_PICTURES,
                };
                let picture = picture(&arguments, menu, self.function)?;
                // A date is judged whole, at commit: there is no picture whose prefix a person
                // typing one character at a time stays inside (ADR 1578 section 6).
                if !event.will_commit || merged.trim().is_empty() {
                    return Ok(accepted);
                }
                if parse_date(&merged, &picture).is_some() {
                    Ok(accepted)
                } else {
                    Ok(refused(format!(
                        "the date or time entered does not exist or does not match the field's \
                         picture {picture}"
                    )))
                }
            }
            Function::SpecialKeystroke => {
                let psf = arguments.number(0).unwrap_or(-1.0);
                let special = mask::Special::of(psf).ok_or_else(|| {
                    Refusal::new(format!(
                        "AFSpecial_Keystroke's psf is {}, and the formats are 0 to 3",
                        number_text(psf)
                    ))
                })?;
                if special.accepts(merged.trim(), event.will_commit) {
                    Ok(accepted)
                } else {
                    Ok(refused(
                        "the value entered does not match the format of the field".to_owned(),
                    ))
                }
            }
            Function::SpecialKeystrokeEx => {
                let mask = arguments.string(0);
                match mask::arbitrary(&mask, &merged, event.will_commit) {
                    Some(committed) if event.will_commit && committed != merged => {
                        Ok(Keyed::Accepted {
                            value: Some(committed),
                        })
                    }
                    Some(_) => Ok(accepted),
                    None => Ok(refused(format!(
                        "the value entered does not match the field's mask {mask}"
                    ))),
                }
            }
            _ => Err(Refusal::new(format!(
                "{} is not a keystroke function",
                self.function.name()
            ))),
        }
    }

    /// A value this keystroke script accepts, both typed and committed, or `None` where it is no
    /// keystroke function or names a style or picture this library does not define.
    ///
    /// What an instrument types into a scripted field so that the edit it witnesses is one the
    /// document's own script lets stand: a number for the number and percent families, the
    /// fifth of January 2024 at 14:30:15 written in the field's own picture for the date and time
    /// families, the digits of a complete value for the special formats, and the arbitrary mask
    /// with each place-holder filled. Every answer is checked against [`Self::keystroke`] before it
    /// is given, so it cannot claim a value the script would refuse (ADR 1579).
    #[must_use]
    pub fn accepted_example(&self) -> Option<String> {
        let arguments = self.serving(Trigger::Keystroke).ok()?;
        let example = match self.function {
            Function::NumberKeystroke | Function::PercentKeystroke => "1234.5".to_owned(),
            Function::DateKeystroke | Function::DateKeystrokeEx => {
                let picture = picture(&arguments, &date::DATE_PICTURES, self.function).ok()?;
                print_date(&picture, &EXAMPLE_MOMENT).ok()?
            }
            Function::TimeKeystroke | Function::TimeKeystrokeEx => {
                let picture = picture(&arguments, &date::TIME_PICTURES, self.function).ok()?;
                print_date(&picture, &EXAMPLE_MOMENT).ok()?
            }
            Function::SpecialKeystroke => {
                let special = mask::Special::of(arguments.number(0).unwrap_or(-1.0))?;
                special.example().to_owned()
            }
            Function::SpecialKeystrokeEx => mask::filled(&arguments.string(0)),
            _ => return None,
        };
        let stands = |will_commit: bool| {
            let event = if will_commit {
                Keystroke {
                    value: &example,
                    change: "",
                    selection: (example.len(), example.len()),
                    will_commit,
                }
            } else {
                Keystroke {
                    value: "",
                    change: &example,
                    selection: (0, 0),
                    will_commit,
                }
            };
            matches!(self.keystroke(&event), Ok(Keyed::Accepted { .. }))
        };
        (stands(false) && stands(true)).then_some(example)
    }

    /// Table 199's `/V`: `None` where the value stands, or what a person is told where it does not.
    ///
    /// An empty value, and a value that states no number, are not this function's to judge — a
    /// range is a statement about numbers — and stand (ADR 1578 section 4).
    ///
    /// # Errors
    ///
    /// A [`Refusal`] where the call is not a validation function.
    pub fn validate(&self, value: &str) -> Result<Option<String>, Refusal> {
        let arguments = self.serving(Trigger::Validate)?;
        let Some(number) = make_number(value) else {
            return Ok(None);
        };
        let low = arguments.boolean(0).then(|| arguments.number(1)).flatten();
        let high = arguments.boolean(2).then(|| arguments.number(3)).flatten();
        Ok(number::range_refusal(number, low, high))
    }

    /// Table 199's `/C`: the value the field is recalculated to.
    ///
    /// `values` answers, for one field name the call lists, the values of every terminal field it
    /// names — itself where it is terminal, its descendants where it is not — which is what
    /// Adobe's `getField(name).getArray()` hands `AFSimple_Calculate`. A name that names no field
    /// contributes nothing.
    ///
    /// # Errors
    ///
    /// A [`Refusal`] where the call is not a calculation, or names a function its menu does not.
    pub fn calculate(
        &self,
        values: &mut dyn FnMut(&str) -> Vec<String>,
    ) -> Result<String, Refusal> {
        let arguments = self.serving(Trigger::Calculate)?;
        let name = arguments.string(0);
        let arithmetic = Arithmetic::of(&name).ok_or_else(|| {
            Refusal::new(format!(
                "AFSimple_Calculate's cFunction is {name:?}, and the functions are AVG, SUM, PRD, \
                 MIN and MAX"
            ))
        })?;
        let mut numbers = Vec::new();
        for field in arguments.names(1) {
            for text in values(&field) {
                numbers.push(make_number(&text).unwrap_or(0.0));
            }
        }
        Ok(arithmetic.over(&numbers))
    }
}
