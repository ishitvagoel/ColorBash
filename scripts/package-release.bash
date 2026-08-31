#!/usr/bin/env bash
# Package a built `mbx` binary the same way `.github/workflows/release.yml`
# does: tarball + sha256, with README and both license texts. Does not push a
# tag or create a GitHub release — those stay maintainer-gated (REL-001).
set -euo pipefail

usage() {
    printf 'usage: %s VERSION TARGET [BIN]\n' "${0##*/}" >&2
    exit 2
}

(($# == 2 || $# == 3)) || usage

version=$1
target=$2
bin=${3:-target/release/mbx}
root=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd -P)
outdir=${MBX_PACKAGE_OUT:-$root}

cd "$root"
[[ -x $bin ]] || {
    printf 'missing executable: %s\n' "$bin" >&2
    exit 1
}
for required in README.md LICENSE-MIT LICENSE-APACHE; do
    [[ -f $required ]] || {
        printf 'missing %s (M-068)\n' "$required" >&2
        exit 1
    }
done

stage="mbx-${version}-${target}"
mkdir -p "$outdir"
(
    cd "$outdir"
    rm -rf "$stage" "${stage}.tar.gz" "${stage}.tar.gz.sha256"
    mkdir -p "$stage"
)
cp "$bin" "$outdir/$stage/mbx"
cp README.md LICENSE-MIT LICENSE-APACHE "$outdir/$stage/"
tar -C "$outdir" -czf "$outdir/${stage}.tar.gz" "$stage"
# Checksum file contains only the tarball basename so it is valid after
# download, matching the workflow's `sha256sum` from the build cwd.
(
    cd "$outdir"
    sha256sum "${stage}.tar.gz" >"${stage}.tar.gz.sha256"
)
printf '%s\n' "$outdir/${stage}.tar.gz"
printf '%s\n' "$outdir/${stage}.tar.gz.sha256"
