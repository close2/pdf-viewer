//! XFA 3.3's null test, run where a form asks for its validations before a save (ADR 2450).
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

use super::config::{Config, Messaging};
use super::datasets::{Change, item};
use super::template::{Class, Container, NullTest, Template};

/// The messages of a save's null tests: one `Vec` per message the person is shown, each holding
/// one sentence per field it reports — page 386's `allMessagesTogether` puts every field in one,
/// `firstMessageOnly` keeps the first, and `noMessages` shows none. Empty where the form does not
/// ask for its validations before a save, or every field passes.
///
/// `fields` are the PDF field names of the form's fields, each with the value the save leaves in
/// its `/V`, in the terms its data value needs.
pub(crate) fn before_save(
    template: &Template,
    config: &Config,
    fields: &[(String, Change)],
) -> Vec<Vec<String>> {
    if !config.validates_on_save() {
        return Vec::new();
    }
    let failed: Vec<String> = fields
        .iter()
        .filter_map(|(name, value)| {
            let container = template.get(template.resolve(name)?)?;
            failure(name, container, value)
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

/// The sentence for `name` where its container's null test fails on `value`.
fn failure(name: &str, container: &Container, value: &Change) -> Option<String> {
    if !matches!(container.class, Class::Field | Class::ExclGroup) {
        return None;
    }
    let pictures = &container.pictures;
    let (level, wants) = match pictures.null_test {
        NullTest::Disabled => return None,
        NullTest::Warning => ("warning", "recommends"),
        NullTest::Error => ("error", "requires"),
    };
    if !is_null(container, value) {
        return None;
    }
    let said = pictures
        .message("nullTest")
        .map(|said| format!(", and its message for it is {said:?}"))
        .unwrap_or_default();
    Some(format!(
        "{name} is saved empty, and its XFA form {wants} a value there (its validate element's \
         null test at the level {level}, XFA 3.3 page 384){said}; the form asks for its \
         validations before a save (page 1340), which saves the form whether they succeed or not"
    ))
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
    use super::before_save;
    use crate::xfa::config::Config;
    use crate::xfa::datasets::Change;
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
        let said = before_save(
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
        assert!(before_save(&template(), &config("preSubmit", ""), &fields()).is_empty());
        let unstated = Config::read(&parse(b"<config/>").expect("well formed"));
        assert!(before_save(&template(), &unstated, &fields()).is_empty());
    }

    #[test]
    fn validation_messaging_shapes_the_messages() {
        let shaped = |messaging| before_save(&template(), &config("preSave", messaging), &fields());
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
        assert!(before_save(&template(), &config("preSave", ""), &filled).is_empty());
    }
}
