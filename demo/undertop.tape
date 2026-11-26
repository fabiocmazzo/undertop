# Records assets/undertop.gif. From the project root:
#   cargo build --release && vhs demo/undertop.tape
# then shrink it (vhs writes ~16 MB; this brings it to ~7 MB, under GitHub's 10 MB comfort zone):
#   ffmpeg -i assets/undertop.gif -vf "fps=8,scale=1120:-1:flags=neighbor,split[a][b];[a]palettegen=max_colors=64:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle" /tmp/u.gif && mv /tmp/u.gif assets/undertop.gif
#
# Careful when editing: in the termination dialog, Enter or Y sends SIGTERM to the
# selected process. This tape only leaves the dialog with "n".
#
# Off camera it starts a busy `yes` (stopped by `timeout`), so the ticker raises an alert
# and there is something harmless to open a dossier on. Status messages
# (palette changes) hold alerts back, so the alert is shown before anything else.


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
Type "export PATH=$PWD/target/release:$PATH && (timeout 40 yes > /dev/null &) && clear"
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
Sleep 2s
Type "t"
Sleep 2s
Type "t"
Sleep 1.5s
Type "t"
Sleep 1s

# filter down to the `yes` and open its dossier (Enter keeps the filter, the
# next Enter opens the dossier, Escape closes it, "/" + Escape clears the filter)
Type "/"
Sleep 300ms
Type "yes"
Sleep 1s
Enter
Sleep 500ms
Enter
Sleep 4s
Escape
Sleep 800ms
Type "/"
Escape
Sleep 1s

# lineage: the manifest as a tree
Type "v"
Sleep 2.5s
Down 6
Sleep 2s
Type "v"
Sleep 1s

# operator manual
Type "?"
Sleep 3s
Type " "
Sleep 1s

# termination dialog, aborted
Type "K"
Sleep 2.5s
Type "n"
Sleep 1.5s

# do not press !
Type "!"
Sleep 4.5s
Escape
Sleep 2s

# power down
Type "q"
Sleep 2s
