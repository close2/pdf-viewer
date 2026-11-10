// Selected positive and negative Table123 collection controls.
// It uses Table 123's collection rows as the independent expected mapping and does not require a
// system font.

#[test]
fn each_table_123_collection_row_recognizes_its_distinctive_character_classes() {
    use super::Decision;

    // One clearly character-described class from each separately listed collection row. Japan2
    // is deprecated and KR is added in PDF 2.0, but Table 123 still gives each its stated set.
    for (registry, ordering, class, character) in [
        ("Adobe", "GB1", "Hanzi", '\u{4e00}'),
        ("Adobe", "CNS1", "Hanzi", '\u{4e00}'),
        ("Adobe", "Japan1", "HKana", '\u{ff71}'),
        ("Adobe", "Japan2", "HojoKanji", '\u{4e00}'),
        ("Adobe", "Korea1", "Hangul", '\u{d55c}'),
        ("Adobe", "KR", "Hangul", '\u{d55c}'),
    ] {
        let decision = Decision::read(&[class.to_owned()], Some((registry, ordering)));
        assert_eq!(decision.refused(), &[], "{registry}-{ordering} /{class}");
        assert_eq!(
            decision.class_for('A'),
            None,
            "Latin is outside this selected class"
        );
        assert_eq!(
            decision.class_for(character),
            Some(0),
            "{registry}-{ordering} /{class} should include {character:?}"
        );
    }

    // The table's row is part of the key: a name listed in a different collection is not silently
    // treated as the same class just because the name has a familiar script meaning.
    for (ordering, class, character) in [
        ("GB1", "HojoKanji", '一'),
        ("CNS1", "HojoKanji", '一'),
        ("Japan2", "HKana", 'ｱ'),
        ("Korea1", "AlphaNum", '7'),
        ("KR", "AlphaNum", '7'),
    ] {
        let decision = Decision::read(&[class.to_owned()], Some(("Adobe", ordering)));
        assert_eq!(decision.class_for(character), None, "{ordering} /{class}");
        assert_eq!(decision.refused().len(), 1, "{ordering} /{class}");
        assert!(decision.refused()[0].1.contains("does not list"));
    }
}
