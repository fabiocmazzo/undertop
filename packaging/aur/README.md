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

The release workflow does this by itself. Its `pkgbuilds` job runs after the release is published:

1. It runs [`update.sh`](update.sh) with the tag, which sets `pkgver`, resets `pkgrel` to 1, writes the
   new checksums with `updpkgsums` and regenerates the `.SRCINFO` files.
2. It builds `undertop-bin` and checks it with `namcap`.
3. It commits the result to `main` as `packaging/aur: update to vX.Y.Z`.

By hand, the same thing is `packaging/aur/update.sh 0.3.1`. To publish to the AUR, copy each `PKGBUILD`
and `.SRCINFO` to its AUR repository, then commit and push.

A packaging-only fix (same version) is a manual `pkgrel` bump followed by `update.sh` with the same version.
