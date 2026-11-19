"""What every local model's setup.py shares (tools/sao, tokensynth, matchmove, splat, goo).

raea runs a model's setup.py the first time the model is asked for, through uv on any OS
(`uv run --no-project --python 3.11 python -I tools/<model>/setup.py`, raea_io_assets::run_setup), with the install
folder in the model's variable ($RAEA_SAO, ...) and uv in $UV; the setup.sh beside each does the same by hand. Nothing
here needs Homebrew, a system Python or a POSIX shell: files come through curl (on a Mac, Linux, and Windows 10 and
later), checked against their SHA-256 or copied from another install; the venv is uv's, with its own Python; and that
Python is .venv/bin/python, or .venv/Scripts/python.exe on Windows. Standard library only: it runs before any venv."""

import hashlib
import os
import shutil
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

WINDOWS = os.name == "nt"
MAC = sys.platform == "darwin"

# the footage tools' venv (matchmove and splat), at the versions measured (2026-10-20; PyAV 2026-11-19), one set so uv's
# cache serves both: COLMAP, OpenCV, Pillow, and PyAV, whose wheels carry FFmpeg's own libraries (no FFmpeg installed)
FOOTAGE = ["pycolmap==4.2.1", "numpy==2.4.6", "opencv-python-headless==5.0.0.93", "pillow==13.0.0", "av==18.1.0"]


def digest(path):
    """A file's SHA-256, hex"""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def low():
    """Run a heavy step at a low priority where the OS has one (nice 15), so a render or a person's work goes first"""
    return {} if WINDOWS else {"preexec_fn": lambda: os.nice(15)}


class Setup:
    """An install folder, held by one setup at a time: `with Setup("sao", "RAEA_SAO", "raea-sao") as s:` works in
    the folder, and a second setup started meanwhile exits 75 naming the lock."""

    def __init__(self, name, var, folder):
        self.name = name
        self.dir = Path(os.environ.get(var) or Path.home() / ".cache" / folder).expanduser().resolve()
        # another install to copy from instead of downloading: $<VAR>_FROM, else the default folder when installing elsewhere
        other = Path(os.environ.get(f"{var}_FROM") or Path.home() / ".cache" / folder).expanduser()
        self.other = other.resolve() if other.is_dir() and other.resolve() != self.dir else None
        self.uv = os.environ.get("UV") or shutil.which("uv")
        if not self.uv:
            sys.exit(f"{name} setup: needs uv: `raea setup-audio` fetches raea's own, or install it (https://docs.astral.sh/uv)")
        self.lock = self.dir / ".setup-lock"

    def __enter__(self):
        self.dir.mkdir(parents=True, exist_ok=True)
        try:
            self.lock.mkdir()
        except FileExistsError:
            print(f"{self.name} setup: another setup is running in {self.dir} (remove {self.lock} if it isn't)", file=sys.stderr)
            sys.exit(75)
        os.chdir(self.dir)
        return self

    def __exit__(self, *exc):
        shutil.rmtree(self.lock, ignore_errors=True)

    def say(self, *words):
        print(f"{self.name} setup:", *words, file=sys.stderr, flush=True)

    @property
    def python(self):
        return self.dir / ".venv" / ("Scripts/python.exe" if WINDOWS else "bin/python")

    def run(self, cmd, **kw):
        subprocess.run([str(c) for c in cmd], check=True, **kw)

    def imports(self, *modules):
        """Whether the venv's Python imports every one of `modules`"""
        return self.python.exists() and subprocess.run([str(self.python), "-c", "import " + ", ".join(modules)], capture_output=True).returncode == 0

    def venv(self, version, packages, imports=(), what="making the venv", managed=False):
        """The venv at Python `version` with `packages`; one that already imports `imports` is left alone. `managed`:
        on uv's own Python, never one the system or Homebrew has"""
        if imports and self.imports(*imports):
            return
        self.say(what)
        if not self.python.exists():
            self.run([self.uv, "venv", "-q", *(["--python-preference", "only-managed"] if managed else []), "--python", version, ".venv"])
        self.run([self.uv, "pip", "install", "-q", "--python", self.python, *packages], env={**os.environ, "VIRTUAL_ENV": str(self.dir / ".venv")})

    def copy(self, src, dest):
        """A file or folder copied: an APFS clone on a Mac (no extra disk until one side changes)"""
        if MAC and subprocess.run(["cp", "-cR", str(src), str(dest)], capture_output=True).returncode == 0:
            return
        if Path(src).is_dir():
            shutil.copytree(src, dest)
        else:
            shutil.copy2(src, dest)

    def download(self, url, to):
        """`url` into the file `to`, through curl (its progress bar on a terminal; in a log, a render's or the CLI's
        piped, only errors), else Python's own HTTP"""
        curl = shutil.which("curl")
        if curl:
            self.run([curl, "-fL", "--retry", "3", *(["--progress-bar"] if sys.stderr.isatty() else ["-sS"]), "-o", to, url])
            return
        with urllib.request.urlopen(url, timeout=120) as r, open(to, "wb") as f:
            shutil.copyfileobj(r, f, 1 << 20)

    def fetch(self, dest, url, sha256=None):
        """A file of the install: kept when it's here, copied from the other install when that has it, else downloaded;
        checked against `sha256` when one is given"""
        dest = Path(dest)
        if dest.is_file() and dest.stat().st_size > 0:
            return
        dest.parent.mkdir(parents=True, exist_ok=True)
        part = dest.with_name(dest.name + ".part")
        part.unlink(missing_ok=True)
        theirs = self.other / dest if self.other else None
        if theirs and theirs.is_file() and theirs.stat().st_size > 0:
            self.copy(theirs, part)
        else:
            self.say(f"downloading {dest.as_posix()}")
            self.download(url, part)
        if sha256 and digest(part) != sha256:
            part.unlink()
            sys.exit(f"{self.name} setup: {dest.as_posix()} doesn't match its SHA-256")
        os.replace(part, dest)

    def ready(self, *folders):
        """The install marked whole (.ready, the time it finished), with the size of `folders`"""
        (self.dir / ".ready").write_text(time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()) + "\n")
        for f in folders:
            n = sum(p.stat().st_size for p in Path(f).rglob("*") if p.is_file() and not p.is_symlink())
            self.say(f"{f}: {n / 1e6:.0f} MB")
        self.say(f"ready in {self.dir}")
