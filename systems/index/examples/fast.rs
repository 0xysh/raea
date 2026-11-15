//! `raea index` without building all of raea, to measure the fast pass (docs/index.feature IX1, IX14):
//!   fast add FOLDER | fast search WORDS... | fast status | fast remove ID | fast serve   (cli.rs USAGE)
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(raea_sys_index::cli::cli(&args));
}
