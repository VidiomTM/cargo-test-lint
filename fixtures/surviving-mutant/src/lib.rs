pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_nonzero() {
        assert_ne!(add(0, 0), 0);
    }
}
