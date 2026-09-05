mod fidelity;
mod incremental;
mod kernel;
mod scene;
mod seams;
mod topology;
mod trouser;

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
        incremental::run_sync();
        return trouser::run();
    }
    kernel::run(args);
}
