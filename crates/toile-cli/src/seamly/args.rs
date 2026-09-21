use std::path::PathBuf;

/// How the command is called, for every message that has to say it.
pub const USAGE: &str = "uso: toile seamly RUTA.sm2d [SALIDA.toile] [--persona NOMBRE --tomada \
                         AAAA-MM-DD --biblioteca DIR]";

/// What `toile seamly` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    /// The pattern.
    pub input: PathBuf,
    /// The product, beside the pattern unless named.
    pub output: PathBuf,
    /// The person to file in the library and link the product to, if any.
    pub persona: Option<Wanted>,
}

/// The person the body becomes in the user's library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wanted {
    /// Her name, as the library shows it.
    pub name: String,
    /// The day she was measured, as `YYYY-MM-DD`.
    pub taken: String,
    /// The library's folder.
    pub library: PathBuf,
}

/// Reads the arguments, or says why they make no sense.
///
/// The three flags of the person go together: a person with no day or no
/// library to live in is a mistake to point out, not a default to guess.
pub fn parse(args: &[String]) -> Result<Asked, String> {
    let mut paths = Vec::new();
    let (mut name, mut taken, mut library) = (None, None, None);
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let slot = match arg.as_str() {
            "--persona" => &mut name,
            "--tomada" => &mut taken,
            "--biblioteca" => &mut library,
            flag if flag.starts_with("--") => {
                return Err(format!("no existe la opción «{flag}»\n{USAGE}"));
            }
            path => {
                paths.push(path);
                continue;
            }
        };
        let value = rest
            .next()
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("«{arg}» necesita un valor\n{USAGE}"))?;
        if slot.replace(value.clone()).is_some() {
            return Err(format!("«{arg}» aparece dos veces\n{USAGE}"));
        }
    }
    let (input, output) = match paths.as_slice() {
        [input] => (
            PathBuf::from(input),
            PathBuf::from(input).with_extension("toile"),
        ),
        [input, output] => (PathBuf::from(input), PathBuf::from(output)),
        _ => return Err(USAGE.to_owned()),
    };
    let persona = match (name, taken, library) {
        (None, None, None) => None,
        (Some(name), _, _) if name.trim().is_empty() => {
            return Err("el nombre de la persona no puede quedar vacío".to_owned());
        }
        (Some(name), Some(taken), Some(library)) => Some(Wanted {
            name,
            taken,
            library: PathBuf::from(library),
        }),
        _ => {
            return Err(format!(
                "--persona, --tomada y --biblioteca van juntas\n{USAGE}"
            ));
        }
    };
    Ok(Asked {
        input,
        output,
        persona,
    })
}
