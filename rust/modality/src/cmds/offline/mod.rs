//! Offline checking of models over data: `modal check`, `run`, `prove`,
//! `test`, `repl` and `explain`. Nothing here talks to a network.

pub mod check;
pub mod domain;
pub mod fmt;
pub mod project;
pub mod run;
pub mod certs;
pub mod houdini;
pub mod lean;
pub mod prove;
pub mod test;
pub mod explain;
pub mod repl;

/// Print `report()`, then again each time an input file changes, until
/// interrupted.
pub fn watch(inputs: &[String], report: impl Fn() -> anyhow::Result<String>) -> anyhow::Result<()> {
    let stamp = || -> Vec<Option<std::time::SystemTime>> {
        let mut files: Vec<std::path::PathBuf> = Vec::new();
        for i in inputs {
            let p = std::path::Path::new(i);
            if p.is_dir() {
                if let Ok(rd) = std::fs::read_dir(p) {
                    files.extend(rd.filter_map(|e| e.ok().map(|e| e.path())));
                }
            } else {
                files.push(p.to_path_buf());
            }
        }
        files.sort();
        files.iter().map(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok()).collect()
    };
    loop {
        match report() {
            Ok(out) => print!("{out}"),
            Err(e) => println!("error: {e}"),
        }
        println!("-- watching {} for changes (ctrl-c to stop)", inputs.join(", "));
        let before = stamp();
        while stamp() == before {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        println!();
    }
}
