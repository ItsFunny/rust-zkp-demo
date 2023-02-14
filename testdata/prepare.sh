#!/bin/bash

git submodule init

git submodule update


DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
REPO_DIR=$DIR/".."
SETUP_DIR=$REPO_DIR"/testdata/plonk/setup"
SETUP_MK=$SETUP_DIR"/setup_2^20.key"
DOWNLOAD_SETUP_FROM_REMOTE=false
PLONKIT_BIN=$REPO_DIR"/plonkit/target/release/plonkit"

echo "Step0: check for necessary dependencies: node,npm,axel"
PKG_PATH=""
PKG_PATH=$(command -v npm)
echo Checking for npm
if [ -z "$PKG_PATH" ]; then
  echo "npm not found. Installing nvm & npm & node."
  source <(curl -s https://raw.githubusercontent.com/nvm-sh/nvm/v0.37.2/install.sh)
else
  echo npm exists at $PKG_PATH
fi
PKG_PATH=""
PKG_PATH=$(command -v axel)
if ($DOWNLOAD_SETUP_FROM_REMOTE & [ -z "$PKG_PATH" ]) ; then
  echo Checking for axel
  echo "axel not found. Installing axel."
  sudo apt-get --yes install axel
elif [ ! -z "$PKG_PATH" ] ; then
  echo axel exists at $PKG_PATH
fi

echo "Step1: build plonkit binary"
cd ../plonkit
cargo build --release

if ([ ! -f $SETUP_MK ] & $DOWNLOAD_SETUP_FROM_REMOTE); then
  # It is the aztec ignition trusted setup key file. Thanks to matter-labs/zksync/infrastructure/zk/src/run/run.ts
  axel -ac https://universal-setup.ams3.digitaloceanspaces.com/setup_2^20.key -o $SETUP_MK || true
elif [ ! -f $SETUP_MK ] ; then
    echo "generate setup files"
    $PLONKIT_BIN setup --power 20 --srs_monomial_form $SETUP_MK --overwrite
fi

