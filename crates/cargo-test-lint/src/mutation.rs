use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MutationStatus {
    Caught,
    Survived,
    Timeout,
    Unviable,
    Skipped,
}

impl MutationStatus {
    pub const ALL: [MutationStatus; 5] = [
        MutationStatus::Caught,
        MutationStatus::Survived,
        MutationStatus::Timeout,
        MutationStatus::Unviable,
        MutationStatus::Skipped,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mutation {
    pub id: usize,
    pub location: SourceLocation,
    pub status: MutationStatus,
    pub original: String,
    pub replacement: String,
}

#[derive(Debug)]
pub struct MutationSet {
    pub mutations: Vec<Mutation>,
}

impl MutationSet {
    pub fn count_by_status(&self, status: &MutationStatus) -> usize {
        self.mutations.iter().filter(|m| &m.status == status).count()
    }

    pub fn total(&self) -> usize {
        self.mutations.len()
    }

    pub fn surviving(&self) -> Vec<&Mutation> {
        self.mutations.iter().filter(|m| m.status == MutationStatus::Survived).collect()
    }

    pub fn count_invariant_holds(&self) -> bool {
        MutationStatus::ALL.iter().map(|s| self.count_by_status(s)).sum::<usize>() == self.total()
    }

    pub fn no_double_count(&self) -> bool {
        use std::collections::HashSet;
        let mut seen_ids = HashSet::with_capacity(self.mutations.len());
        self.mutations.iter().all(|m| seen_ids.insert(m.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_status() -> impl Strategy<Value = MutationStatus> {
        prop_oneof![
            Just(MutationStatus::Caught),
            Just(MutationStatus::Survived),
            Just(MutationStatus::Timeout),
            Just(MutationStatus::Unviable),
            Just(MutationStatus::Skipped),
        ]
    }

    fn arb_source_location() -> impl Strategy<Value = SourceLocation> {
        (any::<String>(), 1usize..10000, 1usize..1000).prop_map(|(file, line, column)| {
            SourceLocation { file: PathBuf::from(file), line, column }
        })
    }

    fn arb_mutation() -> impl Strategy<Value = Mutation> {
        (any::<usize>(), arb_source_location(), arb_status(), any::<String>(), any::<String>())
            .prop_map(|(id, location, status, original, replacement)| Mutation {
                id,
                location,
                status,
                original,
                replacement,
            })
    }

    fn arb_mutation_set() -> impl Strategy<Value = MutationSet> {
        proptest::collection::vec(arb_mutation(), 0..=100)
            .prop_map(|mutations| MutationSet { mutations })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        #[test]
        fn total_classification_every_mutant_has_exactly_one_status(
            mutations in arb_mutation_set()
        ) {
            for m in &mutations.mutations {
                let matches_one = MutationStatus::ALL.iter().filter(|s| **s == m.status).count();
                prop_assert_eq!(matches_one, 1);
            }
        }

        #[test]
        fn surviving_is_subset_of_all(
            mutations in arb_mutation_set()
        ) {
            let surviving = mutations.surviving();
            for s in &surviving {
                prop_assert!(mutations.mutations.iter().any(|m| m.id == s.id));
            }
            prop_assert!(surviving.len() <= mutations.total());
        }

        #[test]
        fn status_is_exhaustive(
            status in arb_status()
        ) {
            prop_assert!(MutationStatus::ALL.iter().any(|s| s == &status));
        }

        #[test]
        fn count_invariant_holds(
            mutations in arb_mutation_set()
        ) {
            prop_assert!(mutations.count_invariant_holds());
        }

        #[test]
        fn no_double_count(
            mutations in arb_mutation_set()
        ) {
            prop_assert!(mutations.no_double_count());
        }
    }

    #[test]
    fn edge_case_all_caught() {
        let set = MutationSet { mutations: vec![make_mutation(0, MutationStatus::Caught)] };
        assert_eq!(set.count_by_status(&MutationStatus::Caught), 1);
        assert_eq!(set.count_by_status(&MutationStatus::Survived), 0);
        assert!(set.count_invariant_holds());
    }

    #[test]
    fn edge_case_all_survived() {
        let set = MutationSet {
            mutations: vec![
                make_mutation(0, MutationStatus::Survived),
                make_mutation(1, MutationStatus::Survived),
            ],
        };
        assert_eq!(set.surviving().len(), 2);
        assert!(set.count_invariant_holds());
    }

    #[test]
    fn edge_case_mixed() {
        let set = MutationSet {
            mutations: vec![
                make_mutation(0, MutationStatus::Caught),
                make_mutation(1, MutationStatus::Survived),
                make_mutation(2, MutationStatus::Timeout),
                make_mutation(3, MutationStatus::Unviable),
                make_mutation(4, MutationStatus::Skipped),
            ],
        };
        assert!(set.count_invariant_holds());
        assert_eq!(set.total(), 5);
        assert_eq!(set.surviving().len(), 1);
    }

    #[test]
    fn edge_case_empty_set() {
        let set = MutationSet { mutations: vec![] };
        assert_eq!(set.total(), 0);
        assert!(set.count_invariant_holds());
        assert!(set.surviving().is_empty());
    }

    fn make_mutation(id: usize, status: MutationStatus) -> Mutation {
        Mutation {
            id,
            location: SourceLocation { file: PathBuf::from("src/lib.rs"), line: 1, column: 1 },
            status,
            original: "a".into(),
            replacement: "b".into(),
        }
    }
}
