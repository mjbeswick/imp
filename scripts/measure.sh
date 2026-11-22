#!/bin/sh
# usage: scripts/measure.sh [host]
# Reports the binary size, the median cold start of 30 `imp --version` runs,
# and the RSS of the full-screen interface idle for two seconds. With a host,
# measures ~/.local/bin/imp there over SSH; otherwise target/release/imp here.
# Needs python3 on the measured machine.
set -e
cd "$(dirname "$0")/.."
measure='
import os, pty, select, signal, statistics, subprocess, sys, tempfile, time
b = os.path.abspath(os.path.expanduser(sys.argv[1]))
runs = []
for _ in range(30):
    t = time.perf_counter()
    subprocess.run([b, "--version"], stdout=subprocess.DEVNULL)
    runs.append((time.perf_counter() - t) * 1000)
os.chdir(tempfile.mkdtemp())
pid, fd = pty.fork()
if pid == 0:
    os.execv(b, [b])
end = time.time() + 2
while time.time() < end:
    if select.select([fd], [], [], 0.1)[0]:
        try:
            os.read(fd, 65536)
        except OSError:
            break
rss = subprocess.run(["ps", "-o", "rss=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()
os.kill(pid, signal.SIGTERM)
print("size: %d bytes" % os.path.getsize(b))
print("cold start: %.1f ms median of 30" % statistics.median(runs))
print("idle RSS: %s KB" % (rss or "exited early"))
'
if [ -n "$1" ]; then
    ssh "$1" "python3 -c '$measure' .local/bin/imp"
else
    cargo build --release --quiet
    python3 -c "$measure" target/release/imp
fi
