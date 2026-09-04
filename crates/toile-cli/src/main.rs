#![allow(missing_docs, reason = "a binary publishes no API surface")]

mod bench;
mod doc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("bench") => bench::run(&args[2..]),
        Some("doc") => doc::run(&args[2..]),
        Some("drape") => {
            let t = std::time::Instant::now();
            let hash = toile_engine::golden::drape_bodice_hash();
            eprintln!("drape golden en {:.2} s", t.elapsed().as_secs_f64());
            println!("{hash:#018x}");
        }
        _ => {
            println!("toile {}", env!("CARGO_PKG_VERSION"));
            println!(
                "subcomandos: bench [--verts N | --incr | --incr-async | --seams | --measure | --topo] · drape · doc [--resolve-with NOMBRE]"
            );
        }
    }
}
