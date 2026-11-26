# SHIFT LOG

Changes to the terminal, newest shift first.

## Unreleased

- One-line install: `curl -fsSL https://raw.githubusercontent.com/fabiocmazzo/undertop/main/install.sh | sh`.
- PKGBUILDs for the AUR (`undertop` and `undertop-bin`).

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
