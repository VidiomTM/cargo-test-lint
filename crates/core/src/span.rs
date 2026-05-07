use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Span {
    pub fn new(
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> Option<Self> {
        if start_line == 0 || start_col == 0 || end_line == 0 || end_col == 0 {
            return None;
        }
        if start_line > end_line || (start_line == end_line && start_col > end_col) {
            return None;
        }
        Some(Self { start_line, start_col, end_line, end_col })
    }

    pub fn contains(&self, other: &Span) -> bool {
        let start_le = self.start_line < other.start_line
            || (self.start_line == other.start_line && self.start_col <= other.start_col);
        let end_ge = self.end_line > other.end_line
            || (self.end_line == other.end_line && self.end_col >= other.end_col);
        start_le && end_ge
    }

    pub fn merge(spans: &[Span]) -> Vec<Span> {
        if spans.is_empty() {
            return vec![];
        }
        let mut sorted = spans.to_vec();
        sorted.sort_by(|a, b| a.start_line.cmp(&b.start_line).then(a.start_col.cmp(&b.start_col)));
        let mut result: Vec<Span> = vec![sorted[0]];
        for span in &sorted[1..] {
            let last = result.last_mut().unwrap();
            let touches = span.start_line < last.end_line
                || (span.start_line == last.end_line && span.start_col <= last.end_col);
            if touches {
                if span.end_line > last.end_line
                    || (span.end_line == last.end_line && span.end_col > last.end_col)
                {
                    last.end_line = span.end_line;
                    last.end_col = span.end_col;
                }
            } else {
                result.push(*span);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_zero_positions() {
        assert!(Span::new(0, 1, 1, 1).is_none(), "zero start_line should be rejected");
        assert!(Span::new(1, 0, 1, 1).is_none(), "zero start_col should be rejected");
        assert!(Span::new(1, 1, 0, 1).is_none(), "zero end_line should be rejected");
        assert!(Span::new(1, 1, 1, 0).is_none(), "zero end_col should be rejected");
    }

    #[test]
    fn new_rejects_inverted() {
        assert!(Span::new(5, 1, 1, 1).is_none(), "inverted line span should be rejected");
        assert!(Span::new(3, 5, 3, 3).is_none(), "inverted column span should be rejected");
    }

    #[test]
    fn new_accepts_valid() {
        assert!(Span::new(1, 1, 5, 10).is_some(), "normal span should be accepted");
        assert!(Span::new(3, 5, 3, 5).is_some(), "point span should be accepted");
        assert!(Span::new(3, 5, 3, 10).is_some(), "same-line span should be accepted");
    }

    #[test]
    fn contains_self() {
        let s = Span::new(1, 1, 5, 10).unwrap();
        assert!(s.contains(&s), "span should contain itself");
    }

    #[test]
    fn contains_sub_span() {
        let outer = Span::new(1, 1, 10, 20).unwrap();
        let inner = Span::new(2, 3, 8, 15).unwrap();
        assert!(outer.contains(&inner), "outer span should contain inner span");
        assert!(!inner.contains(&outer), "inner span should not contain outer span");
    }

    #[test]
    fn merge_empty_is_empty() {
        assert_eq!(Span::merge(&[]), vec![], "merging empty list should produce empty list");
    }

    #[test]
    fn merge_single_is_identity() {
        let s = Span::new(1, 1, 5, 10).unwrap();
        assert_eq!(Span::merge(&[s]), vec![s], "merging single span should return it unchanged");
    }

    #[test]
    fn merge_overlapping_spans() {
        let a = Span::new(1, 1, 5, 10).unwrap();
        let b = Span::new(3, 1, 8, 5).unwrap();
        let merged = Span::merge(&[a, b]);
        assert_eq!(merged.len(), 1, "overlapping spans should merge into 1");
        assert_eq!(merged[0].start_line, 1, "merged span should start at line 1");
        assert_eq!(merged[0].end_line, 8, "merged span should end at line 8");
    }
}
