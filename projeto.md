# undertop — Subterranean Operations Terminal

A hobby alternative to `htop` / `btop` for Linux, dressed as the operations console of an underground
silo: amber phosphor, chunky block letters, hazard stripes, liquid tanks, and a CRT that flickers a bit.
Loosely inspired by the screens of the series *Silo*
([reference](https://www.hudsandguis.com/home/2025/silo-ui)). The goal is not to be useful on every
machine: it only has to run well on this one (Linux, a 16-thread Intel laptop with a battery), and to make
whoever sees it say "that's different!".

## Running

```sh
cargo run --release                 # boot sequence, then the console
cargo run --release -- --no-boot    # straight to the console
cargo run --release -- --theme phosphor
cargo test                          # renders every screen at several sizes (TestBackend)
```

Minimum terminal size is 80x24; it looks best at 160x45 or larger, in a truecolor terminal.

## Screen

```
╔ SUBTERRANEAN OPERATIONS TERMINAL ══════════════════════════════════════════ OS ╗
║ UNDERTOP (block font)   NODE / KERNEL / UPTIME / LOAD / SHIFT / STATUS   ▚▚▚  CLOCK ║
╚════════════════════════════════════════════════════════════════════ PALETTE ═══╝
┏ 01 REACTOR CORE (CPU) ━━━━━━━━━━━━━━━━━━━━━━━━━┓┏ 02 RESERVOIR (memory) ━━━━━━━┓
┃ per-core vertical gauges │ total load history  ┃┃ MEM tank │ SWAP tank │ stats ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛┃ aux power cell, mem history  ┃
┏ 03 THERMAL ━━━┓┏ 04 COMMS (network) ━━━━━┓┏ 05 VAULTS (disks) ━━┓┏ 07 AUX REACTOR ━┓
┃ sensor bars   ┃┃ RX/TX rates, braille   ┃┃ used/total, I/O     ┃┃ GPUs (≥140 col) ┃
┏ 06 MANIFEST (processes) ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ PID  OPERATOR  CPU%  LOAD  MEM  ST  TIME  COMMAND                               ┃
 [Q] QUIT [?] HELP …                      scrolling ticker / status messages
```

- **Header**: the `UNDERTOP` logo in a 3x5 pixel block font drawn with half blocks, host facts, the work
  shift by the hour of day, a status that blinks when the load or a temperature is high, scrolling hazard
  stripes and a big clock with blinking colons. Below 150 columns the logo shrinks, and below 100 the clock is
  hidden. The boot screen uses the biggest logo that fits.
- **01 Reactor core**: one vertical gauge per logical CPU with 1/8-cell resolution, hex labels, and a
  column chart of total load over a dotted grid. The panel lamp blinks above 50 %.
- **02 Reservoir**: memory and swap drawn as liquid tanks with an animated surface, darker toward the
  bottom; used/free/swap figures, the laptop battery as an "aux power cell", memory history.
- **03 Thermal**: every hwmon sensor as a segmented bar against its critical temperature (100 °C when the
  sensor does not report one); package/composite sensors first.
- **04 Comms**: RX/TX rates (loopback excluded) and totals, as a braille oscilloscope.
- **05 Vaults**: real filesystems only (no tmpfs/overlay/…), one line per device (btrfs subvolumes are
  merged), then the current disk read/write rate and its history in the rows left over.
- **06 Manifest**: the process table (threads hidden), with a segmented load bar per process, the state
  colored, and the sort column highlighted in the header. `v` turns it into **LINEAGE**, a process tree
  drawn with `├─ └─ │`; siblings follow the current sort, and a filter keeps each match's ancestors.
- **07 Aux reactor** (only when the screen is at least 140 columns wide): one block per GPU with name,
  state, a load bar and details. Intel: actual/max frequency ("RC6 IDLE" at 0 MHz). NVIDIA: load,
  temperature, VRAM; `◌ DORMANT` with a breathing bar while the dGPU sleeps.
- **Footer**: keycaps (as many as fit, leaving 30 columns), then a status message, else a blinking
  ticker alert, else the scrolling ticker.
- **Overlays**: the boot sequence (a power-on self test that uses real figures, ~3 s, any key skips), the
  operator manual (`?`), the termination authorization dialog with flashing hazard borders, the
  **dossier** (`Enter`) and the **lockdown** alarm.
- **Easter eggs**:
  - **Lockdown**: `!` (hinted at in the ticker: "DO NOT PRESS !"), or automatically when a sensor reaches
    95 °C or 95 % of its critical temperature (at most every 10 minutes, `lockdown_on_heat` turns it off).
    Switches to the LOCKDOWN palette, flashes a hazard frame around the screen, shows a big `LOCKDOWN`
    with the reason, and rings the terminal bell every 2 s. Lifts after 8 s or on any key, restoring the
    palette.
  - **Ticker alerts**: a process holding a full core for 5 s ("EXCEEDING ALLOCATION"), using more than a
    quarter of the memory ("DRAINING THE RESERVOIR"), or a zombie ("NOTIFY SANITATION"). Each is
    announced once per process and condition, 6 s on screen, queued.
  - **Power-down**: leaving with `q`/`Esc` plays a 0.9 s CRT switch-off: the picture squashes into a
    bright line, the line shrinks to a dot, the dot fades. Any key skips it; `Ctrl-C` quits at once.
- **CRT emulation** (`x` toggles it): scanlines (odd rows dimmer), a vignette, a bright band rolling down
  the screen (one row every 3 frames), a rare brownout that dims a 3-row strip, and once in a while a
  row that glitches sideways for one frame. Effects avoid repainting the whole screen at once: that
  stalled slower terminals and the vhs recorder.

## Keys

| Key | Action |
| --- | --- |
| `↑ ↓` / `j k`, `PgUp PgDn`, `g G` | move through the manifest |
| `c m p n` | sort by CPU / memory / PID / name (again: reverse) |
| `s` / `Tab`, `r` | next sort key, reverse order |
| `/` or `f` | filter by command, user or PID (`Enter` keeps it, `Esc` clears it) |
| `Enter` | dossier of the selected process (`K` in it terminates, any other key closes) |
| `v` / `F5` | lineage (tree) view on/off |
| `K` / `Del` / `F9` | terminate the selected process (`Y` SIGTERM, `9` SIGKILL, any other key aborts) |
| `!` | lockdown (secret; any key lifts it) |
| `t` | next palette: AMBER, PHOSPHOR, COBALT, LOCKDOWN |
| `x` | CRT emulation on/off |
| `+` / `-` | slower / faster sampling (250 ms steps, 250 ms to 10 s) |
| `Space` | freeze telemetry |
| `?` / `h` / `F1` | operator manual |
| `q` / `Esc` / `Ctrl-C` | quit |

## Code layout

| File | Role |
| --- | --- |
| `src/main.rs` | arguments, terminal setup through `ratatui::run` |
| `src/config.rs` | settings: defaults, then `config.toml`, then flags; the `--print-config` template |
| `src/app.rs` | state, main loop (80 ms frames, data refresh on its own interval), keys, sorting/filtering, tree building, lockdown and alert timing |
| `src/sysmon.rs` | telemetry: wraps `sysinfo`, keeps 300-sample histories, battery from sysfs, disk I/O from `/proc/diskstats`, ticker alerts, dossiers |
| `src/gpu.rs` | GPU detection and sampling (sysfs for Intel, `nvidia-smi` in a background thread) |
| `src/theme.rs` | the four RGB palettes, load-level colors, color blending |
| `src/ui/mod.rs` | screen layout and one function per panel, footer, overlays, render test |
| `src/ui/widgets.rs` | hand-drawn instruments: block font, segmented bars, vertical gauges, tanks, area chart |
| `src/ui/boot.rs` | boot sequence |
| `src/ui/overlays.rs` | dossier and lockdown overlays |
| `src/ui/effects.rs` | CRT post-processing pass over the finished buffer, and the power-down animation |

## Decisions

1. **Rust + ratatui 0.30 (crossterm backend)**, as requested. `ratatui::run` handles raw mode, the
   alternate screen and restoring the terminal on panic.
2. **`sysinfo` 0.40 for telemetry** instead of parsing `/proc` by hand: it covers CPU, memory, processes,
   network, disks and hwmon sensors, and is well maintained. Only the battery is read directly from
   `/sys/class/power_supply/BAT*`, since sysinfo does not cover it. (Note: in 0.40 the constructors return
   `Result`, and per-CPU usage is `Cpu::usage()`.)
3. **Instruments drawn cell by cell into the buffer** rather than built from stock widgets: the look needs
   per-cell color (gradients along a bar, darker water at the bottom of a tank, a glowing newest column).
   Stock widgets are used where they already look right: `Table` for processes, `Chart` with braille for
   the network scope, `Paragraph` for text.
4. **Every palette color is RGB**, so the CRT pass can scale and blend colors. This assumes a truecolor
   terminal, which is fine for the one machine this targets.
5. **CRT effects as a post-processing pass** (`ui/effects.rs`) over the finished frame, independent of the
   panels, and switchable at runtime. Randomness comes from a tiny xorshift generator in `App`, so no `rand`
   dependency.
6. **Two clocks in the loop**: frames every 80 ms (~12 fps, enough for blinking, waves, the ticker and the
   effects), and data refreshes on a separate, adjustable interval (1 s by default). Disks and battery
   refresh every 5th data refresh, since they change slowly.
7. **Per-process CPU% is per core** (like `htop`): 100 % is one busy core. The load bar fills at one busy
   core.
8. **`chrono` (clock feature only)** for the local time in the header.
9. **Kill uses `K` (capital)**, not `k`, because `j/k` move the selection; the dialog asks for an explicit
   `Y` (SIGTERM) or `9` (SIGKILL), and any other key aborts.
10. **Test**: render tests draw every screen (console, boot, help, kill, dossier, lockdown, tree,
    power-down) into ratatui's `TestBackend` at sizes from 60x20 to 240x60, and run the animations
    through their whole timing, so drawing out of bounds fails `cargo test`. Unit tests cover the tree
    builder, the config parser, the `nvidia-smi` parser and the disk filter.
    A slow test, `cargo test -- --ignored hog_alert`, runs a real `yes` and waits for the ticker alert
    (the hog threshold is 90 % of a core: a busy single-threaded process reads 98–102 %, so 100 % was
    flaky).
    `UNDERTOP_DUMP=<screen> cargo test -- --nocapture` prints a screen as text.
11. **CI and releases on GitHub Actions**: every push to `main` runs `cargo fmt --check`, the tests and
    clippy with `-D warnings`. A `vX.Y.Z` tag builds the Linux binary and publishes a release whose title
    and notes come from that version's section of `SHIFT_LOG.md`.
12. **Never wake the NVIDIA dGPU.** Any `nvidia-smi` call wakes a runtime-suspended laptop GPU (measured
    here: 0.8 s, then ~16 s awake). A background thread reads the PCI `runtime_status` every second,
    which does not wake it, and only queries while it is `active`, at most every 20 s, longer than the
    idle delay, so the query itself never keeps it awake. The UI never blocks on it. Desktop cards
    without runtime PM are queried every 3 s. Intel exposes no utilization in sysfs, so its frequency
    ratio stands in for load.
13. **Disk I/O from `/proc/diskstats`** instead of sysinfo's per-mount numbers: summing whole devices
    (`nvme0n1`, `sda`… not partitions, loop, zram or dm) counts each byte once, where btrfs subvolumes
    would count it several times.
14. **Config in TOML** (`serde` + `toml`) with `deny_unknown_fields`, so a typo is an error at start, not a
    silently ignored setting. Precedence: defaults, file, flags. A missing file at the default path is
    fine; a missing file named with `--config` is an error.
15. **Dossier data refreshes with the telemetry**, not every frame: reading `/proc/<pid>/fd` of a big
    process 12 times a second would be wasteful. What cannot be read (other users' processes) shows as
    CLASSIFIED instead of an error.
16. **Effects never repaint the whole screen in one frame** (see the CRT note above); the lockdown flashes
    only its frame and box, and the power-down is short.

17. **Measured cost, and what was cut** (release build, 160x45, ~465 processes, 16 threads). Before:
    5.1 % of one core, 18 MB RSS, 109 KiB/s to the terminal with CRT on. A breakdown
    (`cargo test --release -- --ignored refresh_cost --nocapture`) showed drawing at ~1.5 % and data
    collection at ~3.3 %, almost all in `refresh_processes` (45 ms per refresh): sysinfo's default lists
    every thread as a process (3000+ here) and reads every process's I/O. Now: processes are refreshed
    with only CPU, memory, user and command and `without_tasks()` (8 ms); per-process I/O is read only
    for the PID with an open dossier (`Monitor::watch`, shown as "MEASURING…" until two readings exist,
    since the first one is a lifetime total); NVMe sensors (~2 ms each, the drive is queried) are read
    every 5th refresh; the CRT band is flat-topped so moving it repaints 2 rows instead of 8. After:
    2.1 % of one core, 6.9 MB, 56 KiB/s (13 KiB/s with CRT off). No battery-saver mode: at this cost it
    would not be worth the complexity.

## Ideas for later

- Per-interface network view.
- Choose which panels to show in the config file.
- Process environment in the dossier (left out for now: it often holds secrets, and screens get shared).
- Sort and search inside the dossier's open-files list.
