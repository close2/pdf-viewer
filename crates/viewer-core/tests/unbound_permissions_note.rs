//! ISO 32000-2 §7.6.4.3.3 step (f), under the owner's answer A953: a revision 6 file whose
//! `/Perms` block does not verify opens under `/P` as written, and the document report says that
//! its permissions are not bound to its encryption key (ADR 3309).
#![expect(clippy::expect_used, reason = "test fixture construction")]

use std::fmt::Write as _;

use pdf_syntax::serialize::{
    Access, Assembly, Entropy, Form, Options, Protection, serialize_encrypted,
};
use pdf_syntax::{Document, Limits, ObjectId, Version};
use viewer_core::{Command, DocumentId, Event, Viewer};

/// A counting randomness source, so that the written file is a function of its plan.
struct Counter(u8);

impl Entropy for Counter {
    fn fill(&mut self, out: &mut [u8]) -> bool {
        for byte in out.iter_mut() {
            *byte = self.0;
            self.0 = self.0.wrapping_add(1);
        }
        true
    }
}

/// One page, in the clear, with an exact cross-reference table.
fn plain() -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources << >> /Contents 4 0 R >>",
        "<< /Length 19 >>\nstream\n0 g 0 0 20 20 re f\nendstream",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let _ = writeln!(out, "xref\n0 5\n0000000000 65535 f ");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 5 /Root 1 0 R /ID [<0102> <0304>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// [`plain`] written under this tree's revision 6 handler, with the default user password.
fn encrypted() -> Vec<u8> {
    let source = Document::open_with_limits(plain(), Limits::DEFAULT).expect("the fixture opens");
    let mut assembly = Assembly::new(vec![&source]);
    for number in 1..=4 {
        assembly
            .copy(0, ObjectId::new(number, 0))
            .expect("the assembly takes the object");
    }
    assembly.set_root(ObjectId::new(1, 0));
    let mut out = Vec::new();
    serialize_encrypted(
        &assembly,
        Version { major: 1, minor: 7 },
        Options::new(Form::Table),
        &Protection {
            user_password: "",
            owner_password: "keeper",
            access: Access::ALL,
            encrypt_metadata: true,
        },
        &mut Counter(0),
        &mut out,
    )
    .expect("the file is written");
    out
}

/// The report's notes for `bytes`, opened with no password.
fn report(bytes: Vec<u8>) -> Vec<String> {
    let mut viewer = Viewer::new(100, 100, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "A953: the document opens: {opened:?}"
    );
    viewer
        .handle(Command::Report)
        .flat_map(|event| match event {
            Event::Reported { notes, .. } => notes,
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn a_perms_block_that_does_not_verify_is_said_in_the_report() {
    let written = encrypted();
    // Control: the block this writer wrote verifies, and the report says nothing about it.
    assert!(
        !report(written.clone())
            .iter()
            .any(|note| note.contains("not bound")),
        "a verified block is not reported"
    );

    let at = written
        .windows(6)
        .position(|window| window == b"/Perms")
        .expect("Table 21: a revision 6 file states /Perms");
    let open = at
        + written
            .get(at..)
            .and_then(|rest| rest.iter().position(|&byte| byte == b'<'))
            .expect("the block is a hexadecimal string");
    let close = at
        + written
            .get(at..)
            .and_then(|rest| rest.iter().position(|&byte| byte == b'>'))
            .expect("the hexadecimal string closes");
    let mut tampered = written.clone();
    tampered
        .get_mut(open.saturating_add(1)..close)
        .expect("the string is in range")
        .fill(b'0');

    let notes = report(tampered);
    assert!(
        notes.iter().any(|note| note.contains("§7.6.4.3.3 step (f)")
            && note.contains("not bound to its encryption key")
            && note.contains("/P entry as written")),
        "the report says the permissions are not bound to the key: {notes:?}"
    );
}
