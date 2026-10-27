//! XFA 3.3's validations, run where a form asks for them before a save or a submit (ADRs 2450,
//! 2474, 2475).
//!
//! Chapter 10 names the tests a container is put through (page 381's table): the null-content
//! test, the datatype test, and the format and script tests of the `validate` element. This
//! module runs the first two where a save asks for them, and the first, the second and the format
//! test where a submit does (page 447's standard submit processing runs the null test and then the
//! format test over the whole form, whatever the submit carries). The script test is a script's,
//! which waits on a `FormCalc` engine (RFC 0011 section 13).
//!
//! Chapter 10's `validate` element tests a field or an exclusion group for null content first
//! (page 381's table, which marks the test not applicable to a subform). Page 396 runs the null
//! test only at an explicit `execValidate()` or automatically during a pre-event the
//! implementation determines, so that a partly filled form is not complained about; page 506 names
//! saving a form to a file among the commits it is run at; and page 1340's `validate` option is
//! the form's own word on which pre-events those are, `preSubmit` alone by default. So a save runs
//! the null test over every field the form holds where the form's `config` lists `preSave`, and
//! not otherwise.
//!
//! **A failing null test is a report and not a refusal.** Page 1340 says that a processor saving
//! a form to a file saves it whether validation succeeds or fails, the messages still being of
//! use; page 384 lets a processor refuse a commit while the form is invalid and does not require
//! it. A save is the reader's, and the empty value is what the person left in the field, so the
//! data takes it as the `/V` does, which keeps §K.2's consistency, and the person is told which
//! fields the form requires (`error`) or recommends (`warning`) a value in, in the form's own
//! message where it gives one (page 383), presented as page 386's `validationMessaging` says.
//!
//! **Page 381 calls the test not applicable where the template gives a default value, and page
//! 396 is the case that reads it**: a field holding its default is valid, the person clears it,
//! and the null test fails at the next pre-event. So the test is of the value the save leaves in
//! the field, and a default is why a field the person never touched passes it, not an exemption
//! for a field the person emptied. XFA 3.3 is cited and paraphrased, never quoted (RFC 0011's
//! preamble).

use super::canonical;
use super::config::{Config, Messaging};
use super::datasets::{Change, Formatted, canonical_text, format_test, item};
use super::locale::LocaleSet;
use super::template::{Class, Container, Digits, NullTest, Template};

/// The moment a form's validations are run at: page 1340's `preSave` and `preSubmit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occasion {
    /// A form saved to a file (page 1340, page 506).
    Save,
    /// A form submitted to a server (page 1340, page 447).
    Submit,
}

impl Occasion {
    /// What the sentence says of the moment and of page 1340's consequence of a failure there.
    ///
    /// A save saves whether the validations succeed or fail. A submit is cancelled by an error and
    /// goes ahead on a warning, and this program composes the request whatever was found:
    /// cancelling is the reader's to decide in the place a host supplies (`CLAUDE.md` principle 3,
    /// ADR 1198), and the sentence is what that decision is told.
    fn clause(self) -> &'static str {
        match self {
            Self::Save => {
                "the form asks for its validations before a save (page 1340), which saves the \
                 form whether they succeed or not"
            }
            Self::Submit => {
                "the form asks for its validations before a submit (page 1340), which an error \
                 cancels and a warning lets go ahead, and whether this submit is cancelled is its \
                 host\'s submit policy\'s to say"
            }
        }
    }

    /// The word for what became of a field the null test finds empty.
    fn verb(self) -> &'static str {
        match self {
            Self::Save => "saved",
            Self::Submit => "submitted",
        }
    }
}

