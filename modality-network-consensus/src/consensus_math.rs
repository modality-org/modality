pub fn calculate_2f_plus_1(n: f64) -> i64 {
  ((n * 2.0) / 3.0).floor() as i64 + 1
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