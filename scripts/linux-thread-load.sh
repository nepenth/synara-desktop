#!/usr/bin/env bash
# Samples how busy a running Synara is on Linux, without a debugger or root.
#
# For the app process and each WebKit web process it starts, prints per
# interval: CPU of the main thread (the GTK thread that also handles key
# input, or the web process's JavaScript/layout thread), CPU of the whole
# process, and main-thread read syscalls per second. Wakeups near the
# display's refresh rate with little CPU elsewhere mean the page repaints
# every frame (a running animation); near-zero means idle.
#
# Usage: scripts/linux-thread-load.sh [pid] [interval-seconds] [samples]
#   pid defaults to the running process named "synara"; samples 0 = forever.
# Output is tab-separated, one line per process per interval, so it can be
# pasted into a spreadsheet or compared before and after a change.
set -euo pipefail

pid="${1:-}"
interval="${2:-5}"
samples="${3:-0}"

if [[ -z "$pid" ]]; then
  # Skip defunct children named synara; the app is the live one with the
  # most threads.
  pid="$(ps -C synara -o pid=,stat=,nlwp= | awk '$2 !~ /Z/ && $3 > max { max = $3; pid = $1 } END { print pid }')"
fi
if [[ -z "$pid" || ! -d "/proc/$pid" ]]; then
  echo "No running synara process found (pass its pid)." >&2
  exit 1
fi

ticks_per_second="$(getconf CLK_TCK)"

# utime+stime of /proc/<pid>[/task/<tid>]/stat, in clock ticks. The comm
# field can contain spaces, so fields are counted after the closing paren.
cpu_ticks() {
  local stat
  stat="$(cat "$1/stat" 2>/dev/null)" || { echo 0; return; }
  stat="${stat##*) }"
  # After "pid (comm) ", utime and stime are fields 12 and 13.
  awk '{ print $12 + $13 }' <<<"$stat"
}

read_syscalls() {
  awk '/^syscr:/ { print $2 }' "$1/io" 2>/dev/null || echo 0
}

web_processes() {
  local candidate
  for candidate in /proc/[0-9]*; do
    [[ "$(cat "$candidate/comm" 2>/dev/null)" == WebKitWebProces* ]] || continue
    # The web process is a child of the app, or of a bubblewrap helper the app started.
    local parent
    parent="$(awk '{ print $4 }' "$candidate/stat" 2>/dev/null || echo 0)"
    while [[ "$parent" -gt 1 && "$parent" != "$pid" ]]; do
      parent="$(awk '{ print $4 }' "/proc/$parent/stat" 2>/dev/null || echo 1)"
    done
    [[ "$parent" == "$pid" ]] && echo "${candidate#/proc/}"
  done
}

declare -A labels
labels["$pid"]="app"
for web in $(web_processes); do labels["$web"]="web"; done

printf 'time\tprocess\tpid\tmain_thread_cpu_pct\tprocess_cpu_pct\tmain_reads_per_s\n'
count=0
while [[ "$samples" -eq 0 || "$count" -lt "$samples" ]]; do
  declare -A main_before=() all_before=() reads_before=()
  for p in "${!labels[@]}"; do
    main_before[$p]="$(cpu_ticks "/proc/$p/task/$p")"
    all_before[$p]="$(cpu_ticks "/proc/$p")"
    reads_before[$p]="$(read_syscalls "/proc/$p/task/$p")"
  done
  sleep "$interval"
  now="$(date +%T)"
  for p in "${!labels[@]}"; do
    [[ -d "/proc/$p" ]] || continue
    main=$(( $(cpu_ticks "/proc/$p/task/$p") - main_before[$p] ))
    all=$(( $(cpu_ticks "/proc/$p") - all_before[$p] ))
    reads=$(( $(read_syscalls "/proc/$p/task/$p") - reads_before[$p] ))
    awk -v t="$now" -v l="${labels[$p]}" -v p="$p" -v m="$main" -v a="$all" -v r="$reads" \
      -v hz="$ticks_per_second" -v s="$interval" \
      'BEGIN { printf "%s\t%s\t%s\t%.1f\t%.1f\t%d\n", t, l, p, m*100/(hz*s), a*100/(hz*s), r/s }'
  done
  count=$(( count + 1 ))
done
