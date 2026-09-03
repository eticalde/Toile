mod fidelity;
mod incremental;
mod kernel;
mod scene;
mod seams;
mod topology;

/// Dispatches `toile bench`. With no flag, runs the kernel sweep.
pub fn run(args: &[String]) {
    let has = |f: &str| args.iter().any(|a| a == f);
    if has("--topo") {
        return topology::run();
    }
    if has("--measure") {
        return fidelity::run();
    }
    if has("--seams") {
        return seams::run();
    }
    if has("--incr-async") {
        return incremental::run_async();
    }
    if has("--incr") {
        return incremental::run_sync();
    }
    kernel::run(args);
}
