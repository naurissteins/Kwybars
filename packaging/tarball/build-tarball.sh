#!/usr/bin/env bash
# Collects the release binary and everything a package installs into
# <output-dir>/kwybars-<version>-x86_64-linux.tar.gz.
set -euo pipefail

version="${1:?usage: build-tarball.sh <version> [output-dir]}"
output_dir="${2:-dist}"
name="kwybars-${version}-x86_64-linux"
dist_dir="$output_dir/$name"

for path in \
  target/release/kwybars \
  assets/systemd/kwybars.service \
  assets/examples/config.toml \
  docs/man/kwybars.1 \
  README.md \
  LICENSE; do
  if [ ! -f "$path" ]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
done

rm -rf "$dist_dir"
mkdir -p \
  "$dist_dir/examples" \
  "$dist_dir/themes" \
  "$dist_dir/share/man/man1"

install -m755 target/release/kwybars "$dist_dir/kwybars"
install -m644 assets/systemd/kwybars.service "$dist_dir/kwybars.service"
install -m644 assets/examples/*.toml "$dist_dir/examples/"
install -m644 assets/themes/*.toml "$dist_dir/themes/"
install -m644 docs/man/kwybars.1 "$dist_dir/share/man/man1/kwybars.1"
install -m644 README.md "$dist_dir/README.md"
install -m644 LICENSE "$dist_dir/LICENSE"

tar -C "$output_dir" -czf "$output_dir/$name.tar.gz" "$name"
rm -rf "$dist_dir"
echo "$output_dir/$name.tar.gz"
