//! undertop: a system monitor dressed as the operations console of an underground silo.

mod app;
mod config;
mod gpu;
mod shift;
mod sysmon;
mod theme;
mod ui;
mod voice;

use std::io::stdout;
use std::path::PathBuf;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;

use config::Settings;

const USAGE: &str = "\
undertop - subterranean operations terminal

USAGE: undertop [OPTIONS]

OPTIONS:
  --no-boot          skip the power-on self test
  --theme <name>     amber, phosphor, cobalt, lockdown or silo
  --panels <list>    stations to show, e.g. cpu,memory,procs
                     (cpu, memory, thermal, comms, vaults, gpu, procs)
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
            && !["--no-boot", "--theme", "--config", "--panels"].contains(&a.as_str())
            && !(*i > 0 && ["--theme", "--config", "--panels"].contains(&args[i - 1].as_str()))
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

    if let Some(list) = value("--panels")? {
        settings.panels =
            config::Panels::parse(list.split(',')).map_err(|e| eyre!("{e}\n\n{USAGE}"))?;
    }

    let mut app = app::App::new(settings)?;

    // ratatui restores the terminal on a panic but does not know about mouse capture
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(stdout(), DisableMouseCapture);
        previous(info);
    }));
    ratatui::run(|terminal| -> Result<()> {
        execute!(stdout(), EnableMouseCapture)?;
        let result = app.run(terminal);
        execute!(stdout(), DisableMouseCapture)?;
        result
    })?;

    // left with q: the report stays in the scrollback
    if app.report.is_some() {
        print!("{}", app.shift.text(&app.mon));
    }
    Ok(())
}
