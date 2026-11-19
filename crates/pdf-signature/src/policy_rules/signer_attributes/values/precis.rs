//! RFC 8265's two PRECIS profiles a Jabber identifier is built of (RFC 7622 sections 3.3 and
//! 3.4): `UsernameCaseMapped` over RFC 8264's `IdentifierClass` and `OpaqueString` over its
//! `FreeformClass`, each prepared and enforced in its sections' order (ADR 3161). RFC 7622 names
//! RFC 7613, which RFC 8265 obsoletes with the same two profiles.

use unicode_normalization::UnicodeNormalization as _;
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory as _};

use super::code_points::{self, Class};

/// The code points whose Unicode decomposition type is `<wide>` or `<narrow>`, as runs of
/// consecutive code points `first..=last` whose decomposition mappings are consecutive from
/// `to`: RFC 8264 section 5.2.1's width mapping maps each to its decomposition mapping. Read off
/// `UnicodeData.txt`'s fifth field (226 code points; Unicode 15.1, through Python's
/// `unicodedata.decomposition`), and checked against Normalization Form KC by this module's
/// test: a compatibility decomposition is what NFKC applies, so each entry and its mapping have
/// one NFKC form.
const WIDTH: &[(u32, u32, u32)] = &[
    (0x3000, 0x3000, 0x0020),
    (0xFF01, 0xFF5E, 0x0021),
    (0xFF5F, 0xFF60, 0x2985),
    (0xFF61, 0xFF61, 0x3002),
    (0xFF62, 0xFF63, 0x300C),
    (0xFF64, 0xFF64, 0x3001),
    (0xFF65, 0xFF65, 0x30FB),
    (0xFF66, 0xFF66, 0x30F2),
    (0xFF67, 0xFF67, 0x30A1),
    (0xFF68, 0xFF68, 0x30A3),
    (0xFF69, 0xFF69, 0x30A5),
    (0xFF6A, 0xFF6A, 0x30A7),
    (0xFF6B, 0xFF6B, 0x30A9),
    (0xFF6C, 0xFF6C, 0x30E3),
    (0xFF6D, 0xFF6D, 0x30E5),
    (0xFF6E, 0xFF6E, 0x30E7),
    (0xFF6F, 0xFF6F, 0x30C3),
    (0xFF70, 0xFF70, 0x30FC),
    (0xFF71, 0xFF71, 0x30A2),
    (0xFF72, 0xFF72, 0x30A4),
    (0xFF73, 0xFF73, 0x30A6),
    (0xFF74, 0xFF74, 0x30A8),
    (0xFF75, 0xFF76, 0x30AA),
    (0xFF77, 0xFF77, 0x30AD),
    (0xFF78, 0xFF78, 0x30AF),
    (0xFF79, 0xFF79, 0x30B1),
    (0xFF7A, 0xFF7A, 0x30B3),
    (0xFF7B, 0xFF7B, 0x30B5),
    (0xFF7C, 0xFF7C, 0x30B7),
    (0xFF7D, 0xFF7D, 0x30B9),
    (0xFF7E, 0xFF7E, 0x30BB),
    (0xFF7F, 0xFF7F, 0x30BD),
    (0xFF80, 0xFF80, 0x30BF),
    (0xFF81, 0xFF81, 0x30C1),
    (0xFF82, 0xFF82, 0x30C4),
    (0xFF83, 0xFF83, 0x30C6),
    (0xFF84, 0xFF84, 0x30C8),
    (0xFF85, 0xFF8A, 0x30CA),
    (0xFF8B, 0xFF8B, 0x30D2),
    (0xFF8C, 0xFF8C, 0x30D5),
    (0xFF8D, 0xFF8D, 0x30D8),
    (0xFF8E, 0xFF8E, 0x30DB),
    (0xFF8F, 0xFF93, 0x30DE),
    (0xFF94, 0xFF94, 0x30E4),
    (0xFF95, 0xFF95, 0x30E6),
    (0xFF96, 0xFF9B, 0x30E8),
    (0xFF9C, 0xFF9C, 0x30EF),
    (0xFF9D, 0xFF9D, 0x30F3),
    (0xFF9E, 0xFF9F, 0x3099),
    (0xFFA0, 0xFFA0, 0x3164),
    (0xFFA1, 0xFFBE, 0x3131),
    (0xFFC2, 0xFFC7, 0x314F),
    (0xFFCA, 0xFFCF, 0x3155),
    (0xFFD2, 0xFFD7, 0x315B),
    (0xFFDA, 0xFFDC, 0x3161),
    (0xFFE0, 0xFFE1, 0x00A2),
    (0xFFE2, 0xFFE2, 0x00AC),
    (0xFFE3, 0xFFE3, 0x00AF),
    (0xFFE4, 0xFFE4, 0x00A6),
    (0xFFE5, 0xFFE5, 0x00A5),
    (0xFFE6, 0xFFE6, 0x20A9),
    (0xFFE8, 0xFFE8, 0x2502),
    (0xFFE9, 0xFFEC, 0x2190),
    (0xFFED, 0xFFED, 0x25A0),
    (0xFFEE, 0xFFEE, 0x25CB),
];

