//! Table24 owner grant reaches the operation-permission consumer.
#[test]
fn public_key_owner_permission_overrides_each_user_grant_field() {
    use pdf_model::restriction::{Operation, withheld};
    use pdf_syntax::Permissions;

    let owner_permissions = Permissions {
        owner: true,
        revision: 3,
        print: false,
        modify: false,
        copy: false,
        annotate: false,
        fill_forms: false,
        assemble: false,
        print_faithfully: false,
    };
    for operation in [
        Operation::FillInForm,
        Operation::Annotate,
        Operation::Print,
        Operation::PrintFaithfully,
        Operation::Extract,
        Operation::Modify,
        Operation::Assemble,
    ] {
        let user_permissions = Permissions {
            owner: false,
            ..owner_permissions
        };
        assert!(
            withheld(user_permissions, operation).is_some(),
            "user has no {operation:?} grant"
        );
        assert_eq!(
            withheld(owner_permissions, operation),
            None,
            "{operation:?}"
        );
    }
}
