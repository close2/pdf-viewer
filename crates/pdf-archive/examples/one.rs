//! Holds one document to one target and prints the report.
//!
//! ```sh
//! cargo run -p pdf-archive --example one -- <file.pdf> 4
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a report"
)]

use pdf_archive::{Target, check};
use pdf_syntax::{Document, FileBytes};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        println!("usage: one <file.pdf> <2b|2u|2a|4|4f|4e>");
        return;
    };
    let target = arguments
        .next()
        .and_then(|name| Target::parse(&name))
        .unwrap_or(Target::Four(pdf_archive::Flavour::Plain));
    let Ok(bytes) = FileBytes::on_disk(std::path::Path::new(&path)) else {
        println!("{path}: cannot be read");
        return;
    };
    match Document::open(bytes) {
        Ok(document) => print!("{}", check(&document, target).render()),
        Err(error) => println!("{path}: does not open ({error})"),
    }
}
