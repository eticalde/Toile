/// What the command says about the pattern it resolved.
mod said;

use toile_engine::draft::{Command, Doc, Draft, block};

/// Runs `toile doc`: a pattern, resolved and written out.
///
/// This is the headless door onto a pattern — the one a person reads over a
/// terminal and a language model reads over a pipe — so every number it prints
/// comes from the same resolution the viewer drapes. Without a path it reads
/// the base block the program carries, which is the file it ships.
pub fn run(args: &[String]) {
    crate::report::said(printed(args));
}

/// The pattern the arguments name, resolved, as the lines a person reads.
fn printed(args: &[String]) -> Result<Vec<String>, String> {
    let mut doc = asked_for(args)?;
    if let Some(name) = flag(args, "--resolve-with") {
        resolved_with(&mut doc, name)?;
    }
    let draft =
        Draft::from_doc(doc).map_err(|broken| format!("el documento no resuelve: {broken}"))?;
    Ok(said::pattern(&draft))
}

/// The pattern the arguments name: a file, or the block carried in.
fn asked_for(args: &[String]) -> Result<Doc, String> {
    let Some(path) = args.first().filter(|arg| !arg.starts_with("--")) else {
        return Ok(block::trousers());
    };
    let text =
        std::fs::read_to_string(path).map_err(|why| format!("no se pudo leer «{path}»: {why}"))?;
    Doc::from_json(&text).map_err(|why| format!("«{path}» no es un patrón: {why}"))
}

/// Resolves the document against the body it names, listing the bodies it does
/// carry when it carries no such one.
fn resolved_with(doc: &mut Doc, name: &str) -> Result<(), String> {
    let key = doc.mannequin_named(name).ok_or_else(|| {
        format!(
            "no hay ningún cuerpo llamado «{name}»\ncuerpos: {}",
            said::bodies(doc).join(", ")
        )
    })?;
    Command::ResolveWith { mannequin: key }
        .apply(doc)
        .map_err(|refused| format!("no se pudo resolver con «{name}»: {refused}"))?;
    Ok(())
}

/// The value written after `name`, when the arguments carry one.
fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let at = args.iter().position(|arg| arg == name)?;
    args.get(at + 1).map(String::as_str)
}

#[cfg(test)]
mod tests;
