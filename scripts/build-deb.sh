#!/usr/bin/env bash
#
# Builds an Ubuntu 26.04 package containing the Waybar module.
#
# Usage: scripts/build-deb.sh [--no-build]
#
# The package is written to target/debian/. Pass --no-build to package an
# existing target/release/libniri_taskbar.so without running cargo.
#
# Library dependencies are taken from the build host, so this must be run on
# Ubuntu 26.04 for the package to install cleanly there.

set -euo pipefail

cd "$(dirname "$0")/.."

PACKAGE=waybar-niri-taskbar
LIB=libniri_taskbar.so
INSTALL_DIR=usr/lib/waybar
UBUNTU_RELEASE=26.04

# shellcheck source=/dev/null
. /etc/os-release
if [[ "${ID:-}" != ubuntu || "${VERSION_ID:-}" != "$UBUNTU_RELEASE" ]]; then
    echo "error: this script must be run on Ubuntu $UBUNTU_RELEASE" >&2
    exit 1
fi

for tool in dpkg-deb dpkg-shlibdeps; do
    if ! command -v "$tool" >/dev/null; then
        echo "error: $tool not found; install dpkg-dev" >&2
        exit 1
    fi
done

if [[ "${1:-}" != "--no-build" ]]; then
    cargo build --release --locked
fi

if [[ ! -f "target/release/$LIB" ]]; then
    echo "error: target/release/$LIB not found" >&2
    exit 1
fi

field() {
    sed -n "s/^$1 *= *\"\(.*\)\"/\1/p" Cargo.toml | head -n1
}

VERSION="$(field version)-1~ubuntu$UBUNTU_RELEASE"
DESCRIPTION=$(field description)
HOMEPAGE=$(field repository)
ARCH=$(dpkg --print-architecture)
MAINTAINER="${DEBFULLNAME:-$(git config user.name)} <${DEBEMAIL:-$(git config user.email)}>"

OUT_DIR=target/debian
STAGING="$OUT_DIR/${PACKAGE}_${VERSION}_${ARCH}"
DEB="$STAGING.deb"

rm -rf "$STAGING"
install -Dm644 "target/release/$LIB" "$STAGING/$INSTALL_DIR/$LIB"
strip --strip-unneeded "$STAGING/$INSTALL_DIR/$LIB"
install -Dm644 README.md "$STAGING/usr/share/doc/$PACKAGE/README.md"
install -Dm644 LICENSE "$STAGING/usr/share/doc/$PACKAGE/copyright"

# dpkg-shlibdeps insists on running from a directory containing
# debian/control, so give it a throwaway one.
SHLIBDEPS_DIR=$(mktemp -d)
trap 'rm -rf "$SHLIBDEPS_DIR"' EXIT
mkdir "$SHLIBDEPS_DIR/debian"
printf 'Source: %s\n\nPackage: %s\nArchitecture: any\n' "$PACKAGE" "$PACKAGE" \
    >"$SHLIBDEPS_DIR/debian/control"
LIB_PATH="$PWD/$STAGING/$INSTALL_DIR/$LIB"
DEPENDS=$(cd "$SHLIBDEPS_DIR" && dpkg-shlibdeps -O "$LIB_PATH" 2>/dev/null |
    sed -n 's/^shlibs:Depends=//p')

INSTALLED_SIZE=$(du -sk --exclude=DEBIAN "$STAGING" | cut -f1)

mkdir -p "$STAGING/DEBIAN"
cat >"$STAGING/DEBIAN/control" <<EOF
Package: $PACKAGE
Version: $VERSION
Architecture: $ARCH
Maintainer: $MAINTAINER
Installed-Size: $INSTALLED_SIZE
Depends: ${DEPENDS:+$DEPENDS, }waybar
Section: x11
Priority: optional
Homepage: $HOMEPAGE
Description: $DESCRIPTION
 Waybar CFFI module providing a taskbar that orders windows to match the
 Niri layout. Point the Waybar cffi/niri-taskbar module_path at
 /$INSTALL_DIR/$LIB.
EOF

dpkg-deb --root-owner-group --build "$STAGING" "$DEB" >/dev/null
rm -rf "$STAGING"

echo "Built $DEB"
