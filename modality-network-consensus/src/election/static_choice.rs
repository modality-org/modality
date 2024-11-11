use anyhow::Result;

#[derive(Clone)]
pub struct StaticChoice {
  index: usize,
}

impl StaticChoice {
  pub fn new(index: Option<usize>) -> Self {
      StaticChoice {
          index: index.unwrap_or(0)
      }
  }

  pub async fn pick_one<'a, T>(&'a self, options: &'a [T]) -> Result<&'a T> {
      Ok(&options[self.index % options.len()])
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_static_choice() {
      let choices = vec![1, 2, 3, 4, 5];
      
      let picker = StaticChoice::new(Some(0));
      assert_eq!(*picker.pick_one(&choices).await.unwrap(), 1);

      let picker = StaticChoice::new(Some(2));
      assert_eq!(*picker.pick_one(&choices).await.unwrap(), 3);

      let picker = StaticChoice::new(Some(5));
      assert_eq!(*picker.pick_one(&choices).await.unwrap(), 1);  // 5 % 5 = 0, so first element
  }
}