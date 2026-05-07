#!/usr/bin/env bash
# scripts/devnet-up.sh — Start Krax auxiliary devnet services.
#
# Phase 0 placeholder: this script does nothing yet.
# Auxiliary services (Blockscout, Prometheus, Grafana, and eventually Anvil)
# land in docker-compose.yml as each phase introduces them. When a service
# is added there, this script gains the corresponding `docker compose up` call.
#
# For now, Anvil runs natively: open a terminal tab and run `anvil`.
# See docker-compose.yml for the full services roadmap.
set -euo pipefail

echo "devnet-up: no services configured yet (Phase 0 placeholder)"
echo "Run Anvil natively: anvil"
exit 0
