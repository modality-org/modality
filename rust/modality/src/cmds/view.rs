use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MERMAID_JS: &str = "https://cdn.jsdelivr.net/npm/mermaid@11.9.0/dist/mermaid.min.js";

/// Open a Mermaid rendering of a model in the default web browser
#[derive(Parser, Debug)]
pub struct Opts {
    /// Path to the .modality file
    pub input: String,

    /// Name of the model to view (optional, defaults to first model)
    #[arg(short, long)]
    pub model: Option<String>,

    /// Write the HTML file without opening a browser
    #[arg(long)]
    pub no_open: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let source = std::fs::read_to_string(&opts.input)
        .with_context(|| format!("Failed to read {}", opts.input))?;
    let model = crate::cmds::mermaid::load_named_model(&opts.input, opts.model.as_deref())?;
    let mermaid = modality_lang::generate_mermaid_diagram(&model);
    let html = render_html(&model.name, &opts.input, &source, &mermaid);
    let path = temp_html_path(&model.name)?;
    std::fs::write(&path, html)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    if opts.no_open {
        println!("Wrote {}", path.display());
        return Ok(());
    }

    open_in_default_browser(&path)?;
    println!("Opened {} in the default browser", path.display());
    Ok(())
}

fn temp_html_path(model_name: &str) -> Result<PathBuf> {
    let safe: String = model_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Ok(std::env::temp_dir().join(format!("modality-view-{safe}-{nanos}.html")))
}

fn open_in_default_browser(path: &Path) -> Result<()> {
    let status = {
        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg(path).status()
        }
        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open").arg(path).status()
        }
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/C", "start", ""])
                .arg(path)
                .status()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            return Err(anyhow!(
                "Opening a browser is not supported on this platform; HTML is at {}",
                path.display()
            ));
        }
    }
    .with_context(|| format!("Failed to open {}", path.display()))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "Failed to open {} in the default browser (exit {status})",
            path.display()
        ))
    }
}

fn render_html(title: &str, source_path: &str, modality: &str, mermaid: &str) -> String {
    include_str!("view.html")
        .replace("{{TITLE}}", &html_escape(title))
        .replace("{{SOURCE_PATH}}", &html_escape(source_path))
        .replace("{{MERMAID_JS}}", MERMAID_JS)
        .replace("{{MODALITY}}", &html_escape(modality))
        .replace("{{MERMAID}}", &html_escape(mermaid))
}

fn html_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_includes_renderer_and_witness_labels() {
        let modality = "model Contract {\n  part flow {\n    q0 --> q1: +POST\n  }\n}\n";
        let mermaid = "stateDiagram-v2\n    q0 --> q1 : +POST\n    q1 --> q1 : \"+POST +signed_by(/parties/alice.id)\"";
        let html = render_html("Contract", "model/default.modality", modality, mermaid);
        assert!(html.contains(MERMAID_JS));
        assert!(html.contains("Contract"));
        assert!(html.contains("model/default.modality"));
        assert!(html.contains("stateDiagram-v2"));
        assert!(html.contains("q0 --&gt; q1 : +POST"));
        assert!(html.contains("q0 --&gt; q1: +POST"));
        assert!(!html.contains("+MODEL"));
        assert!(html.contains("part flow"));
        assert!(html.contains("+signed_by(/parties/alice.id)"));
        assert!(html.contains("mermaid.initialize"));
    }

    #[test]
    fn html_escape_prevents_markup_breakout() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }
}
