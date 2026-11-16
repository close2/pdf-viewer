//! Interpretation regression: resolve the signed selector before applying later field renames.
//! Table 259 does not expressly prescribe cross-revision identity; this tests the comparison API,
//! not signature authenticity or a viewer enforcement path.
use super::*;

const CATALOG_WITH_ONE_TEXT_FIELD: &str =
    "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R 6 0 R] /SigFlags 3 >> >>";
const CATALOG_WITH_TWO_TEXT_FIELDS: &str =
    "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R 6 0 R 7 0 R] /SigFlags 3 >> >>";
const PAGES: &str = "<< /Type /Pages /Count 1 /Kids [3 0 R] >>";
const OTHER_FIELD: &str =
    "<< /FT /Tx /T (Other) /Subtype /Widget /Type /Annot /Rect [100 200 300 220] /P 3 0 R >>";
const FIELD_MDP_SIGNATURE: &str = "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
     /ByteRange [0000000000 0000000000 0000000000 0000000000] \
     /Contents <00112233445566778899aabbccddeeff> /Reference [ \
     << /Type /SigRef /TransformMethod /FieldMDP /Data 1 0 R /DigestMethod /SHA256 \
     /TransformParams << /Type /TransformParams /Action /Include /Fields [(Name)] /V /1.2 >> >>] >>";

fn field_mdp_signed_form(catalog: &str, extra: &[&str]) -> Vec<u8> {
    signed_file_over(catalog, PAGES, FIELD_MDP_SIGNATURE, extra)
}

fn extracted_field_mdp(document: &Document) -> FieldMdp {
    let transforms = crate::signature::field_mdp(document);
    assert_eq!(transforms.len(), 1, "one signed FieldMDP transform");
    transforms.into_iter().next().expect("the transform exists")
}

#[test]
fn a_fieldmdp_selection_keeps_the_signed_field_identity_across_a_rename() {
    // Positive control 1: a selected field with a stable name is covered when its value changes.
    let mut stable = field_mdp_signed_form(CATALOG_WITH_ONE_TEXT_FIELD, &[EMPTY_FIELD]);
    update(
        &mut stable,
        &[(
            6,
            "<< /FT /Tx /T (Name) /Subtype /Widget /Type /Annot /Rect [100 200 300 220] /P 3 0 R /V (Ada) >>",
        )],
        &[],
        1,
    );
    let stable = Document::open(stable).expect("exact-xref incremental PDF");
    assert!(!stable.was_recovered());
    assert!(!stable.cross_reference_chain_was_cut());
    let stable_comparison = Comparison::of(&only_signature(&stable), &stable)
        .expect("the signed prefix is a comparable state");
    assert_eq!(
        scoped(&stable_comparison, &extracted_field_mdp(&stable)),
        ["6 Name Covered"]
    );

    // Positive control 2: a different field outside Include(Name) stays outside.
    let mut outside =
        field_mdp_signed_form(CATALOG_WITH_TWO_TEXT_FIELDS, &[EMPTY_FIELD, OTHER_FIELD]);
    update(
        &mut outside,
        &[(
            7,
            "<< /FT /Tx /T (Other) /Subtype /Widget /Type /Annot /Rect [100 200 300 220] /P 3 0 R /V (Ada) >>",
        )],
        &[],
        1,
    );
    let outside = Document::open(outside).expect("exact-xref incremental PDF");
    assert!(!outside.was_recovered());
    assert!(!outside.cross_reference_chain_was_cut());
    let outside_comparison = Comparison::of(&only_signature(&outside), &outside)
        .expect("the signed prefix is a comparable state");
    assert_eq!(
        scoped(&outside_comparison, &extracted_field_mdp(&outside)),
        ["7 Other Outside"]
    );

    // Target: object 6 is the signed Name field, then the same object is renamed and filled.
    let mut renamed = field_mdp_signed_form(CATALOG_WITH_ONE_TEXT_FIELD, &[EMPTY_FIELD]);
    update(
        &mut renamed,
        &[(
            6,
            "<< /FT /Tx /T (Other) /Subtype /Widget /Type /Annot /Rect [100 200 300 220] /P 3 0 R /V (Ada) >>",
        )],
        &[],
        1,
    );
    let renamed = Document::open(renamed).expect("exact-xref incremental PDF");
    assert!(!renamed.was_recovered());
    assert!(!renamed.cross_reference_chain_was_cut());
    let renamed_comparison = Comparison::of(&only_signature(&renamed), &renamed)
        .expect("the signed prefix is a comparable state");
    let ranking = renamed_comparison
        .against_field_mdp(&extracted_field_mdp(&renamed))
        .expect("/Data names the whole form's catalog");

    assert_eq!(
        ranking.covered.count(),
        1,
        "Table 259's selector was signed against the Name field; changing that same field's /T must not make its /V update escape Include(Name)"
    );
    assert_eq!(ranking.outside.count(), 0);
    assert_eq!(
        ranking.judgement(),
        FieldJudgement::CoveredFieldChanged { objects: 1 }
    );
}
