cd "$(dirname "$(realpath "$0")")"
cargo build --release
sudo install -m 0644 caps-layout-switcher.service /etc/systemd/system/caps-layout-switcher.service
sudo systemctl daemon-reload
sudo systemctl enable --now caps-layout-switcher.service
sudo systemctl status caps-layout-switcher.service
