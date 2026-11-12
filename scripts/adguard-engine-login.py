#!/usr/bin/python3
"""Start existing AdGuard configuration at login, without resetting it."""
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

CLI = "/usr/local/bin/adguard-cli"
ENV = dict(os.environ, LANG="C.UTF-8", LC_ALL="C.UTF-8", NO_COLOR="1", TERM="dumb")


def invoke(args, timeout):
    # A daemon's descendants can inherit stdout. Waiting for pipe EOF would
    # mistake that for a stuck CLI even after the original process exited.
    with tempfile.TemporaryFile(mode="w+") as output:
        process = subprocess.Popen(args, stdout=output, stderr=subprocess.STDOUT, env=ENV)
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            # Never signal the entire group: the setuid helper needs its own
            # cleanup, and a daemon may still be finishing its initialization.
            process.terminate()
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            raise
        output.seek(0)
        text = re.sub(r"\x1b\[[0-9;]*[A-Za-z]", "", output.read(1024 * 1024))
    if process.returncode:
        raise RuntimeError(f"{args[0]} {args[1]} exited with {process.returncode}")
    return text.lower()


def main():
    attempted = False
    last_error = "AdGuard state is unknown"
    for attempt in range(3):
        if attempt:
            time.sleep(10)
        try:
            status = invoke([CLI, "status"], 6)
            if "the adguard proxy server is running" in status:
                print("AdGuard engine is running with existing configuration.")
                return
            if "the adguard proxy server is not running" not in status:
                raise RuntimeError("AdGuard state is unknown; no reset was attempted")
            if not attempted:
                if shutil.which("nm-online"):
                    invoke(["nm-online", "--quiet", "--timeout=12"], 14)
                attempted = True
                invoke([CLI, "start"], 20)
                status = invoke([CLI, "status"], 6)
                if "the adguard proxy server is running" in status:
                    print("AdGuard engine started with existing configuration.")
                    return
            last_error = "AdGuard has not confirmed startup"
        except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
            last_error = str(error)
    raise RuntimeError(last_error + "; stopped after bounded login checks")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"AdGuard login start failed: {error}", file=sys.stderr)
        sys.exit(1)
