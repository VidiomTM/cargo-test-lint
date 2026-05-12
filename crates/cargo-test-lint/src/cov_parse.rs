use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageLine {
    pub file_path: String,
    pub line_number: u64,
    pub execution_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageData {
    pub lines: Vec<CoverageLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    EmptyInput,
    InvalidLine { line_num: usize, reason: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::EmptyInput => write!(f, "empty input"),
            ParseError::InvalidLine { line_num, reason } => {
                write!(f, "invalid line {line_num}: {reason}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub fn serialize(data: &CoverageData) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(data.lines.len() * 32);
    for line in &data.lines {
        let _ = writeln!(out, "{}:{}:{}", line.file_path, line.line_number, line.execution_count);
    }
    out
}

pub fn parse(text: &str) -> Result<CoverageData, ParseError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let mut lines = Vec::new();
    for (i, raw) in trimmed.lines().enumerate() {
        let entry = raw.trim();
        if entry.is_empty() {
            continue;
        }
        let parts: Vec<&str> = entry.rsplitn(3, ':').collect();
        if parts.len() != 3 {
            return Err(ParseError::InvalidLine {
                line_num: i + 1,
                reason: "expected format file_path:line_number:execution_count".into(),
            });
        }

        let execution_count: u64 = parts[0].parse().map_err(|_| ParseError::InvalidLine {
            line_num: i + 1,
            reason: format!("invalid execution count: {}", parts[0]),
        })?;

        let line_number: u64 = parts[1].parse().map_err(|_| ParseError::InvalidLine {
            line_num: i + 1,
            reason: format!("invalid line number: {}", parts[1]),
        })?;
        if line_number == 0 {
            return Err(ParseError::InvalidLine {
                line_num: i + 1,
                reason: "line number must be >= 1".into(),
            });
        }

        let file_path = parts[2].to_string();
        if file_path.is_empty() {
            return Err(ParseError::InvalidLine {
                line_num: i + 1,
                reason: "file path is empty".into(),
            });
        }

        lines.push(CoverageLine { file_path, line_number, execution_count });
    }

    Ok(CoverageData { lines })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_single_line() {
        let data = CoverageData {
            lines: vec![CoverageLine {
                file_path: "src/main.rs".into(),
                line_number: 10,
                execution_count: 5,
            }],
        };
        let serialized = serialize(&data);
        let parsed = parse(&serialized).unwrap();
        assert_eq!(data, parsed, "single-line coverage data roundtrips correctly");
    }

    #[test]
    fn roundtrip_multiple_lines() {
        let data = CoverageData {
            lines: vec![
                CoverageLine { file_path: "src/lib.rs".into(), line_number: 1, execution_count: 0 },
                CoverageLine {
                    file_path: "src/lib.rs".into(),
                    line_number: 42,
                    execution_count: 100,
                },
            ],
        };
        let serialized = serialize(&data);
        let parsed = parse(&serialized).unwrap();
        assert_eq!(data, parsed, "multi-line coverage data roundtrips correctly");
    }

    #[test]
    fn reject_empty_input() {
        assert!(parse("").is_err(), "empty input should be rejected");
        assert!(parse("   ").is_err(), "whitespace-only input should be rejected");
    }

    #[test]
    fn reject_zero_line_number() {
        let text = "src/main.rs:0:1\n";
        assert!(parse(text).is_err(), "zero line number should be rejected");
    }

    #[test]
    fn reject_empty_file_path() {
        let text = ":10:1\n";
        assert!(parse(text).is_err(), "empty file path should be rejected");
    }

    #[test]
    fn reject_malformed_line() {
        let text = "garbage\n";
        assert!(parse(text).is_err(), "malformed input should be rejected");
    }
}
