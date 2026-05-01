use ctl_core::mutation::MutationKind;

#[test]
fn from_cargo_mutants_name_known_variants() {
    assert_eq!(
        MutationKind::from_cargo_mutants_name("replace_arithmetic_operator"),
        MutationKind::Arithmetic
    );
    assert_eq!(MutationKind::from_cargo_mutants_name("replace_boundary"), MutationKind::Boundary);
    assert_eq!(
        MutationKind::from_cargo_mutants_name("negate_condition"),
        MutationKind::NegateCondition
    );
    assert_eq!(
        MutationKind::from_cargo_mutants_name("void_return_value"),
        MutationKind::VoidReturnValue
    );
    assert_eq!(
        MutationKind::from_cargo_mutants_name("replace_with_default"),
        MutationKind::ReplaceWithDefault
    );
    assert_eq!(
        MutationKind::from_cargo_mutants_name("replace_with_literal"),
        MutationKind::ReplaceWithLiteral
    );
    assert_eq!(
        MutationKind::from_cargo_mutants_name("replace_operator"),
        MutationKind::ReplaceOperator
    );
    assert_eq!(MutationKind::from_cargo_mutants_name("remove_statement"), MutationKind::RemoveStmt);
    assert_eq!(
        MutationKind::from_cargo_mutants_name("unwrap_to_expect"),
        MutationKind::UnwrapToExpect
    );
}

#[test]
fn from_cargo_mutants_name_unknown_variant() {
    assert_eq!(
        MutationKind::from_cargo_mutants_name("some_new_mutation"),
        MutationKind::Other("some_new_mutation".to_string())
    );
}

#[test]
fn mutation_report_empty() {
    let r = MutationKind::from_cargo_mutants_name("replace_arithmetic_operator");
    assert!(matches!(r, MutationKind::Arithmetic));
}

#[test]
fn mutation_kind_equality_other() {
    let a = MutationKind::Other("foo".into());
    let b = MutationKind::Other("foo".into());
    assert_eq!(a, b);

    let c = MutationKind::Other("bar".into());
    assert_ne!(a, c);
}

#[test]
fn mutation_report_empty_fn() {
    let report = ctl_core::mutation::MutationReport::empty();
    assert!(report.mutants.is_empty());
    assert_eq!(report.total, 0);
    assert_eq!(report.survived, 0);
    assert_eq!(report.killed, 0);
    assert_eq!(report.timeout, 0);
}
