#!/bin/sh

set -e

# Default arguments
DEFAULT_ARGS="--datadir /data --dev"

# If no arguments are provided, use the default ones
if [ $# -eq 0 ]; then
  set -- $DEFAULT_ARGS
fi

# Execute the velora binary with the provided arguments
exec velora "$@"
