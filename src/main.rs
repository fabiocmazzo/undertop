//! undertop: a system monitor dressed as the operations console of an underground silo.

mod app;
mod sysmon;
mod theme;
mod ui;

use color_eyre::Result;

const USAGE: &str = "\
undertop - subterranean operations terminal

USAGE: undertop [--no-boot] [--theme <amber|phosphor|cobalt|lockdown>]

Press ? inside the program for the operator manual.";

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    let boot = !args.iter().any(|a| a == "--no-boot");
    let mut app = app::App::new(boot)?;
    if let Some(name) = args
        .iter()
        .position(|a| a == "--theme")
        .and_then(|i| args.get(i + 1))
    {
        match theme::THEMES
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(name))
        {
            Some(i) => app.theme_idx = i,
            None => {
                eprintln!("unknown theme: {name}\n\n{USAGE}");
                std::process::exit(2);
            }
        }
    }
    ratatui::run(|terminal| app.run(terminal))
}
