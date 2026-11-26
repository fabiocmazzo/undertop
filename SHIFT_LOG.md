# SHIFT LOG

Changes to the terminal, newest shift first.

## Unreleased

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
