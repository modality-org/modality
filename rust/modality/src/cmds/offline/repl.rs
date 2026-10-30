//! `modal repl model.modality`: step through a run by hand.
//!
//! ```text
//! :set /in/c.text=0 /in/n.integer=3   post inputs (written by the next move that may)
//! :live                               the moves possible now
//! :step [to]                          take the first possible move (to that node)
//! :run                                take moves until none is possible
//! :state                              every value
//! :reset                              back to the start, inputs kept
//! :quit
//! ```

use super::fmt::Names;
use super::project;
use super::run::parse_set;
use anyhow::Result;
use clap::Parser;
use modality_lang::data::machine::Outcome;
use modality_lang::data::{State, Value, Writes};
use std::io::{BufRead, Write};

#[derive(Parser, Debug, Clone)]
pub struct Opts {
    /// The model file
    pub model: String,
}

struct Session {
    node: String,
    state: State,
    pending: Vec<(String, Value)>,
    inputs: Vec<(String, Value)>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let p = project::load(std::slice::from_ref(&opts.model))?;
    let m = &p.machine;
    let names = Names::new(p.paths());
    let start = m.start.first().cloned().unwrap_or_default();
    let mut s = Session {
        node: start.clone(),
        state: State::new(),
        pending: Vec::new(),
        inputs: Vec::new(),
    };
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    println!("model {} at {start}; :help lists commands", m.name);
    loop {
        print!("> ");
        out.flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(cmd) = words.first() else { continue };
        let moves = |s: &Session| {
            let mut found = Vec::new();
            for e in m.edges_from(&s.node) {
                let extra: Writes = s.pending.iter().filter(|(p, _)| e.leaves_open(p)).cloned().collect();
                if let Outcome::Moved { writes, after } = m.try_move(e, &s.state, &extra) {
                    found.push((e.index, writes, after));
                }
            }
            found
        };
        let show = |w: &Writes, e: usize| -> String {
            m.edges[e]
                .write_order(w)
                .iter()
                .map(|k| format!("{} = {}", names.short(k), w[k]))
                .collect::<Vec<_>>()
                .join("  ")
        };
        match *cmd {
            ":set" => {
                for w in &words[1..] {
                    match parse_set(w) {
                        Ok(kv) => {
                            s.pending.retain(|(p, _)| *p != kv.0);
                            s.inputs.retain(|(p, _)| *p != kv.0);
                            s.pending.push(kv.clone());
                            s.inputs.push(kv);
                        }
                        Err(e) => println!("{e}"),
                    }
                }
            }
            ":live" => {
                let ms = moves(&s);
                if ms.is_empty() {
                    println!("no move from {} is possible", s.node);
                }
                for (e, _, _) in ms {
                    let edge = &m.edges[e];
                    let ls: Vec<String> = edge.labels.iter().map(|l| l.text()).collect();
                    println!("{} [{}]", edge.name(), ls.join(" "));
                }
            }
            ":step" | ":run" => loop {
                let ms = moves(&s);
                let pick = match words.get(1).filter(|_| *cmd == ":step") {
                    Some(to) => ms.into_iter().find(|(e, _, _)| m.edges[*e].to == *to),
                    None => ms.into_iter().next(),
                };
                let Some((e, writes, after)) = pick else {
                    println!("no move from {} is possible", s.node);
                    break;
                };
                println!("{}   {}", m.edges[e].name(), show(&writes, e));
                s.pending.retain(|(p, _)| !writes.contains_key(p));
                s.node = m.edges[e].to.clone();
                s.state = after;
                if *cmd == ":step" {
                    break;
                }
            },
            ":state" => {
                println!("at {}", s.node);
                for (k, v) in &s.state {
                    println!("  {k} = {v}");
                }
            }
            ":reset" => {
                s.node = start.clone();
                s.state = State::new();
                s.pending = s.inputs.clone();
                println!("at {start}");
            }
            ":quit" | ":q" => break,
            _ => println!(":set PATH=VALUE …  :live  :step [node]  :run  :state  :reset  :quit"),
        }
    }
    Ok(())
}