/// The messages of a save's or a submit's validations: one `Vec` per message the person is shown,
/// each holding one sentence per failure — page 386's `allMessagesTogether` puts every failure in
/// one, `firstMessageOnly` keeps the first, and `noMessages` shows none. Empty where the form does
/// not ask for its validations at `occasion`, or every field passes.
///
/// `fields` are the PDF field names of the form's fields, each with the value the save leaves in,
/// or the submit carries from, its `/V`, in the terms its data value needs.
pub(crate) fn before(
    occasion: Occasion,
    (template, locales, config): (&Template, &LocaleSet, &Config),
    fields: &[(String, Change)],
) -> Vec<Vec<String>> {
    let asked = match occasion {
        Occasion::Save => config.validates_on_save(),
        Occasion::Submit => config.validates_on_submit(),
    };
    if !asked {
        return Vec::new();
    }
    let failed: Vec<String> = fields
        .iter()
        .flat_map(|(name, value)| {
            let Some(target) = template.resolve(name) else {
                return Vec::new();
            };
            let Some(container) = template.get(target) else {
                return Vec::new();
            };
            let tested = Tested {
                occasion,
                template,
                locales,
                target,
                container,
            };
            tested.failures(name, value)
        })
        .collect();
    match config.messaging() {
        Messaging::Individually => failed.into_iter().map(|said| vec![said]).collect(),
        Messaging::Together if failed.is_empty() => Vec::new(),
        Messaging::Together => vec![failed],
        Messaging::FirstOnly => failed.into_iter().take(1).map(|said| vec![said]).collect(),
        Messaging::None => Vec::new(),
    }
}

/// One container and the moment it is validated at.
struct Tested<'a> {
    occasion: Occasion,
    template: &'a Template,
    locales: &'a LocaleSet,
    target: usize,
    container: &'a Container,
}

