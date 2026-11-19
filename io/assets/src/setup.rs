//! A local model's install on any OS, with nothing from Homebrew or the system's package manager: its
//! tools/<model>/setup.py, run by uv (which makes the venv and brings its own Python), and the venv's Python run after.
//! uv is the one on the PATH, else raea's own: a pinned release fetched once into ~/.cache/raea-tools/uv and checked
//! against its SHA-256 (where tools/ensure.sh puts the same uv on a Mac, so either finds the other's). Until 0.1.8 each
//! setup was a `sh` script needing uv already installed (`brew install uv`), so Windows had no way to install any local
//! model, and a Mac without Homebrew none either. FFmpeg, on Windows (ffmpeg_tool), is fetched the same way the first
//! time a video needs it.

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

/// The FFmpeg raea fetches on Windows when none is installed: GyanD's essentials build (the one winget's
/// Gyan.FFmpeg.Essentials installs), kept on GitHub for each release; its SHA-256 is GitHub's digest for the asset
const FFMPEG: &str = "9.0.2";
const FFMPEG_SUM: &str = "60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba";

/// `ffmpeg` or `ffprobe` as raea runs it: the one on the PATH; else raea's own in $RAEA_TOOLS_CACHE (else
/// ~/.cache/raea-tools)/ffmpeg/bin. On Windows, where FFmpeg is rarely installed, raea's own is fetched there once
/// (114 MB; ffmpeg.exe, ffprobe.exe and its LICENSE kept, about 210 MB) the first time a video raea's own decoders
/// can't read needs it, when `fetch`: H.264 with B-frames, HEVC, VP9 (a phone's or a camera's). Elsewhere FFmpeg is
/// the system's: a Mac reads those on VideoToolbox, and Linux has FFmpeg in its package manager.
pub fn ffmpeg_tool(name: &str, fetch: bool) -> Result<PathBuf, String> {
    if let Some(p) = on_path(name) {
        return Ok(p);
    }
    let dir = raea_io_home::env_path("RAEA_TOOLS_CACHE", ".cache/raea-tools").join("ffmpeg");
    let bin = dir.join("bin").join(exe(name));
    if bin.is_file() {
        return Ok(bin);
    }
    if !cfg!(windows) {
        return Err(format!("{name} isn't installed: raea reads this video through FFmpeg (apt install ffmpeg, dnf install ffmpeg or brew install ffmpeg)"));
    }
    if !fetch {
        return Err(format!("{name} isn't installed yet (raea fetches its own the first time a video needs it)"));
    }
    // one fetch at a time in a process (a render opens its videos on several threads); another process's is a
    // separate .part folder, and whichever finishes first is moved in
    static FETCHING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = FETCHING.lock().unwrap_or_else(|e| e.into_inner());
    if bin.is_file() {
        return Ok(bin);
    }
    raea_io_home::offline(&format!("fetching FFmpeg {FFMPEG} to read a video raea's own decoders can't"))?;
    eprintln!("raea: this video needs FFmpeg, which isn't installed; fetching FFmpeg {FFMPEG} (114 MB, once) into {}", dir.display());
    let name_zip = format!("ffmpeg-{FFMPEG}-essentials_build.zip");
    let url = format!("https://github.com/GyanD/codexffmpeg/releases/download/{FFMPEG}/{name_zip}");
    let top = format!("ffmpeg-{FFMPEG}-essentials_build");
    let members = [format!("{top}/bin/ffmpeg.exe"), format!("{top}/bin/ffprobe.exe"), format!("{top}/LICENSE")];
    let part = dir.with_file_name(format!("ffmpeg.part{}", std::process::id()));
    let fetched = fetch_archive(&part, &url, &name_zip, FFMPEG_SUM, true, &members.each_ref().map(String::as_str));
    settle(&part, &dir, &bin, fetched, "FFmpeg")
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
    let url = format!("https://github.com/astral-sh/uv/releases/download/{UV}/{name}");
    let fetched = fetch_archive(&part, &url, &name, sum, os != "windows", &[]);
    settle(&part, &dir, &bin, fetched, "uv")
}

