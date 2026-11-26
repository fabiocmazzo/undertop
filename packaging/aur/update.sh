#!/bin/sh
# Points both PKGBUILDs at a release: pkgver, pkgrel=1, checksums and .SRCINFO.
#   packaging/aur/update.sh 0.3.1        (or v0.3.1)
# Needs makepkg and updpkgsums (pacman-contrib); run it as a normal user, after the GitHub release
# exists, since undertop-bin's checksum is that of the release tarball. The release workflow runs it.
set -eu

[ $# -eq 1 ] || {
    echo "usage: $0 <version>" >&2
    exit 2
}
version=${1#v}
case "$version" in
[0-9]*.[0-9]*.[0-9]*) ;;
*)
    echo "not a version: $1" >&2
    exit 2
    ;;
esac

cd "$(dirname "$0")"
for p in undertop undertop-bin; do
    (
        cd "$p"
        old=$(sed -n 's/^pkgver=//p' PKGBUILD)
        sed -i "s/^pkgver=.*/pkgver=$version/" PKGBUILD
        # a new version starts over at pkgrel 1; the same version keeps its pkgrel
        [ "$old" = "$version" ] || sed -i 's/^pkgrel=.*/pkgrel=1/' PKGBUILD
        updpkgsums >/dev/null
        makepkg --printsrcinfo >.SRCINFO
        rm -f ./*.tar.gz
        echo "$p: $old -> $version"
    )
done
