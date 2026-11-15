//! The fast pass by hand, to measure it (docs/index.feature IX1, IX14):
//!   fast index FOLDER        list it and embed its pictures, printing progress as JSON lines
//!   fast search WORDS...     the best ten for the words
//! RAEA_INDEX_HOME picks the index's folder.
use raea_sys_index::{Progress, search, store::Store, text::Text};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut store = Store::open(&Store::default_dir())?;
    let say = |p: &Progress| println!("{}", serde_json::to_string(p).unwrap());
    match args.first().map(String::as_str) {
        Some("index") => {
            let root = Path::new(&args[1]);
            let (source, found) = raea_sys_index::list_folder(&mut store, root, &mut |p| say(p))?;
            eprintln!("listed {found}");
            let t = std::time::Instant::now();
            let model = raea_sys_index::apple::Model::load(&raea_sys_index::image_package()).map_err(anyhow::Error::msg)?;
            eprintln!("model loaded in {:.2} s", t.elapsed().as_secs_f64());
            let p = raea_sys_index::embed_pictures(&mut store, &model, &source, &mut |p| say(p))?;
            eprintln!("done: {} embedded, {} failed, {:.0}/s over {:.1} s", p.done, p.failed, p.per_sec, p.seconds);
        }
        Some("search") => {
            let words = args[1..].join(" ");
            let (m, tok) = raea_sys_index::text_files();
            let t = std::time::Instant::now();
            let mut text = Text::load(&m, &tok)?;
            let rows = search::Rows::load(&store, raea_sys_index::store::MODEL)?;
            eprintln!("loaded {} rows and the text tower in {:.2} s", rows.len(), t.elapsed().as_secs_f64());
            let t = std::time::Instant::now();
            let q = text.embed(&words)?;
            let e = t.elapsed().as_secs_f64();
            let hits = search::search(&store, &rows, &q, 10)?;
            eprintln!("query {:.0} ms, search {:.0} ms", e * 1e3, (t.elapsed().as_secs_f64() - e) * 1e3);
            for h in hits {
                println!("{:.3}  {}", h.score, h.locator);
            }
        }
        _ => eprintln!("fast index FOLDER | fast search WORDS..."),
    }
    Ok(())
}
