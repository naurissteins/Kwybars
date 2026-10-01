#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: build-deb.sh <version> [output-dir]}"
output_dir="${2:-dist}"
revision="${DEB_REVISION:-1}"
architecture="${DEB_ARCHITECTURE:-amd64}"
package_version="${version}-${revision}"

if [[ ! "$package_version" =~ ^[0-9] ]]; then
  echo "invalid Debian package version: $package_version" >&2
  echo "Debian versions must start with a digit, for example 0.1.9-1 or 0.1.9~manual-1." >&2
  exit 1
fi

package_root="$(mktemp -d)"
staging="$package_root/kwybars_${package_version}_${architecture}"
deb_path="$output_dir/kwybars_${package_version}_${architecture}.deb"

cleanup() {
  rm -rf "$package_root"
}
trap cleanup EXIT

require_file() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
}

require_file target/release/kwybars
require_file assets/examples/config.toml
require_file assets/systemd/kwybars.service
require_file LICENSE
require_file README.md
require_file docs/man/kwybars.1

for tool in dpkg-deb objdump; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required command: $tool" >&2
    exit 1
  fi
done

# the newest glibc symbol the binary uses, so apt refuses an older system
# instead of the binary failing to start there
glibc_version="$(
  objdump -T target/release/kwybars |
    grep -o 'GLIBC_[0-9][0-9.]*' |
    sed 's/GLIBC_//' |
    sort -V |
    tail -n 1
)"
if [ -z "$glibc_version" ]; then
  echo "could not read the glibc version target/release/kwybars needs" >&2
  exit 1
fi

rm -rf "$staging" "$deb_path"
mkdir -p \
  "$staging/DEBIAN" \
  "$staging/usr/bin" \
  "$staging/usr/lib/systemd/user" \
  "$staging/usr/share/doc/kwybars/examples" \
  "$staging/usr/share/kwybars/themes" \
  "$staging/usr/share/man/man1" \
  "$output_dir"

install -m755 target/release/kwybars "$staging/usr/bin/kwybars"

install -m644 assets/systemd/kwybars.service \
  "$staging/usr/lib/systemd/user/kwybars.service"
install -m644 assets/examples/*.toml "$staging/usr/share/doc/kwybars/examples/"
install -m644 assets/themes/*.toml "$staging/usr/share/kwybars/themes/"
install -m644 docs/man/kwybars.1 "$staging/usr/share/man/man1/kwybars.1"
gzip -9n "$staging/usr/share/man/man1/kwybars.1"
install -m644 README.md "$staging/usr/share/doc/kwybars/README.md"
install -m644 LICENSE "$staging/usr/share/doc/kwybars/copyright"

installed_size="$(du -sk "$staging" | awk '{print $1}')"
cat >"$staging/DEBIAN/control" <<CONTROL
Package: kwybars
Version: $package_version
Section: sound
Priority: optional
Architecture: $architecture
Maintainer: Nauris Steins <me@naurissteins.com>
Installed-Size: $installed_size
Depends: libc6 (>= $glibc_version), libgcc-s1, libpipewire-0.3-0t64 | libpipewire-0.3-0
Recommends: pipewire
Homepage: https://github.com/naurissteins/Kwybars
Description: Desktop audio visualizer overlay for Wayland
 Kwybars draws real-time audio bars on the desktop of Wayland compositors
 that support layer-shell. It reads audio from PipeWire and hides itself
 while nothing plays.
CONTROL

dpkg-deb --build --root-owner-group "$staging" "$deb_path"
echo "$deb_path"
