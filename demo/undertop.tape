# Records assets/undertop.gif. From the project root:
#   cargo build --release && vhs demo/undertop.tape
# then shrink it (vhs writes ~9 MB; this brings it to ~6 MB):
#   ffmpeg -i assets/undertop.gif -vf "fps=10,split[a][b];[a]palettegen=max_colors=64:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle" /tmp/u.gif && mv /tmp/u.gif assets/undertop.gif
#
# Careful when editing: in the termination dialog, Enter or Y sends SIGTERM to the
# selected process. This tape only leaves the dialog with "n".


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
Type "export PATH=$PWD/target/release:$PATH && clear"
Enter
Sleep 500ms
Show

Type "undertop"
Sleep 400ms
Enter

# boot sequence, then the console warming up
Sleep 6s

# the four palettes
Type "t"
Sleep 2.5s
Type "t"
Sleep 2.5s
Type "t"
Sleep 2.5s
Type "t"
Sleep 1.5s

# sort by memory, walk the manifest
Type "m"
Sleep 300ms
Type "g"
Sleep 1.5s
Down 4
Sleep 1.5s

# filter
Type "/"
Sleep 300ms
Type "undertop"
Sleep 2.0s
Escape
Sleep 1.0s

# operator manual
Type "?"
Sleep 3.0s
Type " "
Sleep 1.0s

# termination dialog, aborted
Type "K"
Sleep 3.0s
Type "n"
Sleep 2.0s

Type "q"
Sleep 1.0s
