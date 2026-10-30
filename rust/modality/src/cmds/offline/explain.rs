//! `modal explain <code>`: what a diagnostic means, and what to do.

use anyhow::{anyhow, Result};
use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct Opts {
    /// A diagnostic code, as `modal check` prints it (check/box-violated)
    pub code: Option<String>,
}

const CODES: &[(&str, &str)] = &[
    (
        "check/box-violated",
        "A box `[L] φ` is a promise about every move that carries the labels L.
`always([L] false)` says no such move may exist. The check found an edge
that a reachable path can take, where some commit could carry L.

Fix it on the edge, or in the rule:
  - add the labels the box forbids the lack of (for `[-signed_by(/a.id)]
    false`, add `+signed_by(/a.id)` to the edge), or
  - promise to leave a path alone (`-modifies(/p)`), so the edge cannot
    write it, or
  - remove the edge.

The path printed is the shortest from a start node to that edge.",
    ),
    (
        "check/diamond-unreachable",
        "A diamond `<L> φ` promises that some move carrying L exists. A rule that
asks for one is met only by an edge whose labels say L: an edge that
merely allows L does not count, because a commit on it may not carry L.
The check found no reachable edge that says L.

Add an edge that carries the labels, or drop the formula. `eventually`
means a path can reach it; no rule can make someone act.",
    ),
    (
        "lint/vacuous-box-guard",
        "`[L] true` holds everywhere: a box of `true` is true whether or not any
move carries L. To say such a move is possible, write `<L> true`; to say
it must be possible and taken, write `[<L>] true`.",
    ),
    (
        "domain/type",
        "A value in the search domain does not fit its path's type: a `.natural`
holds whole numbers from 0, an `.integer` whole numbers, a `.rational` any
exact fraction, a `.bool` true or false. The search stops before it runs,
because no move could ever post that value. Remove it, or change the
path's type.",
    ),
];

pub async fn run(opts: &Opts) -> Result<()> {
    match opts.code.as_deref() {
        None => {
            for (code, text) in CODES {
                println!("{code}\n  {}\n", text.lines().next().unwrap_or(""));
            }
            Ok(())
        }
        Some(code) => {
            let (_, text) = CODES
                .iter()
                .find(|(c, _)| *c == code)
                .ok_or_else(|| anyhow!("no diagnostic {code}; `modal explain` lists them"))?;
            println!("{code}\n\n{text}");
            Ok(())
        }
    }
}
