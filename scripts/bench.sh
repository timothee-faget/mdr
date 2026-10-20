#!/bin/sh
# Measures mdr's startup time, memory and size on a release build.
# Usage: scripts/bench.sh [runs]   (or `just bench`, which builds first)
#
# Startup: from `open` (what a double-click does) to the first frame painted
# with the document, reported by the app when MDR_BENCH is set (bench.rs).
# Memory: phys_footprint (Activity Monitor's "Memory") of mdr plus the WebKit
# processes it spawned (WebContent, GPU, Networking), 3 s after the paint.
# Documents live in a temporary folder outside ~/Documents, so no privacy
# prompt can get in the way, and the config is empty (defaults).
set -eu

runs=${1:-10}
app="$(cd "$(dirname "$0")/.." && pwd)/target/release/bundle/macos/mdr.app"
[ -d "$app" ] || { echo "No release build at $app: run \`cargo tauri build --bundles app\` first." >&2; exit 1; }

work=$(mktemp -d)
trap 'pkill -x mdr 2>/dev/null || true; rm -rf "$work"' EXIT
mkdir -p "$work/config"
root="$(cd "$(dirname "$0")/.." && pwd)"
cp "$root/README.md" "$work/readme.md"
: > "$work/long.md"
copies=0
while [ "$(wc -l < "$work/long.md")" -lt 10000 ]; do
    copies=$((copies + 1))
    # One footnote label per copy: comrak moves everything between two
    # definitions of the same label into the footnotes.
    sed "s/\[^1\]/[^$copies]/g" "$root/src-tauri/tests/fixtures/kitchen_sink.md" >> "$work/long.md"
    cat "$root/src-tauri/tests/fixtures/languages.md" >> "$work/long.md"
done

wait_for_exit() {
    while pgrep -x mdr > /dev/null; do sleep 0.1; done
}

# Prints the milliseconds from `open` to "mdr: painted" in the app's stderr.
launch() { # <mode> <file or empty>
    log="$work/stderr.log"
    rm -f "$log"
    perl -MTime::HiRes=time,sleep -e '
        my ($app, $mode, $config, $log, @files) = @ARGV;
        my $start = time;
        system("open", "-a", $app, "--env", "MDR_BENCH=$mode", "--env", "XDG_CONFIG_HOME=$config",
               "--stderr", $log, @files) == 0 or die "open failed\n";
        while (time - $start < 15) {
            if (open(my $fh, "<", $log)) {
                while (<$fh>) { if (/mdr: painted/) { printf "%.0f\n", (time - $start) * 1000; exit 0 } }
            }
            sleep 0.005;
        }
        die "no paint within 15 s\n";
    ' "$app" "$1" "$work/config" "$log" ${2:+"$2"}
}

median() { sort -n | awk '{ a[NR] = $1 } END { print (NR % 2) ? a[(NR + 1) / 2] : int((a[NR / 2] + a[NR / 2 + 1]) / 2) }'; }

webkit_pids() {
    pgrep -f 'com\.apple\.WebKit\.(WebContent|GPU|Networking)' | sort || true
}

# phys_footprint in MB of the given pids, summed.
footprint_mb() {
    /usr/bin/footprint --noCategories -f bytes $(for pid in "$@"; do printf -- '-p %s ' "$pid"; done) 2>/dev/null |
        awk '/phys_footprint:/ { sum += $2 } END { printf "%.0f", sum / 1048576 }'
}

pkill -x mdr 2>/dev/null || true
wait_for_exit

echo "mdr $(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist"), $(sw_vers -productName) $(sw_vers -productVersion), $(sysctl -n machdep.cpu.brand_string)"
echo "Document: long.md = $(wc -l < "$work/long.md" | tr -d ' ') lines; $runs launches each, median (first launch apart: it pays for cold caches)."
echo
echo "| Document | First launch | Median | Min | Max | Memory |"
echo "|---|---|---|---|---|---|"

for case in empty readme long; do
    file=""
    [ "$case" = empty ] || file="$work/$case.md"

    times=""
    for _ in $(seq "$runs"); do
        times="$times $(launch quit "$file")"
        wait_for_exit
    done
    first=$(echo "$times" | awk '{ print $1 }')
    rest=$(echo "$times" | tr ' ' '\n' | sed '/^$/d' | tail -n +2)
    [ -n "$rest" ] || rest=$first

    webkit_pids > "$work/before"
    launch 1 "$file" > /dev/null
    sleep 3
    webkit_pids > "$work/after"
    spawned=$(comm -13 "$work/before" "$work/after")
    memory=$(footprint_mb "$(pgrep -x mdr)" $spawned)
    pkill -x mdr
    wait_for_exit

    printf '| %s | %s ms | %s ms | %s ms | %s ms | %s MB |\n' "$case" "$first" \
        "$(echo "$rest" | median)" "$(echo "$rest" | sort -n | head -1)" "$(echo "$rest" | sort -n | tail -1)" "$memory"
done

echo
echo "App size: $(du -sh "$app" | cut -f1) (\`.app\`), $(ls -l "$app/Contents/MacOS/mdr" | awk '{ printf "%.1f MB", $5 / 1048576 }') (binary)"
