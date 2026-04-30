use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub file_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub col_start: Option<u32>,
    pub col_end: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByteSpan {
    pub file_path: String,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn to_byte_span(&self, source: &str) -> ByteSpan {
        let mut line_offsets: Vec<usize> = vec![0];
        for (i, ch) in source.char_indices() {
            if ch == '\n' {
                line_offsets.push(i + 1);
            }
        }
        line_offsets.push(source.len());

        let start =
            line_offsets.get(self.line_start.saturating_sub(1) as usize).copied().unwrap_or(0)
                + self.col_start.map(|c| c.saturating_sub(1) as usize).unwrap_or(0);

        let end = line_offsets
            .get(self.line_end.saturating_sub(1) as usize)
            .copied()
            .unwrap_or(source.len())
            + self.col_end.map(|c| c.saturating_sub(1) as usize).unwrap_or(0);

        ByteSpan { file_path: self.file_path.clone(), start, end }
    }
}
