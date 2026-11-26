//! undertop: a system monitor dressed as the operations console of an underground silo.

mod app;
mod config;
mod gpu;
mod sysmon;
mod theme;
mod ui;

use std::path::PathBuf;

use color_eyre::Result;
use color_eyre::eyre::eyre;

use config::Settings;

const USAGE: &str = "\
undertop - subterranean operations terminal

USAGE: undertop [OPTIONS]

OPTIONS:
  --no-boot          skip the power-on self test
  --theme <name>     amber, phosphor, cobalt or lockdown
  --config <path>    read this config file instead of ~/.config/undertop/config.toml
  --print-config     print a config template with the defaults, then exit
  -h, --help         this text

Press ? inside the program for the operator manual.";

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    if args.iter().any(|a| a == "--print-config") {
        print!("{}", config::TEMPLATE);
        return Ok(());
    }
    let value = |flag: &str| -> Result<Option<&String>> {
        match args.iter().position(|a| a == flag) {
            None => Ok(None),
            Some(i) => args
                .get(i + 1)
                .map(Some)
                .ok_or_else(|| eyre!("{flag} needs a value\n\n{USAGE}")),
        }
    };
    if let Some(bad) = args.iter().enumerate().find(|(i, a)| {
        a.starts_with('-')
            && !["--no-boot", "--theme", "--config"].contains(&a.as_str())
            && !(*i > 0 && ["--theme", "--config"].contains(&args[i - 1].as_str()))
    }) {
        return Err(eyre!("unknown option {}\n\n{USAGE}", bad.1));
    }

    let mut settings = match value("--config")? {
        Some(p) => Settings::load(&PathBuf::from(p), true)?,
        None => match config::default_path() {
            Some(p) => Settings::load(&p, false)?,
            None => Settings::default(),
        },
    };
    if args.iter().any(|a| a == "--no-boot") {
        settings.boot = false;
    }
    if let Some(name) = value("--theme")? {
        settings.theme_idx =
            config::theme_index(name).ok_or_else(|| eyre!("unknown theme: {name}\n\n{USAGE}"))?;
    }

    let mut app = app::App::new(settings)?;
    ratatui::run(|terminal| app.run(terminal))
}
