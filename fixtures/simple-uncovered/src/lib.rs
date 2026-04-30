pub fn classify(n: i32) -> &'static str {
    if n > 0 { "positive" } else { "non-positive" }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_positive() { assert_eq!(classify(1), "positive"); }
}
