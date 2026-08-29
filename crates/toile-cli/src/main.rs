//! CLI headless: `toile drape`, `toile check`, goldens de CI y benchmarks.
//!
//! Ver `docs/architecture.html` §2.7.

mod bench;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("bench") => bench::run(&args[2..]),
        _ => {
            println!("toile {}", env!("CARGO_PKG_VERSION"));
            println!("subcomandos: bench [--verts N]");
        }
    }
}
