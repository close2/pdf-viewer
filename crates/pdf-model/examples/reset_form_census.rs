//! Which of Table 241's two spellings of `/Fields` the corpus's reset-form actions use.
//!
//! ISO 32000-2 §12.7.6.3's Table 241 lets one array name a field two ways — "an indirect
//! reference to a field dictionary or (PDF 1.3) a text string representing the fully qualified
//! name of a field" — and Table 242 adds that the array reaches "[a]ll descendants of the
//! specified fields in the field hierarchy". The two spellings therefore ask the same question of
//! different machinery: a name is a prefix test over §12.7.4.2's qualified names, a reference is a
//! walk down that field's own `/Kids`. This counts which spelling documents actually write, and
//! how many of the referenced fields are *not* leaves — which is the population where the
//! difference between the two is visible at all.
//!
//! ```sh
//! cargo run --release -p pdf-model --example reset_form_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_model::action::{Action, ResetTarget};
use pdf_syntax::{Document, ObjectId};

fn main() {
    let mut documents = 0_usize;
    let mut actions = 0_usize;
    let mut without_fields = 0_usize;
    let mut by_name = 0_usize;
    let mut by_reference = 0_usize;
    let mut references_with_kids = 0_usize;
    let mut files: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let mut here = 0_usize;
        // Every object in the file, because an action is reachable from an annotation, an
        // outline item, a field's `/AA` and the catalogue's `/OpenAction` alike, and a census
        // that walked only one of those would be a census of that one.
        for number in document.xref().object_numbers() {
            let id = ObjectId {
                number,
                generation: 0,
            };
            let object = document.get(id);
            let Some(dict) = object.as_dict() else {
                continue;
            };
            if document
                .get_key(dict, "S")
                .as_name()
                .map(|name| name.as_bytes().to_vec())
                != Some(b"ResetForm".to_vec())
            {
                continue;
            }
            let read = pdf_model::action::read(&document, &pdf_syntax::Object::Reference(id));
            let Some(Action::ResetForm(reset)) = read.first() else {
                continue;
            };
            actions = actions.saturating_add(1);
            here = here.saturating_add(1);
            if reset.fields.is_empty() {
                without_fields = without_fields.saturating_add(1);
            }
            for target in &reset.fields {
                match target {
                    ResetTarget::Name(_) => by_name = by_name.saturating_add(1),
                    ResetTarget::Field(field) => {
                        by_reference = by_reference.saturating_add(1);
                        let referenced = document.get(*field);
                        let has_kids = referenced.as_dict().is_some_and(|dict| {
                            document
                                .get_key(dict, "Kids")
                                .as_array()
                                .is_some_and(|kids| !kids.is_empty())
                        });
                        if has_kids {
                            references_with_kids = references_with_kids.saturating_add(1);
                        }
                    }
                }
            }
        }
        if here > 0 {
            files.push(format!("{path} ({here})"));
        }
    }

    println!("{documents} document(s) read");
    println!(
        "{actions} reset-form action(s) in {} document(s)",
        files.len()
    );
    println!("  {without_fields} state no /Fields at all — every field in the form is reset");
    println!("  {by_name} /Fields element(s) are a fully qualified name");
    println!(
        "  {by_reference} are an indirect reference, of which {references_with_kids} name a \
         field that states /Kids"
    );
    for file in &files {
        println!("    {file}");
    }
}
