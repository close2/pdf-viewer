//! Fuzzes `pdf_model::aform`: a field's `/JS` text read as one `AF*` call, and the call run at each
//! of ISO 32000-2 §12.6.3's Table 199 triggers over values a person or a producer chose (ADRs 1578,
//! 1579).
//!
//! The script is a producer's, and since Tier 0 it is read on every keystroke, format and
//! calculation a form raises: `Call::parse` is a hand-written grammar over untrusted text, and each
//! of the twenty-one functions reads its literal arguments — numbers of any size, strings of any
//! length, pictures and masks of any shape — and then the field's value. Neither half is reached by
//! another target: `page` and `forms_data` never raise a trigger.
//!
//! The input is three texts separated by NUL bytes — the script, the field's value, and the
//! keystroke's change — read lossily so that every input is one. The value's first two bytes, as
//! byte offsets modulo its length plus one, are the keystroke's selection, so a stale or inverted
//! selection a host might hand over is reached as well.
//!
//! Beyond never panicking — overflow checks stay on in this profile — two properties:
//!
//! - **A call parses back the same.** Parsing is a function of the text, so a script that parses
//!   once parses again to the same call.
//! - **A keystroke example stands.** `Call::accepted_example` claims a value the script lets stand
//!   both typed and committed; the target types it and commits it again and checks the claim.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_model::aform::{
    Call, DateTime, Keyed, Keystroke, extract_nums, make_number, number_text, parse_date,
    print_date, print_mask,
};

/// A moment every picture can print: each field away from its edge.
const MOMENT: DateTime = DateTime {
    year: 2024,
    month: 2,
    day: 29,
    hour: 23,
    minute: 5,
    second: 9,
};

fuzz_target!(|data: &[u8]| {
    // The grammar refuses a script past 64 KiB before it reads a byte, so a longer input tests
    // only that comparison.
    if data.len() > 80 * 1024 {
        return;
    }
    let mut parts = data.splitn(3, |&byte| byte == 0);
    let script = String::from_utf8_lossy(parts.next().unwrap_or_default());
    let value = String::from_utf8_lossy(parts.next().unwrap_or_default());
    let change = String::from_utf8_lossy(parts.next().unwrap_or_default());
    let span = value.len().saturating_add(1);
    let selection = (
        usize::from(value.bytes().next().unwrap_or(0))
            .checked_rem(span)
            .unwrap_or(0),
        usize::from(value.bytes().nth(1).unwrap_or(0))
            .checked_rem(span)
            .unwrap_or(0),
    );

    // The helpers, on the value and the change as a picture or a mask.
    if let Some(number) = make_number(&value) {
        let _ = number_text(number);
    }
    let _ = extract_nums(&value);
    let _ = parse_date(&value, &change);
    let _ = print_date(&change, &MOMENT);
    let _ = print_mask(&change, &value);

    let Ok(call) = Call::parse(&script) else {
        return;
    };
    assert_eq!(
        Call::parse(&script).as_ref(),
        Ok(&call),
        "parsing one script twice gave two calls"
    );

    let _ = call.format(&value);
    let _ = call.validate(&value);
    let _ =
        call.calculate(&mut |name| vec![name.to_owned(), value.to_string(), change.to_string()]);
    for will_commit in [false, true] {
        let _ = call.keystroke(&Keystroke {
            value: &value,
            change: &change,
            selection,
            will_commit,
        });
    }

    if let Some(example) = call.accepted_example() {
        let committed = call.keystroke(&Keystroke {
            value: &example,
            change: "",
            selection: (example.len(), example.len()),
            will_commit: true,
        });
        assert!(
            matches!(committed, Ok(Keyed::Accepted { .. })),
            "{call:?} offered {example:?} as a value it accepts and then refused it: {committed:?}"
        );
    }
});
