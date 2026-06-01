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

require_file target/release/kwybars-daemon
require_file target/release/kwybars-overlay
require_file target/release/kwybarsctl
require_file assets/examples/config.toml
require_file assets/systemd/kwybars-daemon.service
require_file LICENSE
require_file README.md
require_file docs/man/kwybars.1

if ! command -v dpkg-deb >/dev/null 2>&1; then
  echo "missing required command: dpkg-deb" >&2
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

install -m755 target/release/kwybars-daemon "$staging/usr/bin/kwybars-daemon"
install -m755 target/release/kwybars-overlay "$staging/usr/bin/kwybars-overlay"
install -m755 target/release/kwybarsctl "$staging/usr/bin/kwybarsctl"

install -m644 assets/systemd/kwybars-daemon.service \
  "$staging/usr/lib/systemd/user/kwybars-daemon.service"
install -m644 assets/examples/config.toml "$staging/usr/share/doc/kwybars/examples/config.toml"
install -m644 assets/themes/*.toml "$staging/usr/share/kwybars/themes/"
install -m644 docs/man/*.1 "$staging/usr/share/man/man1/"
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
Depends: libc6, libgtk-4-1, libgdk-pixbuf-2.0-0, libpipewire-0.3-0, cava, libgtk4-layer-shell0 | gtk4-layer-shell
Recommends: libnotify-bin, systemd
Homepage: https://github.com/naurissteins/Kwybars
Description: Desktop audio visualizer overlay for Wayland
 Kwybars renders real-time audio bars as a transparent GTK4 layer-shell
 overlay on Wayland desktops.
CONTROL

dpkg-deb --build --root-owner-group "$staging" "$deb_path"
echo "$deb_path"
