use std::collections::HashMap;

use anyhow::Context;
use ctl_core::coverage::{CoverageFile, CoverageGap, CoverageReport, CoverageSummary};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LlvmCovExport {
    data: Vec<LlvmCovData>,
}

#[derive(Debug, Deserialize)]
struct LlvmCovData {
    files: Vec<LlvmCovFileEntry>,
}

#[derive(Debug, Deserialize)]
struct LlvmCovFileEntry {
    filename: String,
    #[serde(deserialize_with = "deserialize_segments")]
    segments: Vec<LlvmCovSegment>,
    summary: Option<LlvmCovFileSummary>,
}

#[derive(Debug)]
struct LlvmCovSegment {
    line: u64,
    col: u64,
    count: u64,
    has_count: bool,
    is_region_entry: bool,
    is_gap: bool,
}

fn deserialize_segments<'de, D>(deserializer: D) -> Result<Vec<LlvmCovSegment>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{SeqAccess, Visitor};

    struct SegmentVisitor;

    impl<'de> Visitor<'de> for SegmentVisitor {
        type Value = Vec<LlvmCovSegment>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a sequence of segment arrays")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut segments = Vec::new();
            while let Some(arr) = seq.next_element::<serde_json::Value>()? {
                let arr = arr
                    .as_array()
                    .ok_or_else(|| serde::de::Error::custom("segment must be an array"))?;
                let line = arr.first().and_then(|v| v.as_u64()).ok_or_else(|| {
                    serde::de::Error::custom("segment[0] (line) missing or not u64")
                })?;
                let col = arr.get(1).and_then(|v| v.as_u64()).ok_or_else(|| {
                    serde::de::Error::custom("segment[1] (col) missing or not u64")
                })?;
                let count = arr.get(2).and_then(|v| v.as_u64()).ok_or_else(|| {
                    serde::de::Error::custom("segment[2] (count) missing or not u64")
                })?;
                let has_count = arr.get(3).and_then(|v| v.as_bool()).ok_or_else(|| {
                    serde::de::Error::custom("segment[3] (has_count) missing or not bool")
                })?;
                let is_region_entry = arr.get(4).and_then(|v| v.as_bool()).unwrap_or(false);
                let is_gap = arr.get(5).and_then(|v| v.as_bool()).unwrap_or(false);
                segments.push(LlvmCovSegment {
                    line,
                    col,
                    count,
                    has_count,
                    is_region_entry,
                    is_gap,
                });
            }
            Ok(segments)
        }
    }

    deserializer.deserialize_seq(SegmentVisitor)
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LlvmCovFileSummary {
    lines: Option<LlvmCovCount>,
    regions: Option<LlvmCovCount>,
    branches: Option<LlvmCovCount>,
    functions: Option<LlvmCovCount>,
    instantiations: Option<LlvmCovCount>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LlvmCovCount {
    count: u64,
    covered: u64,
    notcovered: Option<u64>,
    percent: f64,
}

pub fn parse_llvm_cov_json(raw: &str) -> anyhow::Result<CoverageReport> {
    let export: LlvmCovExport = serde_json::from_str(raw).context("malformed llvm-cov JSON")?;

    let mut file_map: HashMap<String, FileAccum> = HashMap::new();

    for data_entry in &export.data {
        for file_entry in &data_entry.files {
            let accum = file_map.entry(file_entry.filename.clone()).or_insert_with(|| FileAccum {
                path: file_entry.filename.clone(),
                lines: 0,
                covered: 0,
                not_covered: 0,
                summary_percent: 0.0,
            });

            if let Some(ref summary) = file_entry.summary {
                if let Some(ref lines) = summary.lines {
                    accum.lines += lines.count;
                    accum.covered += lines.covered;
                    accum.not_covered += lines
                        .notcovered
                        .unwrap_or_else(|| lines.count.saturating_sub(lines.covered));
                    if lines.count > 0 {
                        accum.summary_percent = (accum.covered as f64 / accum.lines as f64) * 100.0;
                    }
                }
            }
        }
    }

    let files: Vec<CoverageFile> = file_map
        .into_values()
        .map(|a| CoverageFile {
            path: a.path,
            summary: CoverageSummary {
                lines: a.lines,
                covered: a.covered,
                not_covered: a.not_covered,
                percent: a.summary_percent,
            },
        })
        .collect();

    Ok(CoverageReport { generated_at: chrono_now_rfc3339(), files })
}

pub fn extract_gaps(raw: &str) -> anyhow::Result<Vec<CoverageGap>> {
    let export: LlvmCovExport = serde_json::from_str(raw).context("malformed llvm-cov JSON")?;

    let mut gaps = Vec::new();

    for data_entry in &export.data {
        for file_entry in &data_entry.files {
            for seg in &file_entry.segments {
                if seg.has_count && seg.count == 0 && !seg.is_gap {
                    gaps.push(CoverageGap {
                        file_path: file_entry.filename.clone(),
                        line: seg.line as u32,
                        column_start: Some(seg.col as u32),
                        column_end: None,
                        count: seg.count,
                        is_branch: seg.is_region_entry,
                    });
                }
            }
        }
    }

    Ok(gaps)
}

struct FileAccum {
    path: String,
    lines: u64,
    covered: u64,
    not_covered: u64,
    summary_percent: f64,
}

fn chrono_now_rfc3339() -> String {
    format!("{}Z", chrono_free_rfc3339())
}

fn chrono_free_rfc3339() -> String {
    let dur =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;

    let (y, m, d) = days_to_ymd(days_since_epoch);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", y, m, d, hour, minute, second)
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    loop {
        let dy = if is_leap(y) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days: [u64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        m += 1;
    }
    (y, m + 1, days + 1)
}

#[inline]
fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
