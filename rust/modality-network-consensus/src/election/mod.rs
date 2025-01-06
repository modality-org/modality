use anyhow::Result;

pub mod round_robin;
pub mod static_choice;

#[derive(Clone)]
pub enum Election {
    RoundRobin(round_robin::RoundRobin),
    StaticChoice(static_choice::StaticChoice),
}

impl Election {
  pub async fn pick_one<'a, T>(&'a self, options: &'a [T], input: &str) -> Result<&'a T> {
      match self {
          Election::RoundRobin(rr) => rr.pick_one(options, input).await,
          Election::StaticChoice(sc) => sc.pick_one(options).await,
      }
  }
}