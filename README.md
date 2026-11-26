<div align="center">

# UNDERTOP

**SUBTERRANEAN OPERATIONS TERMINAL — OPERATOR MANUAL, REV 0.1**

*A system monitor for people who live underground. Remain calm. Remain below.*

![undertop: boot sequence, palettes, process manifest and termination dialog](assets/undertop.gif)

</div>

```
▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚
  THIS TERMINAL IS THE PROPERTY OF MECHANICAL. UNAUTHORIZED TERMINATION OF
  PROCESSES WILL BE NOTED IN YOUR SHIFT RECORD.
▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚  ▚▚
```

## § 1. PURPOSE OF THIS TERMINAL

`undertop` is an `htop` / `btop` alternative for Linux, dressed as the operations console of an
underground silo: amber phosphor, chunky block letters, hazard stripes, liquid tanks, and a CRT that is
not entirely well. It is a hobby project, loosely inspired by the screens of the series *Silo*. It is made
to run on one machine and to make whoever sees it say "that's different".

Written in Rust with [ratatui](https://ratatui.rs/) and [sysinfo](https://crates.io/crates/sysinfo).

## § 2. CLEARANCE REQUIREMENTS

| Requirement | Specification |
| --- | --- |
| Operating system | Linux (the only surface that has been surveyed) |
| Terminal | truecolor, 80x24 at the least; 160x45 or more for the full instrument panel |
| Font | any monospace font with box drawing and block characters |
| Toolchain | Rust 1.85 or newer (edition 2024) |

## § 3. COMMISSIONING THE TERMINAL

```sh
cargo install --git https://github.com/fabiocmazzo/undertop
undertop
```

Or from a checkout:

```sh
cargo run --release
```

| Flag | Effect |
| --- | --- |
| `--no-boot` | skip the power-on self test |
| `--theme <amber\|phosphor\|cobalt\|lockdown>` | start with another palette |
| `--help` | print usage |

## § 4. INSTRUMENT PANEL

| Code | Station | Reads |
| --- | --- | --- |
| — | **HEADER** | node, kernel, uptime, load, tasks, the current shift, a status that blinks when things run hot, and the clock |
| 01 | **REACTOR CORE** | one gauge per logical CPU, total output history |
| 02 | **RESERVOIR** | memory and swap as liquid tanks, free/used figures, the battery as the *aux power cell* |
| 03 | **THERMAL** | every hwmon sensor against its critical temperature |
| 04 | **COMMS** | network receive/transmit rates as an oscilloscope |
| 05 | **VAULTS** | mounted filesystems and how full they are |
| 06 | **MANIFEST** | every process on the node: operator, load, memory, state, time, command |

A lit lamp (`◉`) in a station's title means that station needs attention.

## § 5. AUTHORIZED PROCEDURES

| Key | Procedure |
| --- | --- |
| `↑ ↓` `j k` | move through the manifest |
| `PgUp PgDn`, `g G` | one page, first / last entry |
| `c` `m` `p` `n` | sort by CPU / memory / PID / name (press again to reverse) |
| `s` `Tab`, `r` | next sort key, reverse order |
| `/` `f` | filter by command, operator or PID (`Enter` keeps it, `Esc` clears it) |
| `K` `Del` `F9` | request termination of the selected process |
| `t` | next phosphor palette |
| `x` | CRT emulation on / off |
| `+` `-` | slower / faster sampling |
| `Space` | freeze telemetry |
| `?` `h` `F1` | this manual, abridged |
| `q` `Esc` | leave the terminal |

### § 5.1 Termination of a process

Termination requires authorization. The terminal will ask you to confirm with `Y` (SIGTERM, the
courteous way) or `9` (SIGKILL, which is not discussed). Any other key aborts the procedure.

## § 6. PHOSPHOR PALETTES

| Palette | When to use it |
| --- | --- |
| **AMBER** | standard issue |
| **PHOSPHOR** | for operators who remember the green screens |
| **COBALT** | night shift |
| **LOCKDOWN** | when something is wrong, or when you want it to look that way |

## § 7. KNOWN ANOMALIES

The display may flicker, roll, or briefly shift a line sideways. This is CRT emulation and is
**working as intended**. Operators who find it distressing may press `x`. Do not report it to your shift
supervisor.

## § 8. FOR MAINTENANCE PERSONNEL

- Design notes and every decision taken so far: [`projeto.md`](projeto.md).
- `cargo test` renders every screen at several sizes; `UNDERTOP_DUMP=0 cargo test -- --nocapture` prints
  the console as text (`1` boot, `2` manual, `3` termination dialog).
- The demo above is recorded with [vhs](https://github.com/charmbracelet/vhs) from
  [`demo/undertop.tape`](demo/undertop.tape) (the shrinking step is at the top of that file).

```
REMAIN CALM. REMAIN BELOW.   ◆   THE MAINFRAME SEES ALL THREADS   ◆   A CLEAN SWAP IS A HAPPY SWAP
```
