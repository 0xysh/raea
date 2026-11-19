//! A local model's install on any OS, with nothing from Homebrew or the system's package manager: its
//! tools/<model>/setup.py, run by uv (which makes the venv and brings its own Python), and the venv's Python run after.
//! uv is the one on the PATH, else raea's own: a pinned release fetched once into ~/.cache/raea-tools/uv and checked
//! against its SHA-256 (where tools/ensure.sh puts the same uv on a Mac, so either finds the other's). Until 0.1.8 each setup was a `sh` script needing uv already installed (`brew install uv`), so Windows
//! had no way to install any local model, and a Mac without Homebrew none either.

use super::*;

/// The uv raea fetches when there's none on the PATH: the release every local model's setup was measured with
const UV: &str = "0.12.24";

/// uv's release archive for each platform raea ships on, by target, and its SHA-256 (the release's sha256.sum)
const UV_ARCHIVES: [(&str, &str); 6] = [
    ("aarch64-apple-darwin", "0c4346de7abdb49495b393b9ec809fe387aa43e586be20fecb972216c1e71732"),
    ("x86_64-apple-darwin", "4fa82e37cb94767661f532b001e470b67a186c7260e305bd84ddb78fd545c0b6"),
    ("aarch64-unknown-linux-gnu", "5231be65f496304623895dacdbf1de8504fec90303684bdf05805aa34414dd21"),
    ("x86_64-unknown-linux-gnu", "b4dfaef47d491a7296981f8374a4595f55dbf84e8937c8ecd2983574d8bb3da6"),
    ("aarch64-pc-windows-msvc", "4b783bda5cc44bbae0651a837223873a7acee31152381aef5fc86ef6bef25997"),
    ("x86_64-pc-windows-msvc", "7c38608c8a18ee137d748a1773053b07ec8f3a30fab49aebaa6f4e4efeceb019"),
];

fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}

fn on_path(name: &str) -> Option<PathBuf> {
    let exe = exe(name);
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(&exe)).find(|p| p.is_file())
}

/// The Python of the venv in an install folder: .venv/bin/python, or .venv\Scripts\python.exe on Windows
pub fn venv_python(dir: &Path) -> PathBuf {
    if cfg!(windows) { dir.join(".venv").join("Scripts").join("python.exe") } else { dir.join(".venv").join("bin").join("python") }
}

/// uv: $RAEA_UV, the one on the PATH, or raea's own, fetched the first time (about 20 MB) into $RAEA_TOOLS_CACHE (else
/// ~/.cache/raea-tools)/uv, as tools/ensure.sh does
pub fn uv() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("RAEA_UV").map(PathBuf::from).filter(|p| p.is_file()) {
        return Ok(p);
    }
    if let Some(p) = on_path("uv") {
        return Ok(p);
    }
    let dir = raea_io_home::env_path("RAEA_TOOLS_CACHE", ".cache/raea-tools").join("uv");
    let bin = dir.join(exe("uv"));
    if bin.is_file() {
        return Ok(bin);
    }
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    let target = match os {
        "macos" => format!("{arch}-apple-darwin"),
        "linux" => format!("{arch}-unknown-linux-gnu"),
        _ => format!("{arch}-pc-windows-msvc"),
    };
    let Some(&(_, sum)) = UV_ARCHIVES.iter().find(|a| a.0 == target) else {
        return Err(format!("raea has no uv for {os} {arch}: install uv (https://docs.astral.sh/uv) and run again"));
    };
    let name = format!("uv-{target}.{}", if os == "windows" { "zip" } else { "tar.gz" });
    raea_io_home::offline(&format!("fetching uv {UV} for a local model's setup"))?;
    eprintln!("raea: fetching uv {UV} ({name}) into {}", dir.display());
    // unpacked beside it first and moved in whole, so a stopped fetch never leaves a uv that looks installed
    let part = dir.with_file_name(format!("uv.part{}", std::process::id()));
    let fetched = fetch_uv(&part, &name, sum, os != "windows");
    if fetched.is_ok() && !bin.is_file() {
        let _ = std::fs::rename(&part, &dir);
    }
    let _ = std::fs::remove_dir_all(&part);
    fetched?;
    if bin.is_file() { Ok(bin) } else { Err(format!("couldn't put uv in {}", dir.display())) }
}

/// uv's release archive `name` downloaded into `part` with curl, checked against `sum` and unpacked there by tar
/// (Windows' own curl.exe and tar.exe, which reads a zip: the .tar.gz holds uv-<target>/uv, the zip uv.exe at its top)
fn fetch_uv(part: &Path, name: &str, sum: &str, strip: bool) -> Result<(), String> {
    std::fs::create_dir_all(part).map_err(|e| format!("{}: {e}", part.display()))?;
    let archive = part.join(name);
    let url = format!("https://github.com/astral-sh/uv/releases/download/{UV}/{name}");
    let got = std::process::Command::new(exe("curl")).args(["-fsSL", "--retry", "3", "-o"]).arg(&archive).arg(&url).status();
    if !got.is_ok_and(|s| s.success()) {
        return Err(format!("couldn't download {url} with curl"));
    }
    let bytes = std::fs::read(&archive).map_err(|e| format!("{}: {e}", archive.display()))?;
    if format!("{:x}", Sha256::digest(&bytes)) != sum {
        return Err(format!("{name} doesn't match its SHA-256"));
    }
    let mut tar = std::process::Command::new(exe("tar"));
    tar.arg("-xf").arg(&archive).arg("-C").arg(part);
    if strip {
        tar.arg("--strip-components=1");
    }
    if !tar.status().is_ok_and(|s| s.success()) {
        return Err(format!("couldn't unpack {name}"));
    }
    std::fs::remove_file(&archive).map_err(|e| format!("{}: {e}", archive.display()))
}

/// How a person runs a setup themselves: setup.sh on a Mac or Linux, uv on setup.py on Windows
pub fn setup_how(script: &Path) -> String {
    if cfg!(windows) { format!("uv run --no-project --python 3.11 python -I {}", script.with_file_name("setup.py").display()) } else { script.display().to_string() }
}

/// A model's setup, its progress on stderr (stdout is the caller's answer: a render's APPLIED line, the CLI's), with
/// `var` naming the install folder `dir`: tools/<model>/setup.py by uv beside `script` (its setup.sh), else the
/// setup.sh itself by sh.
pub fn run_setup(script: &Path, var: &str, dir: &Path) -> Result<std::process::ExitStatus, String> {
    let py = script.with_file_name("setup.py");
    let mut cmd = if py.exists() {
        let uv = uv()?;
        let mut c = std::process::Command::new(&uv);
        c.args(["run", "-q", "--no-project", "--python", "3.11", "python", "-I"]).arg(&py).env("UV", &uv);
        c
    } else {
        let mut c = std::process::Command::new("sh");
        c.arg(script);
        c
    };
    cmd.env(var, dir).stdout(std::io::stderr()).status().map_err(|e| format!("{}: {e}", py.display()))
}
