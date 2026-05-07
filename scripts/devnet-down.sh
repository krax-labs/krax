#!/usr/bin/env bash
# scripts/devnet-down.sh — Stop Krax auxiliary devnet services.
#
# Phase 0 placeholder: this script does nothing yet.
# Auxiliary services (Blockscout, Prometheus, Grafana, and eventually Anvil)
# land in docker-compose.yml as each phase introduces them. When a service
# is added there, this script gains the corresponding `docker compose down` call.
#
# See docker-compose.yml for the full services roadmap.
set -euo pipefail

echo "devnet-down: no services configured yet (Phase 0 placeholder)"
exit 0
