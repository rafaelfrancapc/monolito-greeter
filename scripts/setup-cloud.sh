#!/bin/bash
# Unprivileged Debian 13 cloud bootstrap. Keep packages outside the checkout.
set -euo pipefail
source /etc/os-release
if [[ "$ID" != debian || "$VERSION_ID" != 13 ]]; then
    echo 'Este bootstrap é para Debian 13. Para Linux Mint, consulte README.md.' >&2
    exit 1
fi
native_prefix=/workspace/.monolito-native
mkdir -p "$native_prefix/apt/lists/partial" "$native_prefix/apt/archives/partial"
cat > "$native_prefix/apt/sources.list" <<'EOF'
deb https://deb.debian.org/debian trixie main
deb https://deb.debian.org/debian trixie-updates main
deb https://security.debian.org/debian-security trixie-security main
EOF
apt_options=(-o "Dir::Etc::sourcelist=$native_prefix/apt/sources.list" -o Dir::Etc::sourceparts=- -o "Dir::State::lists=$native_prefix/apt/lists" -o "Dir::Cache::archives=$native_prefix/apt/archives" -o Debug::NoLocking=true -o APT::Update::Error-Mode=any)
apt-get "${apt_options[@]}" update
apt-get "${apt_options[@]}" --download-only --no-install-recommends -y install cargo rustc rustfmt libgtk-4-dev liblightdm-gobject-dev xvfb xauth xdotool dbus-x11 fonts-dejavu-core
for package in "$native_prefix"/apt/archives/*.deb; do
    dpkg-deb -x "$package" "$native_prefix/root"
done
# Dev-package symlinks can refer to runtime packages already installed on the
# host (APT correctly doesn't download them again). Complete only those links.
multiarch=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
for link in "$native_prefix/root/usr/lib/$multiarch"/*.so; do
    if [[ -L "$link" && ! -e "$link" ]]; then
        target=$(readlink "$link")
        if [[ "$target" != */* && -e "/usr/lib/$multiarch/$target" ]]; then
            ln -s "/usr/lib/$multiarch/$target" "$(dirname "$link")/$target"
        fi
    fi
done
cat > "$native_prefix/env.sh" <<'EOF'
export MONOLITO_NATIVE_ROOT=/workspace/.monolito-native/root
export PATH="$MONOLITO_NATIVE_ROOT/usr/bin:$PATH"
export LD_LIBRARY_PATH="$MONOLITO_NATIVE_ROOT/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export LIBRARY_PATH="$MONOLITO_NATIVE_ROOT/usr/lib/x86_64-linux-gnu${LIBRARY_PATH:+:$LIBRARY_PATH}"
export PKG_CONFIG_PATH="$MONOLITO_NATIVE_ROOT/usr/lib/x86_64-linux-gnu/pkgconfig:$MONOLITO_NATIVE_ROOT/usr/share/pkgconfig"
export PKG_CONFIG_SYSROOT_DIR="$MONOLITO_NATIVE_ROOT"
export CARGO_HOME=/workspace/.monolito-native/cargo
export XDG_DATA_DIRS="$MONOLITO_NATIVE_ROOT/usr/share:/usr/local/share:/usr/share"
EOF
echo "Ative as ferramentas: source $native_prefix/env.sh"
