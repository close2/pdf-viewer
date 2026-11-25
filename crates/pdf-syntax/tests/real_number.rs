//! `pdf_syntax::write`'s formatter for a number this program computed (ADR 3686).
//!
//! ISO 32000-2 §7.3.3: a real "shall be written as one or more decimal digits optionally preceded
//! by a sign and containing a period", and "\[a] PDF writer shall not use the PostScript language
//! syntax for numbers with non-decimal radices (such as 16#FFFE) or in exponential format (such as
//! 6.02E23)". An infinity and a NaN have no such spelling, so they are refused.

use pdf_syntax::write::{NonFinite, real_number, real_number_f32};

/// Every non-finite value is refused, naming itself.
#[test]
fn an_infinity_or_a_nan_is_refused_rather_than_written() {
    assert_eq!(
        real_number(f64::INFINITY),
        Err(NonFinite {
            value: f64::INFINITY
        })
    );
    assert!(real_number(f64::NEG_INFINITY).is_err());
    assert!(real_number(f64::NAN).is_err());
    assert!(real_number_f32(f32::INFINITY).is_err());
    assert!(real_number_f32(f32::NAN).is_err());
    assert!(real_number_f32(f32::MAX - -f32::MAX).is_err());
}

/// A finite value is written in §7.3.3's syntax — an integer where it is integral, which the
/// clause permits "wherever a real number is expected", and never an exponent — and reads back
/// as itself.
#[test]
fn a_finite_value_is_written_without_an_exponent_and_reads_back() {
    assert_eq!(real_number(12.0).as_deref(), Ok("12"));
    assert_eq!(real_number(-0.5).as_deref(), Ok("-0.5"));
    assert_eq!(real_number(0.000_976_562_5).as_deref(), Ok("0.0009765625"));
    assert_eq!(real_number_f32(0.1).as_deref(), Ok("0.1"));
    for value in [1e30_f64, 6.02e23, 1e-7, f64::MAX] {
        let written = real_number(value).expect("finite");
        assert!(!written.contains(['e', 'E']), "{written}");
        assert_eq!(written.parse::<f64>().ok(), Some(value));
    }
    let written = real_number_f32(1e-7).expect("finite");
    assert!(!written.contains(['e', 'E']), "{written}");
    assert_eq!(written.parse::<f32>().ok(), Some(1e-7));
}
