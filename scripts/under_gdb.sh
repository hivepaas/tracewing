#!/bin/sh
#
# Runs a program under gdb as it would run by itself, so that a crash is
# written down where it happened. `make debug` runs tracewing this way on
# Linux when gdb is there.
#
#   scripts/under_gdb.sh target/debug/tracewing [ARGS...]
#
# A stack overflow is why: Rust says only "thread 'tokio-rt-worker' has
# overflowed its stack" and aborts, which names no function. gdb stops at the
# fault instead, on the thread that overflowed, and the innermost frames show
# what was calling itself. That goes to target/crashes/<when>.log, and the
# path is printed. A run that ends by itself leaves nothing behind.
#
# Everything else is as without gdb: what the program prints goes to the
# terminal, and Ctrl+C and the signals that ask a program to stop reach it,
# so that it puts back what it changed (the system proxy) as it always does.

set -eu

program=$1
shift

crashes=${TRACEWING_CRASHES:-target/crashes}
mkdir -p "$crashes"
log="$crashes/$(date +%Y%m%d-%H%M%S).log"

status=0
"${GDB:-gdb}" -q -batch -return-child-result \
    -iex 'set debuginfod enabled off' \
    -iex 'set auto-load python-scripts off' \
    -ex 'set pagination off' \
    -ex 'set confirm off' \
    -ex 'set print thread-events off' \
    -ex 'set print inferior-events off' \
    -ex 'handle SIGINT nostop noprint pass' \
    -ex 'handle SIGTERM nostop noprint pass' \
    -ex 'handle SIGHUP nostop noprint pass' \
    -ex 'handle SIGPIPE nostop noprint pass' \
    -ex 'handle SIGUSR1 nostop noprint pass' \
    -ex 'handle SIGUSR2 nostop noprint pass' \
    -ex run \
    -ex "set logging file $log" \
    -ex 'set logging overwrite on' \
    -ex 'set logging redirect on' \
    -ex 'set logging enabled on' \
    -ex 'info program' \
    -ex 'echo \n--- the thread that stopped, innermost first ---\n' \
    -ex 'bt 80' \
    -ex 'echo \n--- where it began ---\n' \
    -ex 'bt -25' \
    -ex 'echo \n--- every thread ---\n' \
    -ex 'thread apply all bt 12' \
    -ex 'set logging enabled off' \
    --args "$program" "$@" || status=$?

# A program that ended by itself has no stack to show.
if grep -q '^No stack\.' "$log" 2>/dev/null; then
    rm -f "$log"
elif [ -s "$log" ]; then
    echo
    echo "$program stopped on a signal; where it was is in $log:"
    sed -n '1,30p' "$log"
fi
exit "$status"
