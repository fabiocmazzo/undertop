#!/bin/sh
# Installs undertop from a GitHub release, checking its SHA-256 first.
#
#   curl -fsSL https://raw.githubusercontent.com/fabiocmazzo/undertop/main/install.sh | sh
#
# Environment:
#   UNDERTOP_VERSION      a release tag such as v0.3.0 (default: the latest release)
#   UNDERTOP_INSTALL_DIR  where the binary goes (default: ~/.local/bin; no root needed)
#
# To uninstall: rm ~/.local/bin/undertop (or wherever it was installed).

set -eu

REPO="fabiocmazzo/undertop"

say() { printf 'undertop-install: %s\n' "$*"; }
die() {
    say "$*" >&2
    exit 1
}
need() { command -v "$1" >/dev/null 2>&1 || die "this script needs '$1'"; }

# Everything runs from main, so a download cut short never runs half a script.
main() {
    dir="${UNDERTOP_INSTALL_DIR:-$HOME/.local/bin}"

    [ "$(uname -s)" = Linux ] || die "undertop only runs on Linux (this is $(uname -s))"
    case "$(uname -m)" in
    x86_64 | amd64) arch=x86_64 ;;
    aarch64 | arm64) arch=aarch64 ;;
    *) die "no prebuilt binary for $(uname -m); build it with: cargo install --git https://github.com/$REPO" ;;
    esac

    need curl
    need tar
    need mktemp
    if command -v sha256sum >/dev/null 2>&1; then
        sum="sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
        sum="shasum -a 256"
    else
        die "this script needs 'sha256sum' or 'shasum' to check the download"
    fi

    version="${UNDERTOP_VERSION:-}"
    if [ -z "$version" ]; then
        # the latest-release page redirects to its tag: no API, no rate limit
        latest=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest") ||
            die "could not reach GitHub"
        version=${latest##*/}
    fi
    case "$version" in
    v*) ;;
    *) version="v$version" ;;
    esac
    case "$version" in
    v[0-9]*) ;;
    *) die "could not tell which release to install (got '$version')" ;;
    esac

    name="undertop-$version-$arch-linux"
    base="https://github.com/$REPO/releases/download/$version"
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    trap 'exit 130' INT TERM

    say "downloading $name"
    curl -fsSL -o "$tmp/$name.tar.gz" "$base/$name.tar.gz" ||
        die "download failed: $base/$name.tar.gz (is $version a release?)"
    curl -fsSL -o "$tmp/$name.tar.gz.sha256" "$base/$name.tar.gz.sha256" ||
        die "download failed: $base/$name.tar.gz.sha256"
    (cd "$tmp" && $sum -c "$name.tar.gz.sha256" >/dev/null 2>&1) ||
        die "checksum mismatch, nothing was installed"
    say "checksum verified"

    tar xzf "$tmp/$name.tar.gz" -C "$tmp"
    mkdir -p "$dir"
    # copy beside the target, then rename: a running undertop is never overwritten in place
    cp "$tmp/$name/undertop" "$dir/.undertop.new"
    chmod 755 "$dir/.undertop.new"
    mv -f "$dir/.undertop.new" "$dir/undertop"
    say "installed $version as $dir/undertop"

    case ":$PATH:" in
    *":$dir:"*) say "start it with: undertop" ;;
    *)
        say "$dir is not in your PATH. Add this line to your shell's startup file:"
        say "  export PATH=\"$dir:\$PATH\""
        say "or start it with: $dir/undertop"
        ;;
    esac
}

main "$@"
