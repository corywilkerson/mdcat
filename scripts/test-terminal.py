#!/usr/bin/env python3
"""Exercise actual terminal detection without depending on a user's terminal."""
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent


def main():
    if os.name == "nt":
        print("Skipped pseudo-terminal checks on Windows.")
        return
    import errno
    import fcntl
    import pty
    import select
    import struct
    import termios
    import time

    subprocess.run(["cargo", "build", "--locked", "--offline"], cwd=ROOT, check=True)
    binary = ROOT / "target" / "debug" / "mdcat"

    def run(args=(), overrides=None, interactive_input=False):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 32, 0, 0))
        environment = os.environ.copy()
        environment.pop("NO_COLOR", None)
        environment["TERM"] = "xterm-256color"
        environment.update(overrides or {})
        try:
            with subprocess.Popen([str(binary), *args], cwd=ROOT, env=environment,
                                  stdin=slave if interactive_input else subprocess.PIPE,
                                  stdout=slave, stderr=subprocess.PIPE) as process:
                os.close(slave)
                slave = None
                if not interactive_input:
                    process.stdin.write(b"# Title\n\n**bold** [site](https://example.com)\n\none two three four five six seven eight nine ten eleven twelve\n")
                    process.stdin.close()
                output = bytearray()
                deadline = time.monotonic() + 10
                while True:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0 or not select.select([master], [], [], remaining)[0]:
                        process.kill()
                        raise AssertionError(f"terminal check timed out: {args}")
                    try:
                        chunk = os.read(master, 65536)
                    except OSError as error:
                        if error.errno == errno.EIO:
                            break
                        raise
                    if not chunk:
                        break
                    output.extend(chunk)
                error = process.stderr.read().decode()
                status = process.wait(timeout=10)
                return status, output.decode().replace("\r\n", "\n"), error
        finally:
            os.close(master)
            if slave is not None:
                os.close(slave)

    checks = 0
    for args, overrides, styled in [
        ((), {}, True),
        ((), {"NO_COLOR": "1"}, False),
        ((), {"NO_COLOR": ""}, True),
        ((), {"TERM": "dumb"}, False),
        (("--plain",), {}, False),
        (("--color=always",), {"NO_COLOR": "1", "TERM": "dumb"}, True),
    ]:
        status, output, error = run(args, overrides)
        assert status == 0 and not error, (args, overrides, status, error)
        assert ("\x1b[1m" in output) == styled, (args, overrides, repr(output))
        assert ("\x1b]8;;https://example.com" in output) == styled
        checks += 1
    status, output, error = run(("--plain",))
    assert status == 0 and not error
    assert "━" * 32 + "\n" in output, repr(output)
    assert max(map(len, output.splitlines())) <= 32, repr(output)
    checks += 1
    status, output, error = run(interactive_input=True)
    assert status == 2 and not output and "provide a file" in error, (status, output, error)
    checks += 1
    print(f"Passed {checks} pseudo-terminal checks.")


if __name__ == "__main__":
    main()
