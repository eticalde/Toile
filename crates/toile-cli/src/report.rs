/// Prints what a subcommand has to say, and fails the process if it refused.
///
/// Every subcommand that writes something had this shape, and every one of them
/// had it wrong the same way: a refusal went to the error stream and the
/// process still exited zero. A person reads the line and knows; a script that
/// runs `toile pdf` over a folder of patterns cannot tell a garment it printed
/// from one it did not, and the folder it was writing into is where it would
/// find out — by there being nothing in it.
///
/// One place rather than four, because the rule is one rule: what a subcommand
/// says on the way out and what it leaves in `$?` are the same answer.
pub fn said(result: Result<Vec<String>, String>) {
    match result {
        Ok(lines) => lines.iter().for_each(|line| println!("{line}")),
        Err(why) => {
            eprintln!("{why}");
            std::process::exit(1);
        }
    }
}
