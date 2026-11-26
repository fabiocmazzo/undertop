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
┏ 03 THERMAL ━━━━┓┏ 04 COMMS (network) ━━━━━━━┓┏ 05 VAULTS (disks) ━━━━━━━━━━━━┓
┃ sensor bars    ┃┃ RX/TX rates, braille scope ┃┃ mount, used/total, bar        ┃
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
  merged).
- **06 Manifest**: the process table (threads hidden), with a segmented load bar per process, the state
  colored, and the sort column highlighted in the header.
- **Overlays**: the boot sequence (a power-on self test that uses real figures, ~3 s, any key skips), the
  operator manual (`?`), and the termination authorization dialog with flashing hazard borders.
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
| `K` / `Del` / `F9` | terminate the selected process (`Y` SIGTERM, `9` SIGKILL, any other key aborts) |
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
| `src/app.rs` | state, main loop (80 ms frames, data refresh on its own interval), keys, sorting/filtering |
| `src/sysmon.rs` | telemetry: wraps `sysinfo`, keeps 300-sample histories, reads the battery from sysfs |
| `src/theme.rs` | the four RGB palettes, load-level colors, color blending |
| `src/ui/mod.rs` | screen layout and one function per panel, footer, overlays, render test |
| `src/ui/widgets.rs` | hand-drawn instruments: block font, segmented bars, vertical gauges, tanks, area chart |
| `src/ui/boot.rs` | boot sequence |
| `src/ui/effects.rs` | CRT post-processing pass over the finished buffer |

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
10. **Test**: one render test draws every screen (console, boot, help, kill dialog) into ratatui's
    `TestBackend` at sizes from 60x20 to 240x60, so a layout that draws out of bounds fails `cargo test`.
    `UNDERTOP_DUMP=<0..3> cargo test -- --nocapture` prints a screen as text.

## Ideas for later

- Process tree view and per-process detail panel (open files, environment, cgroup).
- GPU panel (sysinfo has a `gpu` feature; the Intel iGPU would need `/sys/class/drm`).
- Per-interface network view, disk I/O rates.
- A config file for default palette, refresh rate and which panels to show.
