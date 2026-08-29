//! CLI headless: `toile drape`, `toile check`, goldens de CI y benchmarks.
//!
//! Ver `docs/architecture.html` §2.7.

fn main() {
    println!("toile {}", env!("CARGO_PKG_VERSION"));
}
