use anyhow::{Result, Context};
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Modify node configuration")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,
    
    /// Node directory containing config.json
    #[clap(long)]
    pub dir: Option<PathBuf>,
    
    /// Set listener addresses (comma-separated, e.g. "/ip4/0.0.0.0/tcp/4040/ws,/ip4/127.0.0.1/tcp/5050/ws")
    #[clap(long)]
    pub set_listeners: Option<String>,
    
    /// Add listener address (e.g. "/ip4/127.0.0.1/tcp/5050/ws")
    #[clap(long)]
    pub add_listener: Option<String>,
    
    /// Remove listener address (e.g. "/ip4/0.0.0.0/tcp/4040/ws")
    #[clap(long)]
    pub remove_listener: Option<String>,
    
    /// Set bootstrapper addresses (comma-separated multiaddrs)
    #[clap(long)]
    pub set_bootstrappers: Option<String>,
    
    /// Add bootstrapper address
    #[clap(long)]
    pub add_bootstrapper: Option<String>,
    
    /// Remove bootstrapper address
    #[clap(long)]
    pub remove_bootstrapper: Option<String>,
    
    /// Replace IP address in listeners and bootstrappers (e.g. "0.0.0.0=127.0.0.1")
    #[clap(long)]
    pub replace_ip: Option<String>,
    
    /// Enable autoupgrade
    #[clap(long)]
    pub enable_autoupgrade: bool,
    
    /// Disable autoupgrade
    #[clap(long)]
    pub disable_autoupgrade: bool,
    
    /// Show current configuration
    #[clap(long)]
    pub show: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    // Determine config file path
    let config_path = if let Some(ref path) = opts.config {
        path.clone()
    } else if let Some(ref d) = dir {
        d.join("config.json")
    } else {
        std::env::current_dir()?.join("config.json")
    };
    
    // Check if config file exists
    if !config_path.exists() {
        anyhow::bail!("Config file not found: {}", config_path.display());
    }
    
    // Load current config as JSON
    let config_content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config from {}", config_path.display()))?;
    
    let mut config: serde_json::Value = serde_json::from_str(&config_content)
        .with_context(|| format!("Failed to parse config from {}", config_path.display()))?;
    
    // If just showing config
    if opts.show {
        println!("📋 Current Configuration: {}", config_path.display());
        println!();
        
        if let Some(listeners) = config.get("listeners") {
            println!("Listeners:");
            if let Some(arr) = listeners.as_array() {
                for listener in arr {
                    println!("  • {}", listener.as_str().unwrap_or(""));
                }
            }
        } else {
            println!("Listeners: (not set)");
        }
        
        println!();
        
        if let Some(bootstrappers) = config.get("bootstrappers") {
            println!("Bootstrappers:");
            if let Some(arr) = bootstrappers.as_array() {
                for bootstrapper in arr {
                    println!("  • {}", bootstrapper.as_str().unwrap_or(""));
                }
            }
        } else {
            println!("Bootstrappers: (not set)");
        }
        
        println!();
        
        if let Some(autoupgrade) = config.get("autoupgrade_enabled") {
            println!("Autoupgrade: {}", 
                if autoupgrade.as_bool().unwrap_or(false) { "✓ Enabled" } else { "✗ Disabled" }
            );
        } else {
            println!("Autoupgrade: (not set)");
        }
        
        return Ok(());
    }
    
    let mut modified = false;
    
    // Handle set_listeners
    if let Some(ref listeners_str) = opts.set_listeners {
        let listeners: Vec<String> = listeners_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        config["listeners"] = serde_json::json!(listeners);
        modified = true;
        println!("✓ Set {} listener(s)", listeners.len());
    }
    
    // Handle add_listener
    if let Some(ref listener) = opts.add_listener {
        let listeners = config.get_mut("listeners")
            .and_then(|v| v.as_array_mut())
            .context("listeners field is not an array")?;
        
        let listener_value = serde_json::Value::String(listener.clone());
        if !listeners.contains(&listener_value) {
            listeners.push(listener_value);
            modified = true;
            println!("✓ Added listener: {}", listener);
        } else {
            println!("⚠️  Listener already exists: {}", listener);
        }
    }
    
    // Handle remove_listener
    if let Some(ref listener) = opts.remove_listener {
        let listeners = config.get_mut("listeners")
            .and_then(|v| v.as_array_mut())
            .context("listeners field is not an array")?;
        
        let listener_value = serde_json::Value::String(listener.clone());
        if let Some(pos) = listeners.iter().position(|x| x == &listener_value) {
            listeners.remove(pos);
            modified = true;
            println!("✓ Removed listener: {}", listener);
        } else {
            println!("⚠️  Listener not found: {}", listener);
        }
    }
    
    // Handle set_bootstrappers
    if let Some(ref bootstrappers_str) = opts.set_bootstrappers {
        let bootstrappers: Vec<String> = bootstrappers_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        config["bootstrappers"] = serde_json::json!(bootstrappers);
        modified = true;
        println!("✓ Set {} bootstrapper(s)", bootstrappers.len());
    }
    
    // Handle add_bootstrapper
    if let Some(ref bootstrapper) = opts.add_bootstrapper {
        let bootstrappers = config.get_mut("bootstrappers")
            .and_then(|v| v.as_array_mut())
            .context("bootstrappers field is not an array")?;
        
        let bootstrapper_value = serde_json::Value::String(bootstrapper.clone());
        if !bootstrappers.contains(&bootstrapper_value) {
            bootstrappers.push(bootstrapper_value);
            modified = true;
            println!("✓ Added bootstrapper: {}", bootstrapper);
        } else {
            println!("⚠️  Bootstrapper already exists: {}", bootstrapper);
        }
    }
    
    // Handle remove_bootstrapper
    if let Some(ref bootstrapper) = opts.remove_bootstrapper {
        let bootstrappers = config.get_mut("bootstrappers")
            .and_then(|v| v.as_array_mut())
            .context("bootstrappers field is not an array")?;
        
        let bootstrapper_value = serde_json::Value::String(bootstrapper.clone());
        if let Some(pos) = bootstrappers.iter().position(|x| x == &bootstrapper_value) {
            bootstrappers.remove(pos);
            modified = true;
            println!("✓ Removed bootstrapper: {}", bootstrapper);
        } else {
            println!("⚠️  Bootstrapper not found: {}", bootstrapper);
        }
    }
    
    // Handle replace_ip
    if let Some(ref replace_str) = opts.replace_ip {
        let parts: Vec<&str> = replace_str.split('=').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid replace_ip format. Expected: OLD_IP=NEW_IP (e.g. 0.0.0.0=127.0.0.1)");
        }
        
        let old_ip = parts[0].trim();
        let new_ip = parts[1].trim();
        
        // Replace in listeners
        if let Some(listeners) = config.get_mut("listeners").and_then(|v| v.as_array_mut()) {
            for listener in listeners.iter_mut() {
                if let Some(s) = listener.as_str() {
                    let new_value = s.replace(old_ip, new_ip);
                    if new_value != s {
                        *listener = serde_json::Value::String(new_value);
                        modified = true;
                    }
                }
            }
        }
        
        // Replace in bootstrappers
        if let Some(bootstrappers) = config.get_mut("bootstrappers").and_then(|v| v.as_array_mut()) {
            for bootstrapper in bootstrappers.iter_mut() {
                if let Some(s) = bootstrapper.as_str() {
                    let new_value = s.replace(old_ip, new_ip);
                    if new_value != s {
                        *bootstrapper = serde_json::Value::String(new_value);
                        modified = true;
                    }
                }
            }
        }
        
        if modified {
            println!("✓ Replaced {} with {} in listeners and bootstrappers", old_ip, new_ip);
        } else {
            println!("⚠️  No occurrences of {} found", old_ip);
        }
    }
    
    // Handle enable_autoupgrade
    if opts.enable_autoupgrade {
        config["autoupgrade_enabled"] = serde_json::json!(true);
        modified = true;
        println!("✓ Enabled autoupgrade");
    }
    
    // Handle disable_autoupgrade
    if opts.disable_autoupgrade {
        config["autoupgrade_enabled"] = serde_json::json!(false);
        modified = true;
        println!("✓ Disabled autoupgrade");
    }
    
    // Save config if modified
    if modified {
        let new_content = serde_json::to_string_pretty(&config)
            .context("Failed to serialize config")?;
        
        std::fs::write(&config_path, new_content)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;
        
        println!();
        println!("✅ Configuration saved to: {}", config_path.display());
    } else {
        println!("⚠️  No changes made to configuration");
    }
    
    Ok(())
}

