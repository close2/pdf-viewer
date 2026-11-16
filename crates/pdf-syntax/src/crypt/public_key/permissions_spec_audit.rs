//! Source-derived Table24 mapping controls; CMS decryption is exercised separately.

#[test]
fn table24_public_key_flags_map_one_based_positions_and_subfilter_revisions() {
    use super::{SubFilter, permissions};

    // Table 24 numbers positions from one at the low-order bit. Independent source-derived
    // controls set every named permission position, including 2 (owner), while also setting
    // ignored bit 10. Bits outside the permitted set are separately exercised below.
    let table24_word: u32 = [2_u32, 3, 4, 5, 6, 9, 10, 11, 12]
        .into_iter()
        .fold(0, |word, position| word | (1 << (position - 1)));
    let s4 = permissions(table24_word, SubFilter::S4);
    assert_eq!(s4.revision, 3);
    assert!(s4.owner);
    assert!(s4.print);
    assert!(s4.modify);
    assert!(s4.copy);
    assert!(s4.annotate);
    assert!(s4.fill_forms);
    assert!(s4.assemble);
    assert!(s4.print_faithfully);

    // Positive bit controls make a shared all-zero result impossible to pass; each field is
    // pinned to the corresponding Table 24 bit rather than to a test helper's interpretation.
    for (position, expected) in [
        (2, (true, false, false, false, false, false, false, false)),
        (3, (false, true, false, false, false, false, false, false)),
        (4, (false, false, true, false, false, false, false, false)),
        (5, (false, false, false, true, false, false, false, false)),
        (6, (false, false, false, false, true, false, false, false)),
        (9, (false, false, false, false, false, true, false, false)),
        (11, (false, false, false, false, false, false, true, false)),
        (12, (false, false, false, false, false, false, false, true)),
    ] {
        let value = permissions(1 << (position - 1), SubFilter::S4);
        assert_eq!(
            (
                value.owner,
                value.print,
                value.modify,
                value.copy,
                value.annotate,
                value.fill_forms,
                value.assemble,
                value.print_faithfully,
            ),
            expected,
            "Table 24 position {position}"
        );
    }

    // Position 10 is expressly ignored by processors. Writers shall set it to 1 for compatibility
    // with older readers; this word is a processor test, not a proposed conforming writer word.
    // Positions not named by Table 24's processor-ignore sentence are ignored too; none maps to a
    // grant.
    let ignored = permissions((1 << 9) | 1 | (1 << 12) | (1 << 31), SubFilter::S5);
    assert_eq!(ignored.revision, 3);
    assert_eq!(
        (
            ignored.owner,
            ignored.print,
            ignored.modify,
            ignored.copy,
            ignored.annotate,
            ignored.fill_forms,
            ignored.assemble,
            ignored.print_faithfully,
        ),
        (false, false, false, false, false, false, false, false)
    );

    assert_eq!(permissions(table24_word, SubFilter::S3).revision, 2);
    assert_eq!(permissions(table24_word, SubFilter::S5).revision, 3);
}
