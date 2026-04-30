use ctl_core::span::{Span, byte_offset};

#[test]
fn byte_offset_first_line() {
    let source = "hello\nworld\n";
    assert_eq!(byte_offset(source, 1, None), 0);
}

#[test]
fn byte_offset_second_line() {
    let source = "hello\nworld\n";
    assert_eq!(byte_offset(source, 2, None), 6);
}

#[test]
fn byte_offset_with_column() {
    let source = "hello\nworld\n";
    assert_eq!(byte_offset(source, 2, Some(2)), 7);
}

#[test]
fn byte_offset_third_line() {
    let source = "line1\nline2\nline3\n";
    assert_eq!(byte_offset(source, 3, None), 12);
}

#[test]
fn byte_offset_out_of_range_returns_zero() {
    let source = "hello";
    assert_eq!(byte_offset(source, 100, None), 0);
}

#[test]
fn span_to_byte_span_basic() {
    let source = "hello\nworld\n";
    let span = Span {
        file_path: "test.rs".into(),
        line_start: 1,
        line_end: 1,
        col_start: Some(1),
        col_end: Some(5),
    };
    let byte_span = span.to_byte_span(source);
    assert_eq!(byte_span.start, 0);
    assert_eq!(byte_span.end, 4);
}

#[test]
fn span_to_byte_span_multiline() {
    let source = "abc\ndef\nghi\n";
    let span = Span {
        file_path: "test.rs".into(),
        line_start: 1,
        line_end: 2,
        col_start: Some(1),
        col_end: Some(3),
    };
    let byte_span = span.to_byte_span(source);
    assert_eq!(byte_span.start, 0);
    assert_eq!(byte_span.end, 6);
}
