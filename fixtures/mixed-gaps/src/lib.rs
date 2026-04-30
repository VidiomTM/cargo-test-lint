pub fn process(items: &[i32]) -> i32 {
    let sum: i32 = items.iter().filter(|&&x| x > 0).sum();
    if sum > 100 { sum } else { sum * 2 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_process() { assert_eq!(process(&[1, 2]), 6); }
}
