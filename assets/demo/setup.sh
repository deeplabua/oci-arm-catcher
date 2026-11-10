#!/usr/bin/env bash
# Sourced (hidden) by the .tape files: a throwaway OCI profile, a mock OCI API on
# localhost, and a clean prompt. Nothing here touches a real account.
#
# Record from the repo root:
#   cargo build --release
#   vhs assets/demo/demo.tape && vhs assets/demo/discover.tape

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$DEMO_DIR/../.." && pwd)"
WORK="$(mktemp -d)"

cat > "$WORK/oci-config" <<EOF
[DEFAULT]
user=ocid1.user.oc1..aaaademo
fingerprint=12:34:56:78:9a:bc:de:f0:12:34:56:78:9a:bc:de:f0
tenancy=ocid1.tenancy.oc1..aaaademotenancy
region=us-ashburn-1
key_file=$DEMO_DIR/test-key-pkcs8.pem
EOF
mkdir -p "$WORK/.ssh"
echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAADEMO demo@laptop" > "$WORK/.ssh/id_ed25519.pub"

cat > "$WORK/.env" <<EOF
COMPARTMENT_ID="ocid1.tenancy.oc1..aaaademotenancy"
AVAILABILITY_DOMAINS="Abcd:US-ASHBURN-AD-1,Abcd:US-ASHBURN-AD-2,Abcd:US-ASHBURN-AD-3"
SUBNET_ID="ocid1.subnet.oc1.us-ashburn-1.aaaademopublicsubnet"
IMAGE_ID="ocid1.image.oc1.us-ashburn-1.aaaademoubuntu2404arm"
SSH_KEY_FILE="$WORK/.ssh/id_ed25519.pub"
OCPUS=2
MEMORY_GB=12
RETRY_INTERVAL=5
EOF

python3 "$DEMO_DIR/mock-oci.py" 8799 &
MOCK_PID=$!
trap 'kill $MOCK_PID 2>/dev/null; rm -rf "$WORK"' EXIT

export OCI_CONFIG_FILE="$WORK/oci-config"
export OCI_ARM_CATCHER_ENDPOINT="http://127.0.0.1:8799"
export PATH="$REPO/target/release:$PATH"
export HOME="$WORK"
cd "$WORK" || exit 1
export PS1='\[\e[32m\]$\[\e[0m\] '
sleep 0.5
clear