/// The longest a localpart or resourcepart may be once enforced, in octets (RFC 7622 sections
/// 3.3 and 3.4).
pub(super) const MAX_PART: usize = 1023;

/// RFC 8264 section 5.2.1's width mapping of one code point.
pub(super) fn width(c: char) -> char {
    let point = u32::from(c);
    WIDTH
        .iter()
        .find(|&&(first, last, _)| (first..=last).contains(&point))
        .and_then(|&(first, _, to)| {
            let offset = point.checked_sub(first)?;
            char::from_u32(to.checked_add(offset)?)
        })
        .unwrap_or(c)
}

/// RFC 8265 section 3.3: `text` prepared (3.3.2: width mapping, then every code point allowed by
/// the `IdentifierClass`) and enforced (3.3.3: `toLowerCase`, Normalization Form C, the Bidi Rule
/// where a right-to-left code point is held, and not empty), or `None` where it is not a
/// conforming string.
pub(super) fn username_case_mapped(text: &str) -> Option<String> {
    let prepared: Vec<char> = text.chars().map(width).collect();
    if !code_points::all_allowed(&prepared, |c| code_points::precis(c, Class::Identifier)) {
        return None;
    }
    // Rust's `str::to_lowercase` is the Unicode Standard's `toLowerCase`, with the final sigma's
    // context, as section 3.3.1's case mapping rule prefers.
    let lowered = prepared.iter().collect::<String>().to_lowercase();
    let enforced: Vec<char> = lowered.nfc().collect();
    if code_points::right_to_left(&enforced) && !code_points::bidi_rule(&enforced) {
        return None;
    }
    let enforced: String = enforced.into_iter().collect();
    (!enforced.is_empty()).then_some(enforced)
}

/// RFC 8265 section 4.2: `text` prepared (4.2.1: every code point allowed by the
/// `FreeformClass`) and enforced (4.2.2: no width mapping, every space of category `Zs` but SPACE
/// mapped to SPACE, no case mapping, Normalization Form C, no directionality rule), or `None`
/// where it is not a conforming string. Section 4.2's empty string is not a resourcepart (RFC
/// 7622 section 3.4), so it is refused here.
pub(super) fn opaque_string(text: &str) -> Option<String> {
    let prepared: Vec<char> = text.chars().collect();
    if !code_points::all_allowed(&prepared, |c| code_points::precis(c, Class::Freeform)) {
        return None;
    }
    let mapped = prepared.into_iter().map(|c| {
        if c != ' ' && c.general_category() == GeneralCategory::SpaceSeparator {
            ' '
        } else {
            c
        }
    });
    let enforced: String = mapped.collect::<String>().nfc().collect();
    (!enforced.is_empty()).then_some(enforced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_width_mapping_keeps_its_compatibility_form() {
        for &(first, last, to) in WIDTH {
            for point in first..=last {
                let from = char::from_u32(point).expect("listed code points are scalar values");
                let mapped = width(from);
                assert_eq!(u32::from(mapped), to + (point - first));
                assert!(
                    from.to_string().nfkc().eq(mapped.to_string().nfkc()),
                    "U+{point:04X}"
                );
            }
        }
        // RFC 8264 section 5.2.1's own example: FULLWIDTH DIGIT ZERO is DIGIT ZERO.
        assert_eq!(width('\u{FF10}'), '0');
        assert_eq!(width('a'), 'a');
    }
}
