---
name: macos
description: Inspect and change things on macOS. Use for launchd services, defaults, brew, disk space, network, or system settings.
---

# macOS

- Services: `launchctl list | grep <name>`, `brew services list`.
- Defaults: read with `defaults read <domain> <key>`; write only when asked.
- Disk: `df -h /`, `du -sh <dir>/* | sort -h | tail`.
- Network: `scutil --dns`, `networksetup -listallhardwareports`.
- Sleep/energy: `pmset -g`; do not change without being asked.
