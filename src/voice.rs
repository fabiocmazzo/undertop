//! Robotic announcements through whatever speech synthesizer is installed. Optional: with none,
//! the terminal stays silent.

use std::process::{Command, Stdio};
use std::thread;

/// Speaker programs to try, in order, with arguments that make them sound like a facility PA.
const SPEAKERS: &[(&str, &[&str])] = &[
    // espeak's "robosoft" variant is the most mechanical voice commonly installed
    (
        "espeak-ng",
        &["-v", "en-us+robosoft", "-s", "125", "-p", "25"],
    ),
    ("espeak", &["-v", "en-us", "-s", "125", "-p", "20"]),
    ("spd-say", &["-w", "-l", "en", "-r", "-40", "-p", "-60"]),
];

fn find(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|d| d.join(program).is_file()))
}

/// Says `text` in the background. Returns false when there is nothing to say it with.
pub fn say(text: &str) -> bool {
    let Some((program, args)) = SPEAKERS.iter().find(|(p, _)| find(p)) else {
        return false;
    };
    let child = Command::new(program)
        .args(*args)
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        // reaped on its own thread, so the frame never waits and no zombie is left
        Ok(mut c) => {
            thread::spawn(move || c.wait());
            true
        }
        Err(_) => false,
    }
}
