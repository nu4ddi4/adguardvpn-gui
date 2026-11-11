#!/usr/bin/python3
"""Start existing AdGuard configuration at login, without resetting it."""
import os
import re
import shutil
import subprocess
import sys

CLI = "/usr/local/bin/adguard-cli"
ENV = dict(os.environ, LANG="C.UTF-8", LC_ALL="C.UTF-8", NO_COLOR="1", TERM="dumb")


def invoke(args, timeout):
    result = subprocess.run(args, capture_output=True, text=True, env=ENV, timeout=timeout)
    text = re.sub(r"\x1b\[[0-9;]*[A-Za-z]", "", result.stdout + "\n" + result.stderr)
    if result.returncode:
        raise RuntimeError(f"{args[0]} {args[1]} exited with {result.returncode}")
    return text.lower()


def main():
    status = invoke([CLI, "status"], 6)
    if "the adguard proxy server is running" in status:
        print("AdGuard engine is already running; configuration unchanged.")
        return
    if "the adguard proxy server is not running" not in status:
        raise RuntimeError("AdGuard state is unknown; no start or reset was attempted")
    if shutil.which("nm-online"):
        invoke(["nm-online", "--quiet", "--timeout=12"], 14)
    invoke([CLI, "start"], 20)
    status = invoke([CLI, "status"], 6)
    if "the adguard proxy server is running" not in status:
        raise RuntimeError("AdGuard did not confirm that its engine is running")
    print("AdGuard engine started with existing configuration.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"AdGuard login start failed: {error}", file=sys.stderr)
        sys.exit(1)
