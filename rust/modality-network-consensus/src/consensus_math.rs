pub fn calculate_2f_plus_1(n: f64) -> u64 {
  ((n * 2.0) / 3.0).floor() as u64 + 1
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_calculate_2f_plus_1() {
      assert_eq!(calculate_2f_plus_1(3.0), 3);
      assert_eq!(calculate_2f_plus_1(6.0), 5);
      assert_eq!(calculate_2f_plus_1(9.0), 7);
  }
}