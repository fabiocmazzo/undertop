# AUR packages

| Package | What it installs |
| --- | --- |
| `undertop` | builds the tagged source with cargo (x86_64, aarch64) |
| `undertop-bin` | the prebuilt x86_64 binary from the GitHub release |

Both were built with `makepkg` and checked with `namcap`. The only warning left is the usual "unused shared
library ld-linux" that every Rust binary gets.

Until they are on the AUR, [`install-arch.sh`](../../install-arch.sh) builds and installs them with
`makepkg -si` straight from this directory, so the `PKGBUILD` here must always match the latest release.

## First publication

Each package is its own git repository on the AUR. You need an AUR account with your SSH public key.

```sh
git clone ssh://aur@aur.archlinux.org/undertop.git /tmp/aur-undertop
cp packaging/aur/undertop/PKGBUILD packaging/aur/undertop/.SRCINFO /tmp/aur-undertop/
cd /tmp/aur-undertop && git add PKGBUILD .SRCINFO && git commit -m "undertop 0.3.0" && git push
```

Then do the same for `undertop-bin`.

## After each release

1. Set `pkgver` to the new version and `pkgrel=1`.
2. Run `updpkgsums` to fetch the new tarball and write its checksum.
3. Run `makepkg --printsrcinfo > .SRCINFO`.
4. Run `makepkg -f && namcap PKGBUILD *.pkg.tar.zst` to try it.
5. Copy `PKGBUILD` and `.SRCINFO` to the AUR repository, then commit and push.

Do this for both packages.