/// A fetch's end: its `part` folder moved in as `dir` when it's whole and nothing got there first, the part removed,
/// and, once the tool is in place, the parts stopped fetches left beside it swept (`NAME.part<pid>`, from a Ctrl+C or
/// a killed process: FFmpeg's held a 114 MB zip, and each run made a new one). A part another process is still
/// writing stays on Windows, where its open file can't be removed; elsewhere that fetch fails and finds the tool in
/// place, which counts as done.
fn settle(part: &Path, dir: &Path, bin: &Path, fetched: Result<(), String>, what: &str) -> Result<PathBuf, String> {
    if fetched.is_ok() && !bin.is_file() {
        let _ = std::fs::rename(part, dir);
    }
    let _ = std::fs::remove_dir_all(part);
    if bin.is_file() {
        let stale = dir.file_name().map(|n| format!("{}.part", n.to_string_lossy())).unwrap_or_default();
        let beside = dir.parent().map(std::fs::read_dir).into_iter().flatten().flatten().flatten();
        for e in beside.filter(|e| !stale.is_empty() && e.file_name().to_string_lossy().starts_with(&stale)) {
            let _ = std::fs::remove_dir_all(e.path());
        }
        return Ok(bin.to_path_buf());
    }
    fetched?;
    Err(format!("couldn't put {what} in {}", dir.display()))
}

/// A release archive `name` from `url` downloaded into `part` with curl (resumed across its tries: a dropped connection
/// costs only what wasn't fetched), checked against `sum` and unpacked there by tar: `members` alone when given, its top
/// folder stripped when `strip`. Windows' own curl.exe and tar.exe do it there (bsdtar reads a zip).
fn fetch_archive(part: &Path, url: &str, name: &str, sum: &str, strip: bool, members: &[&str]) -> Result<(), String> {
    use std::io::IsTerminal;
    std::fs::create_dir_all(part).map_err(|e| format!("{}: {e}", part.display()))?;
    let archive = part.join(name);
    let quiet = if std::io::stderr().is_terminal() { "--progress-bar" } else { "-sS" };
    let got = std::process::Command::new(exe("curl"))
        .args(["-fL", quiet, "-C", "-", "--retry", "5", "--retry-all-errors", "--connect-timeout", "30", "-o"])
        .arg(&archive)
        .arg(url)
        .status();
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
    tar.args(members);
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

#[cfg(test)]
mod tests {
    use super::*;

    // a fetch's end sweeps the parts stopped fetches left beside its tool, and only those
    #[test]
    fn settle_sweeps_stopped_fetches() {
        let root = std::env::temp_dir().join(format!("raea-settle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (dir, bin) = (root.join("ffmpeg"), root.join("ffmpeg").join("bin").join("ffmpeg"));
        let mk = |p: &Path, f: &str| {
            std::fs::create_dir_all(p).unwrap();
            std::fs::write(p.join(f), b"x").unwrap();
        };
        // two stopped fetches (one with its zip), another tool's part, and this run's finished part
        mk(&root.join("ffmpeg.part111"), "ffmpeg-9.0.2-essentials_build.zip");
        mk(&root.join("ffmpeg.part222"), "x");
        mk(&root.join("uv.part333"), "x");
        let part = root.join("ffmpeg.part444");
        mk(&part.join("bin"), "ffmpeg");
        assert_eq!(settle(&part, &dir, &bin, Ok(()), "FFmpeg").unwrap(), bin);
        assert!(bin.is_file(), "the part moved in");
        for gone in ["ffmpeg.part111", "ffmpeg.part222", "ffmpeg.part444"] {
            assert!(!root.join(gone).exists(), "{gone} swept");
        }
        assert!(root.join("uv.part333").exists(), "another tool's part kept");
        // a fetch that failed but finds the tool in place (another process put it there) is done, its part removed
        let late = root.join("ffmpeg.part555");
        mk(&late, "half.zip");
        assert_eq!(settle(&late, &dir, &bin, Err("curl failed".into()), "FFmpeg").unwrap(), bin);
        assert!(!late.exists());
        // a failure with no tool says so
        let none = root.join("none");
        let err = settle(&root.join("none.part1"), &none, &none.join("bin").join("x"), Err("curl failed".into()), "x").unwrap_err();
        assert_eq!(err, "curl failed");
        let _ = std::fs::remove_dir_all(&root);
    }
}
