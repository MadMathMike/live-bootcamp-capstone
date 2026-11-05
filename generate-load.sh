#!/usr/bin/env bash
# Send requests at a fixed rate. Prints each response's status code (000 = failed).
# Usage: ./load.sh [TPS] [URL]
set -u

tps=${1:-10}
url=${2:-http://localhost:3000/work}

if ! [[ $tps =~ ^[1-9][0-9]*$ ]]; then
    echo "TPS must be a positive integer, got: $tps" >&2
    exit 1
fi

interval_us=$((1000000 / tps))
next=${EPOCHREALTIME/[.,]/}

while :; do
    curl -s -o /dev/null --max-time 5 -w '%{http_code}\n' "$url" &

    # Schedule against an absolute deadline so loop overhead doesn't lower the rate.
    next=$((next + interval_us))
    delay=$((next - ${EPOCHREALTIME/[.,]/}))
    if ((delay > 0)); then
        sleep "$(printf '%d.%06d' $((delay / 1000000)) $((delay % 1000000)))"
    fi
done
