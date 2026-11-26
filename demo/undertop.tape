# Records assets/undertop.gif. From the project root:
#   cargo build --release && vhs demo/undertop.tape
# then shrink it (vhs writes ~16 MB; this brings it to ~8 MB, under GitHub's 10 MB comfort zone):
#   ffmpeg -i assets/undertop.gif -vf "fps=8,scale=1120:-1:flags=neighbor,split[a][b];[a]palettegen=max_colors=64:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle" /tmp/u.gif && mv /tmp/u.gif assets/undertop.gif
#
# Careful when editing: in the termination dialog, Enter or Y sends SIGTERM to the
# selected process. This tape only leaves the dialog with "n".
#
# Off camera it starts a busy `yes` (stopped by `timeout`), so the ticker raises an alert,
# there is something harmless to open a dossier on, and a sector lit on the level 142 plan.
# Status messages (palette changes) hold alerts back, so the alert is shown before anything else.


Output assets/undertop.gif

Set Shell "bash"
Set FontSize 13
Set Width 1280
Set Height 760
Set Padding 0
Set Framerate 12
Set TypingSpeed 80ms
Set PlaybackSpeed 1.0

Hide
Type "export PATH=$PWD/target/release:$PATH && (timeout 80 yes > /dev/null &) && clear"
Enter
Sleep 500ms
Show

Type "undertop"
Sleep 400ms
Enter

# boot, then the console: `yes` holds a core and the ticker raises an alert
Sleep 10s

# palettes
Type "t"
Sleep 1.5s
Type "t"
Sleep 1.5s
Type "t"
Sleep 1.2s
Type "t"
Sleep 800ms

# filter down to the `yes` and open its dossier (Enter keeps the filter, the
# next Enter opens the dossier, Escape closes it, "/" + Escape clears the filter)
Type "/"
Sleep 300ms
Type "yes"
Sleep 800ms
Enter
Sleep 400ms
Enter
Sleep 4.5s
Escape
Sleep 600ms
Type "/"
Escape
Sleep 800ms

# lineage: the manifest as a tree
Type "v"
Sleep 2s
Down 6
Sleep 1.5s
Type "v"
Sleep 800ms

# operator manual
Type "?"
Sleep 2.5s
Type " "
Sleep 800ms

# termination dialog, aborted
Type "K"
Sleep 2s
Type "n"
Sleep 1.2s

# the level 142 schematic: access, elevation, plan, safeguard
Type "142"
Sleep 2.8s
Type " "
Sleep 3.5s
Type " "
Sleep 4s
Type " "
Sleep 3.5s
Escape
Sleep 1s

# do not press !
Type "!"
Sleep 3.5s
Escape
Sleep 1.5s

# end of shift: the report, then the power-down
Type "q"
Sleep 3.5s
Type " "
Sleep 2s
