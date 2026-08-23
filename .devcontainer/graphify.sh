#! /bin/bash

set -euo pipefail

sudo mkdir /workspaces/graphify
sudo chown musicalninja:musicalninja /workspaces/graphify
git clone --branch=feat/vibe-install https://github.com/xavierpestel-ai/graphify /workspaces/graphify
uv tool install /workspaces/graphify
