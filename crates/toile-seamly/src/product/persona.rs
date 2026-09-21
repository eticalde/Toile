use toile_doc::{Persona, Snapshot};

use crate::{Error, Product};

impl Product {
    /// The product's body as a person for the user's library, called `name`
    /// and measured on `taken`, written `YYYY-MM-DD`.
    ///
    /// Her tape is the product's body and nothing else: the measurements the
    /// mapping file names, and any other a formula of the pattern reads, so a
    /// product linked to her resolves exactly as it did. The measurement
    /// file's personal block cannot reach her, since `Measurements` never
    /// holds it. Her notes name the file she came from, `source`, and notes
    /// never leave the library.
    pub fn persona(&self, name: &str, taken: &str, source: &str) -> Persona {
        let values = self
            .doc
            .measures()
            .map(|set| set.values.clone())
            .unwrap_or_default();
        Persona {
            name: name.to_owned(),
            notes: format!(
                "Medidas importadas de «{source}» (Seamly2D). Los datos personales de ese archivo \
                 no se leen."
            ),
            taken: vec![Snapshot {
                date: taken.to_owned(),
                values,
                phenotype: None,
            }],
        }
    }

    /// Makes the product's body the copy of `persona` that a library link
    /// carries, filed under `stem`, and names the body after her.
    ///
    /// # Errors
    /// A person whose current tape is not the product's body, since the copy
    /// would move the product, and whatever `Persona::to_mannequin` refuses:
    /// no session, a stem no library file could have, or a session a document
    /// could not hold.
    pub fn link(&mut self, persona: &Persona, stem: &str) -> Result<(), Error> {
        let linked = persona
            .to_mannequin(stem)
            .map_err(|why| Error::Product(format!("`{}` cannot be linked: {why}", persona.name)))?;
        let key = self.doc.resolve_with;
        let Some(body) = self.doc.mannequins.get_mut(key) else {
            return Err(Error::Product("the product has no body to link".to_owned()));
        };
        if body.values != linked.values {
            return Err(Error::Product(format!(
                "`{}` is not measured as the product's body is",
                persona.name
            )));
        }
        *body = linked;
        persona.name.clone_into(&mut self.report.body);
        Ok(())
    }
}
