#!/bin/sh
# Installs undertop on Arch Linux (and derivatives) as a pacman package, from the PKGBUILDs in
# this repository: what the AUR would do, until the packages are published there.
#
#   curl -fsSL https://raw.githubusercontent.com/fabiocmazzo/undertop/main/install-arch.sh | sh
#
# Environment:
#   UNDERTOP_PKG      undertop-bin (default: the release binary) or undertop (built from source)
#   UNDERTOP_INSTALL  0 builds the package and prints where it is, without installing it
#
# pacman then knows about it: `pacman -Qi undertop-bin`, and `sudo pacman -R undertop-bin` to remove.

set -eu

RAW="https://raw.githubusercontent.com/fabiocmazzo/undertop/main"

say() { printf 'undertop-install: %s\n' "$*"; }
die() {
    say "$*" >&2
    exit 1
}
need() { command -v "$1" >/dev/null 2>&1 || die "$2"; }

# Everything runs from main, so a download cut short never runs half a script.
main() {
    pkg="${UNDERTOP_PKG:-undertop-bin}"
    install="${UNDERTOP_INSTALL:-1}"
    case "$pkg" in
    undertop | undertop-bin) ;;
    *) die "UNDERTOP_PKG must be undertop or undertop-bin (got '$pkg')" ;;
    esac

    need pacman "pacman not found: this script is for Arch Linux; elsewhere use install.sh"
    need makepkg "makepkg not found: install it with: sudo pacman -S --needed base-devel"
    need curl "this script needs curl"
    [ "$(id -u)" -ne 0 ] || die "run it as your user, not root: makepkg refuses to build as root and asks for sudo when it installs"
    if [ "$pkg" = undertop ]; then
        need cargo "building from source needs cargo: sudo pacman -S rust (or use the default undertop-bin)"
    fi

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    trap 'exit 130' INT TERM

    curl -fsSL -o "$tmp/PKGBUILD" "$RAW/packaging/aur/$pkg/PKGBUILD" ||
        die "could not download the PKGBUILD for $pkg"
    version=$(sed -n 's/^pkgver=//p' "$tmp/PKGBUILD")
    say "building $pkg $version with makepkg"

    # With `curl | sh` the script's stdin is the download, so pacman's questions and sudo's password
    # go through the terminal; without one, nothing can be asked and pacman takes its defaults.
    if [ "$install" = 0 ]; then
        (cd "$tmp" && makepkg -f)
        mkdir -p "${XDG_CACHE_HOME:-$HOME/.cache}/undertop"
        cp "$tmp"/*.pkg.tar.* "${XDG_CACHE_HOME:-$HOME/.cache}/undertop/"
        say "built, not installed: $(ls "${XDG_CACHE_HOME:-$HOME/.cache}/undertop/$pkg-$version"-*.pkg.tar.*)"
        say "install it with: sudo pacman -U <that file>"
        return
    fi
    if [ -r /dev/tty ] && (: </dev/tty) 2>/dev/null; then
        (cd "$tmp" && makepkg -si </dev/tty)
    else
        (cd "$tmp" && makepkg -si --noconfirm)
    fi
    say "installed $pkg $version as /usr/bin/undertop"

    # a copy from install.sh in ~/.local/bin usually comes first in PATH and would hide this one
    local_copy="$HOME/.local/bin/undertop"
    if [ -e "$local_copy" ]; then
        say "note: $local_copy (from install.sh) is still there and may be found first."
        say "remove it to use the pacman one: rm $local_copy"
    fi
    say "start it with: undertop"
}

main "$@"
