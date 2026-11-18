#!/usr/bin/env python3
"""Fail when source changes are not accompanied by an OpenSpec change.

Two questions, no diff parsing:
  1. Did anything outside openspec/ change?
  2. Did anything inside openspec/changes/ change, and is a change active?
"""

import subprocess
import sys

IGNORE_PREFIXES = ("target/", ".jj/", ".git/", ".DS_Store")
SPEC_PREFIX = "openspec/changes/"


def changed_files():
    """Files changed in the working copy, via jj, falling back to git."""
    try:
        out = subprocess.run(
            ["jj", "diff", "--name-only"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        return [line.strip() for line in out.splitlines() if line.strip()]
    except (FileNotFoundError, subprocess.CalledProcessError):
        pass

    try:
        out = subprocess.run(
            ["git", "status", "--porcelain"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        return [line[3:].strip() for line in out.splitlines() if line.strip()]
    except (FileNotFoundError, subprocess.CalledProcessError):
        return []


def active_changes():
    try:
        out = subprocess.run(
            ["openspec", "list", "--json"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    except (FileNotFoundError, subprocess.CalledProcessError):
        return []
    import json

    try:
        data = json.loads(out)
    except json.JSONDecodeError:
        return []
    items = data.get("changes", data if isinstance(data, list) else [])
    return [c.get("name", "") for c in items if isinstance(c, dict)]


def main():
    files = [f for f in changed_files() if not f.startswith(IGNORE_PREFIXES)]
    if not files:
        print("ok (nothing changed)")
        return 0

    code = [f for f in files if not f.startswith("openspec/")]
    specs = [f for f in files if f.startswith(SPEC_PREFIX)]
    changes = active_changes()

    if code and not specs:
        print("FAIL: source changed with no update under openspec/changes/")
        for f in code[:8]:
            print(f"  changed: {f}")
        print("  Fix: tick the task you are doing, or add a change that covers it.")
        return 1

    if code and not changes:
        print("FAIL: source changed but no OpenSpec change is active")
        for f in code[:8]:
            print(f"  changed: {f}")
        print("  Fix: openspec new change <name>, then write proposal and tasks.")
        return 1

    print(f"ok ({len(code)} source, {len(specs)} spec, {len(changes)} active)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
