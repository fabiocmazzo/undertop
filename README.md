<div align="center">

# UNDERTOP

**SUBTERRANEAN OPERATIONS TERMINAL — OPERATOR MANUAL, REV 0.5**

*A system monitor for people who live underground. Remain calm. Remain below.*

[![MAINFRAME INTEGRITY](https://github.com/fabiocmazzo/undertop/actions/workflows/ci.yml/badge.svg)](https://github.com/fabiocmazzo/undertop/actions/workflows/ci.yml)
[![Built With Ratatui](https://ratatui.rs/built-with-ratatui/badge.svg)](https://ratatui.rs/)

![undertop in its SILO palette: boot sequence, a ticker alert, a process dossier with its charts, the lineage tree, the termination dialog, the level 142 schematic, a lockdown and the shift report](assets/undertop.gif)

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

One line, no root (x86_64 or aarch64; it checks the download's SHA-256 and installs to `~/.local/bin`):

```sh
curl -fsSL https://raw.githubusercontent.com/fabiocmazzo/undertop/main/install.sh | sh
```

`UNDERTOP_VERSION=v0.3.0` picks a release and `UNDERTOP_INSTALL_DIR` another directory. Read
[`install.sh`](install.sh) first if you prefer: it is short.

On Arch Linux and its derivatives, this one installs it as a pacman package instead (`undertop-bin`;
`UNDERTOP_PKG=undertop` builds it from source), removable with `sudo pacman -R undertop-bin`:

```sh
curl -fsSL https://raw.githubusercontent.com/fabiocmazzo/undertop/main/install-arch.sh | sh
```

Or download the binary from the [latest release](https://github.com/fabiocmazzo/undertop/releases/latest),
or build it:

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
| `--theme <amber\|phosphor\|cobalt\|lockdown\|silo>` | start with another palette |
| `--panels <list>` | stations to show, e.g. `cpu,memory,procs` (see § 3.1) |
| `--config <path>` | read another config file |
| `--print-config` | print a config template with every default |
| `--help` | print usage |

### § 3.1 Standing orders (config file)

Defaults live in `~/.config/undertop/config.toml` (or `$XDG_CONFIG_HOME/undertop/config.toml`). Every
key is optional; flags win over the file.

```sh
mkdir -p ~/.config/undertop && undertop --print-config > ~/.config/undertop/config.toml
```

```toml
theme = "silo"           # silo, amber, phosphor, cobalt, lockdown
panels = ["cpu", "memory", "thermal", "comms", "vaults", "gpu", "procs"]
interval_ms = 1000       # 250 to 10000
crt = true               # CRT emulation
boot = true              # power-on self test
power_down = true        # CRT switch-off when leaving
shift_report = true      # end-of-shift report when leaving (§ 5.3)
voice = true             # robotic voice for announcements (needs espeak-ng, espeak or spd-say)
inspections = true       # surprise inspections, about every 30 minutes on average
lockdown_on_heat = true  # see § 7
```

`panels` lists the stations to man; the others stay dark and the rest of the console takes their room.
`panels = ["cpu", "procs"]` makes a compact reactor-and-manifest terminal.

## § 4. INSTRUMENT PANEL

This terminal sits on level 142, in I.T. Each station carries the plate of the department that answers
for it: **MCH** Mechanical, **SUP** Supply, **IT**, and **JUD** Judicial, who keeps the manifest and signs
every termination.

| Code | Station | Reads |
| --- | --- | --- |
| — | **HEADER** | node, kernel, uptime, load, tasks, the current shift, a status that blinks when things run hot, and the clock |
| 01 | **REACTOR CORE** | one gauge per logical CPU, total output history |
| 02 | **RESERVOIR** | memory and swap as liquid tanks, free/used figures, the battery as the *aux power cell* |
| 03 | **THERMAL** | every hwmon sensor against its critical temperature |
| 04 | **COMMS** | network receive/transmit rates as an oscilloscope, and each link (wire, wifi, VPN) with its own rates |
| 05 | **VAULTS** | mounted filesystems, how full they are, disk read/write rates |
| 06 | **MANIFEST** | every process on the node: operator, load, memory, state, time, command |
| 07 | **AUX REACTOR** | GPUs: Intel iGPU frequency, NVIDIA load, temperature and VRAM (wide screens only) |

The AUX REACTOR never wakes a sleeping NVIDIA GPU to read it: a laptop dGPU in power saving is shown as
`◌ DORMANT` and left alone.

The ticker at the bottom carries facility announcements, and alerts when a process holds a full core
for 5 seconds, takes more than a quarter of the memory, or becomes a zombie.

A lit lamp (`◉`) in a station's title means that station needs attention.

## § 5. AUTHORIZED PROCEDURES

| Key | Procedure |
| --- | --- |
| `↑ ↓` `j k` | move through the manifest |
| `PgUp PgDn`, `g G` | one page, first / last entry |
| `c` `m` `p` `n` | sort by CPU / memory / PID / name (press again to reverse) |
| `s` `Tab`, `r` | next sort key, reverse order |
| `/` `f` | filter by command, operator or PID (`Enter` keeps it, `Esc` clears it) |
| `Enter` | open the selected process's dossier |
| `v` `F5` | lineage view: the manifest as a process tree |
| `K` `Del` `F9` | request termination of the selected process |
| `t` | next phosphor palette |
| `x` | CRT emulation on / off |
| `+` `-` | slower / faster sampling |
| `Space` | freeze telemetry |
| `?` `h` `F1` | this manual, abridged |
| `q` `Esc` | leave the terminal (after the shift report) |

The mouse works too: a click selects a process, a double click opens its dossier, the wheel scrolls,
a click on a column title sorts by it, and the keycaps in the bottom line can be clicked. A click
outside a dossier or on the manual closes it. Termination is only ever confirmed from the keyboard.
To select text with the mouse, hold `Shift` while dragging.

### § 5.1 Dossiers

`Enter` opens everything the terminal knows about a process: parent and children, start time, threads,
resident and virtual memory, disk I/O, executable, working directory, control group, and its open files.
While it stays open, it charts the process's CPU and resident memory.
What belongs to other operators is marked **CLASSIFIED**.

### § 5.2 Termination of a process

Termination requires authorization. The terminal will ask you to confirm with `Y` (SIGTERM, the
courteous way) or `9` (SIGKILL, which is not discussed). Any other key aborts the procedure.

### § 5.3 End of shift

Leaving with `q` files a shift report: how long you were on duty, the peak and average reactor load,
the memory and thermal peaks, what went through comms, and every alert, lockdown, termination and
dossier, with a verdict from management. Any key signs you out; the report stays printed in your
terminal afterwards. `Ctrl-C` leaves without paperwork; `shift_report = false` skips it for good.

## § 6. PHOSPHOR PALETTES

| Palette | When to use it |
| --- | --- |
| **SILO** | standard issue: teal line-art, like the schematic screens on the lower levels |
| **AMBER** | the old standard issue, still found in many stations |
| **PHOSPHOR** | for operators who remember the green screens |
| **COBALT** | night shift |
| **LOCKDOWN** | when something is wrong, or when you want it to look that way |

## § 7. KNOWN ANOMALIES

The display may flicker, roll, or briefly shift a line sideways. This is CRT emulation and is
**working as intended**. Operators who find it distressing may press `x`. Do not report it to your shift
supervisor.

If a thermal sensor runs critically hot, the facility will seal itself. Remain at your station. The
seal lifts on its own, or when you acknowledge it. (`lockdown_on_heat = false` in the config file, for
operators who prefer to burn quietly.)

There are rumours of a manual override. Management neither confirms nor denies them.

The facility schematics are restricted. Operators are reminded that their terminal sits on level 142,
and that curiosity is noted. If a speech synthesizer such as `espeak-ng` is installed, the facility
will tell you so out loud (`voice = false` to keep it quiet).

Inspectors from the upper levels may sweep the console without notice, about every half hour. They wait
until you close what you are reading, and any key sends them on their way. Zombie processes, an
unanswered alert or a reactor above 90 % fail the level, and that goes in your file and in the shift
report. (`inspections = false`, for operators with nothing to hide.)

Some operators still remember the old codes from before. Those who type one into the manifest find
that the palettes go further than `t` lets on, until the end of the shift.

## § 8. FOR MAINTENANCE PERSONNEL

- Design notes and every decision taken so far: [`projeto.md`](projeto.md).
- What changed in each version: [`SHIFT_LOG.md`](SHIFT_LOG.md).
- AUR packages (`undertop` from source, `undertop-bin` prebuilt): [`packaging/aur`](packaging/aur), with
  the steps to update them after a release.
- Releases: push a tag `vX.Y.Z` with a matching `## vX.Y.Z — Name` section in the shift log; the
  release workflow builds the binary and publishes it with that section as notes.
- `cargo test` renders every screen at several sizes; `UNDERTOP_DUMP=console cargo test -- --nocapture`
  prints one as text (`console`, `boot`, `help`, `kill`, `dossier`, `lockdown`, `tree`, `power-down`,
  `report`, `access`, `elevation`, `plan`, `safeguard`).
- The demo above is recorded with [vhs](https://github.com/charmbracelet/vhs) from
  [`demo/undertop.tape`](demo/undertop.tape) (the shrinking step is at the top of that file).

## § 9. LICENSE

MIT. See [`LICENSE`](LICENSE).

```
REMAIN CALM. REMAIN BELOW.   ◆   THE MAINFRAME SEES ALL THREADS   ◆   A CLEAN SWAP IS A HAPPY SWAP
```
