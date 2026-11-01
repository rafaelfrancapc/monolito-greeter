#!/bin/sh
# Stage with DESTDIR=/tmp/monolito-package. Enabling a LightDM seat is explicit.
set -eu
cd "$(dirname "$0")/.."
destination=${DESTDIR:-}
test -x target/release/monolito-greeter || {
    echo 'Compile primeiro: cargo build --release --locked' >&2
    exit 1
}
install -Dm755 target/release/monolito-greeter "$destination/usr/bin/monolito-greeter"
install -Dm644 data/monolito-greeter.desktop "$destination/usr/share/xgreeters/monolito-greeter.desktop"
install -Dm644 data/50-monolito-greeter.conf "$destination/usr/share/doc/monolito-greeter/50-monolito-greeter.conf"
install -Dm644 data/config.ini "$destination/usr/share/doc/monolito-greeter/config.ini"
if [ ! -e "$destination/etc/monolito-greeter/config.ini" ]; then
    install -Dm644 data/config.ini "$destination/etc/monolito-greeter/config.ini"
fi
echo 'Arquivos instalados. A configuração ativa do LightDM não foi alterada.'
