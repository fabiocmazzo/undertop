# SHIFT LOG

Changes to the terminal, newest shift first.

## v0.5.0 — Inspection Day

- Release binaries for aarch64 (Raspberry Pi, ARM servers) besides x86_64; `install.sh` and
  `undertop-bin` pick the right one.
- Surprise inspections, about every 30 minutes: a scan line sweeps the console, then a stamp says
  PASSED or FAILED (zombies, an unanswered alert or the reactor above 90 %). They never interrupt an
  open overlay, any key dismisses them, the shift report counts them, and `inspections = false` turns
  them off.
- An old code, typed in the manifest, unlocks a secret palette for the rest of the shift.
- The manual's revision follows the version.

## v0.4.0 — Lower Levels

- `panels` in the config (and `--panels`) picks the stations; the others give up their room.
- SILO palette, teal line-art on near-black, is now the default (`theme = "amber"` brings the old
  one back).
- Shift+K opens the termination dialog on terminals that report it as Shift + `k`.
- The console looks more like a silo terminal: department plates on every station (MCH, SUP, IT,
  JUD), a drawing's scale on their bottom edges, a depth gauge on level 142 in the header (hazard
  stripes only when something runs hot), the termination dialog as a judicial order, and new notices
  on the ticker.
- The header no longer cuts the task count short on mid-sized terminals.

## v0.3.1 — Supply Line

- One-line install: `curl -fsSL https://raw.githubusercontent.com/fabiocmazzo/undertop/main/install.sh | sh`.
- PKGBUILDs for the AUR (`undertop` and `undertop-bin`), and `install-arch.sh`, which installs them as
  pacman packages with one line until they are published there.

## v0.3.0 — Level 142

- Mouse: click to select, double click for the dossier, wheel to scroll, click a column title to sort
  by it, click the keycaps in the bottom line. The termination dialog still wants the keyboard.
- End-of-shift report when leaving with `q`: time on duty, peaks, traffic, incidents and a verdict,
  also left printed in the terminal (`shift_report = false` to skip it).
- The dossier charts the process's CPU and resident memory while it is open.
- COMMS lists each network link (wire, wifi, VPN, cellular) with its own rates; idle container
  bridges stay hidden.
- Something on level 142. With a speech synthesizer installed, it speaks.

## v0.2.1 — Conservation Order

- Less than half the CPU (5.1 % → 2.1 % of one core) and a third of the memory (18 → 7 MB): no more
  reading 3000 threads and every process's I/O each second; NVMe sensors read every 5 s.
- The CRT emulation sends half as much to the terminal (the rolling band is flat-topped).
- The dossier's disk I/O says MEASURING… until it has a real rate, instead of a first bogus spike.

## v0.2.0 — Second Shift

The facility grows new stations, and a sense of humour.

- DOSSIER (`Enter`): parent, children, threads, memory, disk I/O, executable, working directory,
  cgroup and open files of a process; other operators' secrets are CLASSIFIED.
- LINEAGE (`v` / `F5`): the manifest as a process tree; filtering keeps the ancestors of each match.
- VAULTS shows disk read/write rates and their history.
- AUX REACTOR: Intel iGPU and NVIDIA GPUs. A sleeping laptop dGPU is never woken to be measured.
- Standing orders: `~/.config/undertop/config.toml`, `--config`, `--print-config`.
- The ticker now announces processes that hog a core, drain the reservoir or turn into zombies.
- The terminal switches off like an old CRT when you leave (any key skips it).
- Classified: what happens when things get too hot.

## v0.1.0 — First Shift

Mainframe brought online.

- Boot sequence, block-font logo and clock, hazard stripes.
- Stations: REACTOR CORE (CPU), RESERVOIR (memory, swap, battery), THERMAL, COMMS (network),
  VAULTS (disks) and the process MANIFEST with sort, filter and authorized termination.
- Four phosphor palettes: AMBER, PHOSPHOR, COBALT, LOCKDOWN.
- CRT emulation: scanlines, vignette, rolling band, brownouts and the odd glitch.
