//! Fuzzes the script worker's wire: the run a host sends and the reply a worker sends back (ADR
//! 1609).
//!
//! **Both ends are a parser over the other's bytes.** `pdf-script-worker` runs a document's scripts
//! behind `pdf_sandbox::lockdown::Profile::Script`, and what it writes back is read by a host that
//! is not confined — so the host's decoder is a parser over bytes a subverted worker chose, which is
//! `confined_wire`'s argument for `pdf-view-worker`. The worker's decoder is here for the mirror
//! reason: what reaches its standard input is whatever reached it.
//!
//! The input is one message: decoded whole as a run, and as a reply whose frame kind is its first
//! byte and whose payload is the rest. Beyond never panicking — overflow checks stay on in this
//! profile — two properties:
//!
//! - **What decodes once decodes again to the same.** The readers carry a cursor and nest
//!   `pdf_script::wire`'s request and outcome inside this module's frames, and a reader that leaked
//!   state between the two would answer differently the second time.
//! - **What decodes re-encodes to what decodes the same.** A run and a reply the host or the worker
//!   would act on survive the trip their encoder makes. Compared as they print, because a field's
//!   rectangle and a colour cross as `f64`, and a `NaN` that is the same bits is not `==` itself.
//!
//! A form's `FormCalc` run is a conversation on the same pipes (ADR 2524): the input is also decoded as
//! a form run, and as a question and as an answer by the same first-byte rule, with the same two
//! properties.
//!

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` is how a violated one reaches libFuzzer, and each message names the property"
)]

use libfuzzer_sys::fuzz_target;
use pdf_script_worker::form_wire::{
    decode_answer, decode_form_run, decode_question, encode_answer, encode_form_run,
    encode_question,
};
use pdf_script_worker::wire::{decode_reply, decode_run, encode_reply, encode_run};

fuzz_target!(|data: &[u8]| {
    if let Ok(run) = decode_run(data) {
        let again = decode_run(data).expect("bytes that decoded once decode again");
        assert_eq!(
            format!("{run:?}"),
            format!("{again:?}"),
            "the same run decoded twice gave two different runs"
        );
        let trip = decode_run(&encode_run(&run)).expect("a decoded run re-encodes to a run");
        assert_eq!(
            format!("{run:?}"),
            format!("{trip:?}"),
            "a run changed on its way through the encoder"
        );
    }

    if let Some((&kind, payload)) = data.split_first()
        && let Ok(reply) = decode_reply(kind, payload)
    {
        let again = decode_reply(kind, payload).expect("bytes that decoded once decode again");
        assert_eq!(
            format!("{reply:?}"),
            format!("{again:?}"),
            "the same reply decoded twice gave two different replies"
        );
        let (kind, bytes) = encode_reply(&reply);
        let trip = decode_reply(kind, &bytes).expect("a decoded reply re-encodes to a reply");
        assert_eq!(
            format!("{reply:?}"),
            format!("{trip:?}"),
            "a reply changed on its way through the encoder"
        );
    }

    if let Ok(run) = decode_form_run(data) {
        let trip = decode_form_run(&encode_form_run(&run)).expect("a decoded form run re-encodes");
        assert_eq!(
            format!("{run:?}"),
            format!("{trip:?}"),
            "a form run changed on its way through the encoder"
        );
    }

    if let Some((&kind, payload)) = data.split_first() {
        if let Ok(question) = decode_question(kind, payload) {
            let (kind, bytes) = encode_question(&question);
            let trip = decode_question(kind, &bytes).expect("a decoded question re-encodes");
            assert_eq!(
                format!("{question:?}"),
                format!("{trip:?}"),
                "a question changed on its way through the encoder"
            );
        }
        if let Ok(answer) = decode_answer(kind, payload) {
            let (kind, bytes) = encode_answer(&answer);
            let trip = decode_answer(kind, &bytes).expect("a decoded answer re-encodes");
            assert_eq!(
                format!("{answer:?}"),
                format!("{trip:?}"),
                "an answer changed on its way through the encoder"
            );
        }
    }
});
