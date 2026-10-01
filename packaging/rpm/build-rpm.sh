#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: build-rpm.sh <version> [output-dir] [source-tarball]}"
output_dir="${2:-dist}"
source_tarball="${3:-$output_dir/kwybars-${version}-x86_64-linux.tar.gz}"
release="${RPM_RELEASE:-1}"
architecture="${RPM_ARCHITECTURE:-x86_64}"
work_root="$(mktemp -d)"
rpmbuild_root="$work_root/rpmbuild"

cleanup() {
  rm -rf "$work_root"
}
trap cleanup EXIT

require_file() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
}

require_file "$source_tarball"

if ! command -v rpmbuild >/dev/null 2>&1; then
  echo "missing required command: rpmbuild" >&2
  exit 1
fi

source_basename="$(basename "$source_tarball")"
source_dirname="$(tar -tzf "$source_tarball" | awk -F/ 'NR == 1 { print $1 }')"

if [ -z "$source_dirname" ]; then
  echo "failed to determine source directory from $source_tarball" >&2
  exit 1
fi

rm -rf "$rpmbuild_root"
mkdir -p \
  "$rpmbuild_root/BUILD" \
  "$rpmbuild_root/BUILDROOT" \
  "$rpmbuild_root/RPMS" \
  "$rpmbuild_root/SOURCES" \
  "$rpmbuild_root/SPECS" \
  "$rpmbuild_root/SRPMS" \
  "$output_dir"

cp "$source_tarball" "$rpmbuild_root/SOURCES/$source_basename"

cat >"$rpmbuild_root/SPECS/kwybars.spec" <<SPEC
%global debug_package %{nil}

Name:           kwybars
Version:        ${version}
Release:        ${release}%{?dist}
Summary:        Desktop audio visualizer overlay for Wayland
License:        GPL-3.0-or-later
URL:            https://github.com/naurissteins/Kwybars
Source0:        ${source_basename}
BuildArch:      ${architecture}
Requires:       pipewire-libs
Recommends:     pipewire

%description
Kwybars draws real-time audio bars on the desktop of Wayland compositors
that support layer-shell. It reads audio from PipeWire and hides itself
while nothing plays.

%prep
%setup -q -n ${source_dirname}

%build

%install
install -d %{buildroot}%{_bindir}
install -m755 kwybars %{buildroot}%{_bindir}/kwybars

install -d %{buildroot}/usr/lib/systemd/user
install -m644 kwybars.service %{buildroot}/usr/lib/systemd/user/kwybars.service

install -d %{buildroot}%{_datadir}/kwybars/themes
install -m644 themes/*.toml %{buildroot}%{_datadir}/kwybars/themes/

install -d %{buildroot}%{_docdir}/%{name}/examples
install -m644 examples/*.toml %{buildroot}%{_docdir}/%{name}/examples/
install -m644 README.md %{buildroot}%{_docdir}/%{name}/README.md

install -d %{buildroot}%{_mandir}/man1
install -m644 share/man/man1/kwybars.1 %{buildroot}%{_mandir}/man1/kwybars.1

install -d %{buildroot}%{_licensedir}/%{name}
install -m644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE

%files
%license %{_licensedir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/README.md
%doc %{_docdir}/%{name}/examples
%{_mandir}/man1/kwybars.1*
%{_bindir}/kwybars
/usr/lib/systemd/user/kwybars.service
%{_datadir}/kwybars/themes
SPEC

rpmbuild --define "_topdir $rpmbuild_root" -bb "$rpmbuild_root/SPECS/kwybars.spec"

find "$rpmbuild_root/RPMS" -type f -name '*.rpm' -exec cp {} "$output_dir/" \;
find "$output_dir" -maxdepth 1 -type f -name 'kwybars-*.rpm' -print