impl Tested<'_> {
    /// The sentences for `name` holding `value`, in page 381's order of tests.
    fn failures(&self, name: &str, value: &Change) -> Vec<String> {
        if !matches!(self.container.class, Class::Field | Class::ExclGroup) {
            return Vec::new();
        }
        [
            self.null_failure(name, value),
            self.datatype_failure(name, value),
            self.format_failure(name, value),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// The sentence for `name` where its container's null test fails on `value`.
    fn null_failure(&self, name: &str, value: &Change) -> Option<String> {
        let pictures = &self.container.pictures;
        let (level, wants) = match pictures.null_test {
            NullTest::Disabled => return None,
            NullTest::Warning => ("warning", "recommends"),
            NullTest::Error => ("error", "requires"),
        };
        if !is_null(self.container, value) {
            return None;
        }
        let said = pictures
            .message("nullTest")
            .map(|said| format!(", and its message for it is {said:?}"))
            .unwrap_or_default();
        Some(format!(
            "{name} is {} empty, and its XFA form {wants} a value there (its validate element's \
             null test at the level {level}, XFA 3.3 page 384){said}; {}",
            self.occasion.verb(),
            self.occasion.clause()
        ))
    }

    /// The sentence for `name` where its field's value is not of the type its `value` element
    /// holds (page 386's datatype test).
    ///
    /// **The test is of the unformatted value** (page 382), so a text a person typed into an edit
    /// widget is first read as chapter 4 reads it. A text no number can be read from in a numeric
    /// field is a value the type cannot hold; a picture this reader cannot read leaves the test
    /// unrun, which is said and not passed. An empty value has no content to be of a type, and
    /// page 396 leaves it to the null test.
    fn datatype_failure(&self, name: &str, value: &Change) -> Option<String> {
        let container = self.container;
        if container.class != Class::Field {
            return None;
        }
        let kind = container
            .value_type
            .as_deref()
            .filter(|kind| restrictive(kind))?;
        let tested: Vec<String> = match value {
            Change::Text(text) if text.is_empty() => Vec::new(),
            Change::Text(text) => match self.canonical(text) {
                Ok(canonical) => vec![canonical],
                Err(cannot) if cannot.not_a_number => vec![text.clone()],
                Err(cannot) => {
                    return Some(format!(
                        "{name}'s datatype test (XFA 3.3 page 386) cannot be run: {}; {}",
                        cannot.reason,
                        self.occasion.clause()
                    ));
                }
            },
            other => data_values(container, other),
        };
        let (shown, why) = tested.iter().find_map(|shown| {
            inconsistency(kind, container.digits, shown).map(|why| (shown, why))
        })?;
        Some(format!(
            "{name} holds {shown:?}, which {why}, so its datatype test (XFA 3.3 page 386) fails and \
             the field is invalid; {}",
            self.occasion.clause()
        ))
    }

    /// The sentence for `name` where its `validate` picture cannot format its value (page 382's
    /// format test), at a submit only: a save reports that test as a change it did not keep
    /// (ADR 2434).
    fn format_failure(&self, name: &str, value: &Change) -> Option<String> {
        let container = self.container;
        let Change::Text(text) = value else {
            return None;
        };
        if self.occasion != Occasion::Submit
            || container.class != Class::Field
            || text.is_empty()
            || container.pictures.validate.is_none()
        {
            return None;
        }
        let canonical = match self.canonical_value(text) {
            Ok(canonical) => canonical,
            Err(cannot) => {
                let datatype_says = container.value_type.as_deref().is_some_and(restrictive);
                return (!datatype_says && !cannot.not_a_number).then(|| {
                    format!(
                        "{name}'s format test (XFA 3.3 page 382) cannot be run: {}; {}",
                        cannot.reason,
                        self.occasion.clause()
                    )
                });
            }
        };
        match format_test(container, self.locales, &canonical) {
            Formatted::Untested | Formatted::Passes => None,
            Formatted::Fails { level } => {
                let said = container
                    .pictures
                    .message("formatTest")
                    .map(|said| format!(", and its message for it is {said:?}"))
                    .unwrap_or_default();
                Some(format!(
                    "{name} holds {:?}, which its validate picture cannot format (its format test \
                     at the level {level}, XFA 3.3 page 382){said}; {}",
                    canonical.value,
                    self.occasion.clause()
                ))
            }
            Formatted::Unrun(reason) => Some(format!(
                "{name}'s format test (XFA 3.3 page 382) cannot be run: {reason}; {}",
                self.occasion.clause()
            )),
        }
    }

    /// The canonical value of a text typed into the field, `text` itself where its widget is not
    /// one chapter 4 reads input for.
    fn canonical(&self, text: &str) -> Result<String, super::datasets::Cannot> {
        match self.container.ui.as_ref().map(|(ui, _)| ui.as_str()) {
            Some("textEdit" | "passwordEdit" | "numericEdit" | "dateTimeEdit") => {
                self.canonical_value(text).map(|canonical| canonical.value)
            }
            _ => Ok(text.to_owned()),
        }
    }

    fn canonical_value(
        &self,
        text: &str,
    ) -> Result<super::datasets::Canonical, super::datasets::Cannot> {
        let ui = self.container.ui.as_ref().map_or("", |(ui, _)| ui.as_str());
        canonical_text(
            self.template,
            self.locales,
            self.target,
            self.container,
            ui,
            text,
        )
    }
}

/// Whether the value type restricts what a field can hold: the datatype property page 386
/// calls restrictive, an integer or a float for example. The `text` type and the content types that are not
/// scalar values (`image`, `exData`) hold anything their element can enclose.
fn restrictive(kind: &str) -> bool {
    matches!(
        kind,
        "integer" | "decimal" | "float" | "boolean" | "date" | "time" | "dateTime"
    )
}

/// What a non-text value comes to as data: the items a check button writes (page 198), the
/// selected values of a choice, the export value of another button.
fn data_values(container: &Container, value: &Change) -> Vec<String> {
    let check_button = container.class == Class::Field
        && container
            .ui
            .as_ref()
            .is_some_and(|(ui, _)| ui == "checkButton");
    let values = match value {
        Change::On(_) if check_button => vec![item(container, 0)],
        Change::Off if check_button => vec![item(container, 1)],
        Change::On(export) => vec![export.clone()],
        Change::Choices(values) => values.clone(),
        Change::Text(text) => vec![text.clone()],
        Change::Off | Change::Cleared => Vec::new(),
    };
    values.into_iter().filter(|v| !v.is_empty()).collect()
}

/// Why `text` is not content the `kind` element can enclose, `None` where it is: the content
/// clause under each element's own heading.
///
/// - `integer` (page 755): an optional leading minus sign and decimal digits.
/// - `decimal` (page 671) and `float` (page 739): an optional leading minus sign and decimal
///   digits separated by a single period. The grammar is silent on a point with digits on one
///   side only, and a leading or trailing point is read as the number it plainly writes. A
///   `decimal` also errors where it holds more fraction digits than `fracDigits` or more leading
///   digits than `leadDigits` (pages 671–672); a `float` states neither.
/// - `boolean` (page 627): `0` or `1`.
/// - `date` (page 666), `time` (page 868) and `dateTime` (page 668): the canonical formats of
///   chapter 24. A date or time element given a full date-time keeps the part it holds and
///   truncates the rest (pages 666 and 868), which is no error.
fn inconsistency(kind: &str, digits: Digits, text: &str) -> Option<String> {
    let number = |text: &str| -> Option<(usize, usize)> {
        let unsigned = text.strip_prefix('-').unwrap_or(text);
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        let digits_only = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
        (digits_only(whole) && digits_only(fraction) && !(whole.is_empty() && fraction.is_empty()))
            .then_some((whole.len(), fraction.len()))
    };
    let fails = |reason: &str| Some(reason.to_owned());
    match kind {
        "integer" => {
            let unsigned = text.strip_prefix('-').unwrap_or(text);
            let integer = !unsigned.is_empty() && unsigned.bytes().all(|b| b.is_ascii_digit());
            if integer {
                None
            } else {
                fails("is no integer (an optional minus sign and decimal digits, page 755)")
            }
        }
        "decimal" | "float" => {
            let Some((lead, fraction)) = number(text) else {
                return fails(
                    "is no number (an optional minus sign and decimal digits around a single \
                     period, pages 671 and 739)",
                );
            };
            if kind == "float" {
                return None;
            }
            if digits.fraction.is_some_and(|most| fraction > most) {
                return Some(format!(
                    "has {fraction} digits after the point where the decimal states at most {} \
                     (fracDigits, page 671)",
                    digits.fraction.unwrap_or_default()
                ));
            }
            digits.lead.filter(|&most| lead > most).map(|most| {
                format!(
                    "has {lead} digits before the point where the decimal states at most {most} \
                     (leadDigits, page 672)"
                )
            })
        }
        "boolean" => (text != "0" && text != "1")
            .then(|| "is neither 0 nor 1, the two values a boolean has (page 627)".to_owned()),
        "date" => (!canonical::is_date(text) && !canonical::is_date_time(text))
            .then(|| "is no date in the canonical format (page 666)".to_owned()),
        "time" => (!canonical::is_time(text) && !canonical::is_date_time(text))
            .then(|| "is no time in the canonical format (page 868)".to_owned()),
        "dateTime" => (!canonical::is_date_time(text))
            .then(|| "is no date and time in the canonical format (page 668)".to_owned()),
        _ => None,
    }
}

/// Whether `value` leaves `container` with null content: no value, an empty one, no choice
/// selected, an exclusion group with no button on, or a check button whose state's `items` value
/// is empty (page 198's on and off values; a check button that is off holds its off value, which
/// is content).
fn is_null(container: &Container, value: &Change) -> bool {
    let check_button = container.class == Class::Field
        && container
            .ui
            .as_ref()
            .is_some_and(|(ui, _)| ui == "checkButton");
    match value {
        Change::On(_) if check_button => item(container, 0).is_empty(),
        Change::Off if check_button => item(container, 1).is_empty(),
        Change::Cleared | Change::Off => true,
        Change::Text(text) => text.is_empty(),
        Change::Choices(values) => values.iter().all(String::is_empty),
        Change::On(export) => export.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Occasion, before};
    use crate::xfa::config::Config;
    use crate::xfa::datasets::Change;
    use crate::xfa::locale::LocaleSet;
    use crate::xfa::template::Template;
    use crate::xfa::xml::parse;

    const TEMPLATE: &str = "<template><subform name=\"form1\">\
        <field name=\"LoanAmount\"><ui><numericEdit/></ui><value><decimal/></value>\
        <validate nullTest=\"error\"><message><text name=\"nullTest\">Enter the amount of the \
        loan in dollars.</text></message></validate></field>\
        <field name=\"Note\"><ui><textEdit/></ui><validate nullTest=\"warning\"/></field>\
        <field name=\"Free\"><ui><textEdit/></ui><validate/></field>\
        <field name=\"Agree\"><ui><checkButton/></ui><items><integer>1</integer>\
        <integer>0</integer></items><validate nullTest=\"error\"/></field>\
        <exclGroup name=\"Choice\"><validate nullTest=\"error\"/>\
        <field name=\"a\"><ui><checkButton/></ui><items><text>a</text></items></field>\
        </exclGroup></subform></template>";

    fn config(validate: &str, messaging: &str) -> Config {
        Config::read(
            &parse(
                format!(
                    "<config><acrobat><common><validationMessaging>{messaging}\
                     </validationMessaging></common><validate>{validate}</validate></acrobat>\
                     </config>"
                )
                .as_bytes(),
            )
            .expect("well formed"),
        )
    }

    /// The default locale\'s patterns and symbols, which the default pictures of a date, a time
    /// and a number are read from (pages 164 and 166).
    const LOCALES: &str = "<localeSet><locale name=\"en_US\">\
        <datePatterns><datePattern name=\"med\">YYYY-MM-DD</datePattern></datePatterns>\
        <timePatterns><timePattern name=\"med\">HH:MM:SS</timePattern></timePatterns>\
        <numberPatterns><numberPattern name=\"numeric\">z,zz9.zzz</numberPattern>\
        </numberPatterns><numberSymbols><numberSymbol name=\"decimal\">.</numberSymbol>\
        <numberSymbol name=\"grouping\">,</numberSymbol><numberSymbol name=\"percent\">%\
        </numberSymbol><numberSymbol name=\"minus\">-</numberSymbol>\
        <numberSymbol name=\"zero\">0</numberSymbol></numberSymbols></locale></localeSet>";

    fn locales() -> LocaleSet {
        let tree = parse(LOCALES.as_bytes()).expect("well formed");
        LocaleSet::read(tree.first("localeSet").map(|at| (&tree, at)), None)
    }

    fn save(template: &Template, config: &Config, fields: &[(String, Change)]) -> Vec<Vec<String>> {
        before(Occasion::Save, (template, &locales(), config), fields)
    }

    fn submit(
        template: &Template,
        config: &Config,
        fields: &[(String, Change)],
    ) -> Vec<Vec<String>> {
        before(Occasion::Submit, (template, &locales(), config), fields)
    }

    fn template() -> Template {
        Template::read(&parse(TEMPLATE.as_bytes()).expect("well formed"), 0)
    }

    fn fields() -> Vec<(String, Change)> {
        vec![
            ("form1[0].LoanAmount[0]".to_owned(), Change::Cleared),
            ("form1[0].Note[0]".to_owned(), Change::Text(String::new())),
            ("form1[0].Free[0]".to_owned(), Change::Cleared),
            ("form1[0].Agree[0]".to_owned(), Change::Off),
            ("form1[0].Choice[0]".to_owned(), Change::Off),
        ]
    }

    #[test]
    fn a_save_the_form_validates_before_names_each_empty_field_its_null_test_holds() {
        // XFA 3.3 EXAMPLE 10.5's field and message, a warning, a disabled test (page 881's
        // default), a check button that is off (its off value is content) and an exclusion group
        // with no button on.
        let said = save(
            &template(),
            &config("preSubmit preSave", "allMessagesIndividually"),
            &fields(),
        );
        assert_eq!(said.len(), 3, "{said:?}");
        let said: Vec<&str> = said.iter().flatten().map(String::as_str).collect();
        assert!(
            said[0].starts_with("form1[0].LoanAmount[0] is saved empty, and its XFA form requires")
        );
        assert!(said[0].contains("\"Enter the amount of the loan in dollars.\""));
        assert!(
            said[1].starts_with("form1[0].Note[0] is saved empty, and its XFA form recommends")
        );
        assert!(said[2].starts_with("form1[0].Choice[0] is saved empty"));
    }

    #[test]
    fn a_form_that_validates_before_submitting_alone_is_not_tested_at_a_save() {
        // Page 1340's default, and what RFC 0011 section 4's four hybrids state.
        assert!(save(&template(), &config("preSubmit", ""), &fields()).is_empty());
        let unstated = Config::read(&parse(b"<config/>").expect("well formed"));
        assert!(save(&template(), &unstated, &fields()).is_empty());
    }

    #[test]
    fn validation_messaging_shapes_the_messages() {
        let shaped = |messaging| save(&template(), &config("preSave", messaging), &fields());
        assert_eq!(shaped("allMessagesTogether").len(), 1);
        assert_eq!(shaped("allMessagesTogether")[0].len(), 3);
        assert_eq!(
            shaped("firstMessageOnly"),
            vec![shaped("allMessagesIndividually")[0].clone()]
        );
        assert!(shaped("noMessages").is_empty());
    }

    #[test]
    fn a_value_passes() {
        let filled = vec![
            (
                "form1[0].LoanAmount[0]".to_owned(),
                Change::Text("12".to_owned()),
            ),
            ("form1[0].Agree[0]".to_owned(), Change::On("1".to_owned())),
            ("form1[0].Choice[0]".to_owned(), Change::On("a".to_owned())),
        ];
        assert!(save(&template(), &config("preSave", ""), &filled).is_empty());
    }

    const TYPED: &str = "<template><subform name=\"form1\">\
        <field name=\"Count\"><ui><numericEdit/></ui><value><integer/></value></field>\
        <field name=\"Price\"><ui><numericEdit/></ui><value><decimal/></value></field>\
        <field name=\"Rate\"><ui><numericEdit/></ui>\
        <value><decimal fracDigits=\"-1\" leadDigits=\"2\"/></value></field>\
        <field name=\"Ratio\"><ui><numericEdit/></ui><value><float/></value></field>\
        <field name=\"Born\"><ui><dateTimeEdit/></ui><value><date/></value></field>\
        <field name=\"At\"><ui><dateTimeEdit/></ui><value><time/></value></field>\
        <field name=\"Stamp\"><ui><dateTimeEdit/></ui><value><dateTime/></value></field>\
        <field name=\"Free\"><ui><textEdit/></ui><value><text/></value></field>\
        <field name=\"Both\"><ui><numericEdit/></ui><value><integer/></value>\
        <validate formatTest=\"error\"><picture>99</picture><message>\
        <text name=\"formatTest\">Two digits.</text></message></validate></field>\
        </subform></template>";

    fn typed(values: &[(&str, &str)]) -> Vec<(String, Change)> {
        values
            .iter()
            .map(|(name, text)| {
                (
                    format!("form1[0].{name}[0]"),
                    Change::Text((*text).to_owned()),
                )
            })
            .collect()
    }

    fn typed_template() -> Template {
        Template::read(&parse(TYPED.as_bytes()).expect("well formed"), 0)
    }

    #[test]
    fn a_value_of_another_type_fails_the_datatype_test() {
        // The content clauses under each value element's heading: integer-data (page 755), the
        // decimal's digit limits (pages 671-672), the boolean-free scalar types, and a text field
        // restricting nothing.
        let said = save(
            &typed_template(),
            &config("preSave", "allMessagesIndividually"),
            &typed(&[
                ("Count", "1.5"),
                ("Price", "1.234"),
                ("Rate", "123"),
                ("Ratio", "1.234567"),
                ("Born", "yesterday"),
                ("At", "noon"),
                ("Stamp", "2001-03-26"),
                ("Free", "anything at all"),
            ]),
        );
        let said: Vec<&str> = said.iter().flatten().map(String::as_str).collect();
        let names: Vec<&str> = said
            .iter()
            .map(|said| {
                let name = said.split_whitespace().next().unwrap_or_default();
                name.trim_end_matches("\'s")
            })
            .collect();
        assert_eq!(
            names,
            [
                "form1[0].Count[0]",
                "form1[0].Price[0]",
                "form1[0].Rate[0]",
                "form1[0].Born[0]",
                "form1[0].At[0]",
                "form1[0].Stamp[0]"
            ],
            "{said:#?}"
        );
        assert!(said[0].contains("is no integer") && said[0].contains("page 386"));
        assert!(said[1].contains("3 digits after the point") && said[1].contains("at most 2"));
        assert!(said[2].contains("3 digits before the point") && said[2].contains("at most 2"));
        assert!(said[3].contains("is no date"));
        // A date-time typed with no edit picture is read by a default two pages disagree on
        // (page 165, page 1156), so the test is said to be unrun and not passed.
        assert!(said[5].contains("cannot be run"), "{}", said[5]);
        assert!(said[0].ends_with("whether they succeed or not"));
    }

    #[test]
    fn a_value_of_the_type_passes_and_an_empty_one_is_the_null_tests() {
        // Page 396: an empty value has no content to be of a type. A date element given a
        // date-time keeps the date (page 666).
        let passes = typed(&[
            ("Count", "-12"),
            ("Price", "12.5"),
            ("Rate", "99.123456"),
            ("Ratio", ".5"),
            ("Born", "2001-03-26"),
            ("At", "06:30:00"),
            ("Stamp", "2001-03-26T06:30:00"),
        ]);
        assert!(save(&typed_template(), &config("preSave", ""), &passes).is_empty());
        let empty = typed(&[("Count", ""), ("Born", "")]);
        assert!(save(&typed_template(), &config("preSave", ""), &empty).is_empty());
        let dated = typed(&[("Born", "2001-03-26T06:30:00")]);
        assert!(save(&typed_template(), &config("preSave", ""), &dated).is_empty());
    }

    #[test]
    fn a_number_a_numeric_field_cannot_read_fails_the_datatype_test() {
        // Page 156 evaluates text that is no number as zero in a numeric field, which the type
        // cannot hold as typed.
        let said = save(
            &typed_template(),
            &config("preSave", ""),
            &typed(&[("Price", "abc")]),
        );
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0][0].contains("\"abc\"") && said[0][0].contains("is no number"));
    }

    #[test]
    fn a_submit_runs_the_validations_unless_the_config_says_otherwise() {
        // Page 1340: preSubmit is the default, and an empty element prevents it.
        let fields = typed(&[("Count", "1.5")]);
        let unstated = Config::read(&parse(b"<config/>").expect("well formed"));
        let said = submit(&typed_template(), &unstated, &fields);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0][0].contains("before a submit (page 1340)"));
        assert!(said[0][0].contains("submit policy"));
        assert!(submit(&typed_template(), &config("", ""), &fields).is_empty());
        assert!(submit(&typed_template(), &config("preSave", ""), &fields).is_empty());
        assert_eq!(
            submit(&typed_template(), &config("preSubmit", ""), &fields).len(),
            1
        );
    }

    #[test]
    fn a_submit_reports_the_null_test_in_its_own_words_and_a_save_keeps_its_own() {
        let said = submit(&template(), &config("preSubmit", ""), &fields());
        assert_eq!(said.len(), 3, "{said:?}");
        assert!(said[0][0].starts_with("form1[0].LoanAmount[0] is submitted empty"));
        assert!(said[0][0].contains("\"Enter the amount of the loan in dollars.\""));
    }

    #[test]
    fn a_submit_runs_the_format_test_and_a_save_leaves_it_to_the_write() {
        // Page 447 step 5: formatTest over the whole form. A picture the value fails, at the level
        // the form states and in its words (page 383).
        let fields = typed(&[("Both", "123")]);
        let said = submit(&typed_template(), &config("preSubmit", ""), &fields);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0][0].contains("cannot format") && said[0][0].contains("level error"));
        assert!(said[0][0].contains("\"Two digits.\""));
        assert!(save(&typed_template(), &config("preSave", ""), &fields).is_empty());
    }
}
