#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

# Check for root privileges
if [ "$EUID" -ne 0 ]; then
  echo "❌ Error: Please run as root (e.g., sudo bash install_all.sh)"
  exit 1
fi

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "🚀 Starting automated deployment for RK1828..."

# Determine the actual non-root user calling sudo (for Rust user-level install)
REAL_USER=${SUDO_USER:-$USER}
USER_HOME=$(eval echo "~$REAL_USER")

# Total steps updated to 5
TOTAL_STEPS=5

# ==========================================
# Replace System APT Sources
# ==========================================
if [ -f "$BASE_DIR/sources.list" ]; then
    echo "📦 [1/$TOTAL_STEPS] Configuring APT sources..."
    cp "$BASE_DIR/sources.list" /etc/apt/sources.list
    echo "✓ APT sources updated successfully."
else
    echo "⚠️ Warning: sources.list not found. Skipping repository configuration."
fi

# ==========================================
# Install Proxy & Configure Systemd Service
# ==========================================
echo "⚙️ [2/$TOTAL_STEPS] Setting up rknn3_transfer_proxy and systemd service..."
if [ -f "$BASE_DIR/proxy/rknn3_transfer_proxy" ]; then
    cp "$BASE_DIR/proxy/rknn3_transfer_proxy" /usr/bin/
    chmod +x /usr/bin/rknn3_transfer_proxy
    chown root:root /usr/bin/rknn3_transfer_proxy

    # Create systemd service configuration
    cat << 'SERVICE_EOF' > /etc/systemd/system/rknn3-transfer-proxy.service
[Unit]
Description=RKNN3 Transfer Proxy DeviceManager Service
After=network.target local-fs.target

[Service]
Type=simple
ExecStart=/usr/bin/rknn3_transfer_proxy
Restart=always
RestartSec=3
WorkingDirectory=/tmp

[Install]
WantedBy=multi-user.target
SERVICE_EOF

    systemctl daemon-reload
    systemctl enable rknn3-transfer-proxy.service
    echo "✓ Proxy service configured and set to enable on boot."
else
    echo "❌ Error: rknn3_transfer_proxy not found in proxy/ directory."
    exit 1
fi

# ==========================================
# Deploy Shared Libraries (.so)
# ==========================================
echo "📚 [3/$TOTAL_STEPS] Installing shared libraries..."
# Detect standard 64-bit library directory automatically
TARGET_LIB_DIR="/usr/lib/aarch64-linux-gnu"
if [ ! -d "$TARGET_LIB_DIR" ]; then
    if [ -d "/usr/lib64" ]; then
        TARGET_LIB_DIR="/usr/lib64"
    else
        TARGET_LIB_DIR="/usr/lib"
    fi
fi

echo "ℹ️ Selected target library directory: $TARGET_LIB_DIR"

if [ -d "$BASE_DIR/lib" ]; then
    cp "$BASE_DIR/lib"/librknn3_api*.so "$TARGET_LIB_DIR/"
    chmod 755 "$TARGET_LIB_DIR"/librknn3_api*.so
    chown root:root "$TARGET_LIB_DIR"/librknn3_api*.so

    # Add directory to ldconfig path and update cache
    echo "$TARGET_LIB_DIR" > /etc/ld.so.conf.d/rknn3.conf
    ldconfig
    echo "✓ Shared libraries deployed and linker cache refreshed."
else
    echo "❌ Error: lib/ directory not found."
    exit 1
fi

# ==========================================
# Install Rust Toolchain & OpenCV Dependencies
# ==========================================
echo "🦀 [4/$TOTAL_STEPS] Installing Rust Toolchain & OpenCV Build Dependencies..."

# 1. Update apt and install OpenCV, pkg-config, cmake, build tools
echo "🔄 Updating repositories and installing build dependencies..."
apt update && apt install -y pkg-config cmake build-essential libopencv-dev clang curl || {
    echo "⚠️ Apt installation encountered an issue. Attempting to proceed..."
}

# 2. Configure Rust Mirror Environment Variables
export RUSTUP_DIST_SERVER="https://rsproxy.cn"
export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
export RUSTUP_HOME="$USER_HOME/.rustup"
export CARGO_HOME="$USER_HOME/.cargo"

# 3. Download and Install Rust non-interactively
echo "🚀 Downloading and installing Rust via rsproxy mirror..."
curl --proto '=https' --tlsv1.2 -sSf https://rsproxy.cn/rustup-init.sh | sh -s -- -y --default-toolchain stable

# Adjust ownership of .cargo and .rustup so the regular user owns them
chown -R "$REAL_USER:$REAL_USER" "$USER_HOME/.cargo" "$USER_HOME/.rustup"

# 4. Configure Cargo Sparse Index Mirror
mkdir -p "$USER_HOME/.cargo"
cat << 'CARGO_CONF_EOF' > "$USER_HOME/.cargo/config.toml"
[source.crates-io]
replace-with = 'rsproxy-sparse'

[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[net]
git-fetch-with-cli = true
CARGO_CONF_EOF

chown "$REAL_USER:$REAL_USER" "$USER_HOME/.cargo/config.toml"

# 5. Global Environment Variable Script (Applies to all users)
cat << 'ENV_EOF' > /etc/profile.d/rust.sh
export RUSTUP_DIST_SERVER="https://rsproxy.cn"
export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
export PATH="$HOME/.cargo/bin:$PATH"
ENV_EOF
chmod +x /etc/profile.d/rust.sh

echo "✓ Rust and OpenCV dependencies installed successfully."

# ==========================================
# Compile pcie-rkep Driver & Install Package
# ==========================================
echo "🔧 [5/$TOTAL_STEPS] Compiling drivers and installing package..."
cd "$BASE_DIR/driver"

# Compile kernel module
if [ -d "pcie-rkep" ]; then
    cd pcie-rkep
    make
    mkdir -p /usr/lib/modules
    cp ./pcie-rkep.ko /usr/lib/modules/pcie-rkep.ko
    cd ..
    echo "✓ pcie-rkep driver compiled and copied successfully."
else
    echo "❌ Error: driver/pcie-rkep directory not found."
    exit 1
fi

# Extract and execute the RKNN installer script
INSTALLER_TGZ=$(ls rknn3_rk182x_m2_*_installer_arm64.tgz 2>/devnull | head -n 1)
if [ -n "$INSTALLER_TGZ" ]; then
    echo "📦 Extracting installer package: $INSTALLER_TGZ"
    tar -zxvf "$INSTALLER_TGZ"
    bash install.sh
    sync
    echo "✓ RKNN3 installer package executed successfully."
else
    echo "❌ Error: Installer archive (rknn3_rk182x_m2_*_installer_arm64.tgz) not found in driver/ directory."
    exit 1
fi

echo "=========================================="
echo "Deployment finished! Rebooting in 5 seconds..."
echo "=========================================="
sleep 5
reboot
