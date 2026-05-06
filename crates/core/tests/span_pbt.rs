use test_lint_core::span::Span;
use proptest::collection::vec;
use proptest::prelude::*;

prop_compose! {
    fn arb_span()(
        start_line in 1usize..=100,
        start_col in 1usize..=200,
        end_line in 1usize..=100,
        end_col in 1usize..=200,
    ) -> Span {
        if start_line > end_line || (start_line == end_line && start_col > end_col) {
            Span { start_line: end_line, start_col: end_col, end_line: start_line, end_col: start_col }
        } else {
            Span { start_line, start_col, end_line, end_col }
        }
    }
}

prop_compose! {
    fn arb_span_set()(spans in vec(arb_span(), 1..=20)) -> Vec<Span> {
        spans
    }
}

prop_compose! {
    fn arb_contained_spans()(
        outer in arb_span(),
        inner in arb_span(),
    ) -> (Span, Span, Span) {
        let mid_start_line = outer.start_line.max(inner.start_line);
        let mid_start_col = if mid_start_line == outer.start_line {
            outer.start_col.max(inner.start_col)
        } else { inner.start_col };
        let mid_end_line = outer.end_line.min(inner.end_line);
        let mid_end_col = if mid_end_line == outer.end_line {
            outer.end_col.min(inner.end_col)
        } else { inner.end_col };
        let mid_start_line = mid_start_line.max(1);
        let mid_start_col = mid_start_col.max(1);
        let mid_end_line = mid_end_line.max(mid_start_line);
        let mid_end_col = if mid_end_line == mid_start_line {
            mid_end_col.max(mid_start_col)
        } else { mid_end_col.max(1) };
        let outermost = Span {
            start_line: 1, start_col: 1,
            end_line: outer.end_line.max(mid_end_line).max(inner.end_line).max(1),
            end_col: {
                let max_line = outer.end_line.max(mid_end_line).max(inner.end_line);
                if max_line == outer.end_line { outer.end_col.max(mid_end_col).max(inner.end_col) }
                else if max_line == mid_end_line { mid_end_col.max(outer.end_col).max(inner.end_col) }
                else { inner.end_col.max(outer.end_col).max(mid_end_col) }
            },
        };
        let mid = Span { start_line: mid_start_line, start_col: mid_start_col, end_line: mid_end_line, end_col: mid_end_col };
        let innermost = Span {
            start_line: mid_start_line.max(inner.start_line),
            start_col: if mid_start_line.max(inner.start_line) == mid_start_line {
                mid_start_col.max(inner.start_col)
            } else { inner.start_col },
            end_line: mid_end_line.min(inner.end_line),
            end_col: if mid_end_line.min(inner.end_line) == mid_end_line {
                mid_end_col.min(inner.end_col)
            } else { inner.end_col },
        };
        (outermost, mid, innermost)
    }
}

proptest! {
    #[test]
    fn non_overlap_after_merge(spans in arb_span_set()) {
        let merged = Span::merge(&spans);
        for i in 0..merged.len() {
            for j in (i + 1)..merged.len() {
                let a = &merged[i];
                let b = &merged[j];
                prop_assert!(
                    a.end_line < b.start_line
                        || (a.end_line == b.start_line && a.end_col < b.start_col),
                    "merged spans overlap: {:?} and {:?}", a, b
                );
            }
        }
    }

    #[test]
    fn containment_transitive(triple in arb_contained_spans()) {
        let outer = triple.0;
        let mid = triple.1;
        let inner = triple.2;
        let outer_contains_mid = outer.contains(&mid);
        let mid_contains_inner = mid.contains(&inner);
        let outer_contains_inner = outer.contains(&inner);
        prop_assert!(outer_contains_mid, "outer={:?} should contain mid={:?}", outer, mid);
        prop_assert!(mid_contains_inner, "mid={:?} should contain inner={:?}", mid, inner);
        prop_assert!(outer_contains_inner, "outer={:?} should contain inner={:?}", outer, inner);
    }

    #[test]
    fn empty_spans_rejected(start in 2usize..=100) {
        prop_assert!(Span::new(start, 1, 1, 1).is_none());
    }

    #[test]
    fn zero_positions_rejected(
        sl in prop_oneof![1usize..=10, 0usize..=0usize],
        sc in prop_oneof![1usize..=10, 0usize..=0usize],
        el in prop_oneof![1usize..=10, 0usize..=0usize],
        ec in prop_oneof![1usize..=10, 0usize..=0usize],
    ) {
        let has_zero = sl == 0 || sc == 0 || el == 0 || ec == 0;
        if has_zero {
            prop_assert!(Span::new(sl, sc, el, ec).is_none());
        }
    }

    #[test]
    fn merge_preserves_coverage(spans in arb_span_set()) {
        let merged = Span::merge(&spans);

        // Every input span is fully contained within the merged result
        for span in &spans {
            let covered = merged.iter().any(|m| m.contains(span));
            prop_assert!(covered, "input span {:?} not covered by merged {:?}", span, merged);
        }

        // Every merged span boundary comes from some input span boundary
        for m in &merged {
            let start_from_input = spans.iter().any(|s| {
                s.start_line == m.start_line && s.start_col == m.start_col
            });
            let end_from_input = spans.iter().any(|s| {
                s.end_line == m.end_line && s.end_col == m.end_col
            });
            prop_assert!(start_from_input || end_from_input,
                "merged span {:?} bounds not from any input {:?}", m, spans);
        }
    }

    #[test]
    fn single_span_identity(span in arb_span()) {
        let merged = Span::merge(&[span]);
        prop_assert_eq!(merged.len(), 1);
        prop_assert_eq!(merged[0], span);
    }
}
