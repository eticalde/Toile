/// The longest a stem is cut to, which leaves room under any file system's
/// name limit for a collision suffix and the extension.
const LONGEST: usize = 60;

/// The stem of a name with nothing in it that folds to a Latin letter.
const NOBODY: &str = "persona";

/// A person's name as a library file's stem.
///
/// Lowercase ASCII letters, digits and `_` are kept, and every other run of
/// characters becomes one `-`. A Latin letter carrying a mark folds to its bare
/// letter, and a combining mark is dropped, so a name typed precomposed and the
/// same name pasted decomposed give one stem. An apostrophe or a Catalan middle
/// dot joins the letters around it instead of splitting them. A name in a
/// script with no Latin fold is filed as `persona`, and the collision suffix
/// tells such people apart.
pub(super) fn of(name: &str) -> String {
    let mut stem = String::new();
    let mut gap = false;
    let mut buf = [0; 4];
    for c in name.chars().flat_map(char::to_lowercase) {
        let bare = match c {
            'a'..='z' | '0'..='9' | '_' => &*c.encode_utf8(&mut buf),
            '\u{300}'..='\u{36f}' | '\'' | '\u{2019}' | '\u{b7}' => continue,
            _ => {
                if let Some(bare) = latin(c) {
                    bare
                } else {
                    gap = true;
                    continue;
                }
            }
        };
        if gap && !stem.is_empty() {
            stem.push('-');
        }
        gap = false;
        stem.push_str(bare);
        if stem.len() >= LONGEST {
            break;
        }
    }
    stem.truncate(LONGEST);
    let stem = stem.trim_end_matches('-');
    if stem.is_empty() {
        NOBODY.to_owned()
    } else {
        stem.to_owned()
    }
}

/// The bare letters for a lowercase letter of the Latin-1 and Latin
/// Extended-A blocks: every language of Spain and Latin America, and the
/// names their neighbours bring to a fitting.
fn latin(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĳ' => "ij",
        'ĵ' => "j",
        'ķ' | 'ĸ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŉ' | 'ŋ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'œ' => "oe",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' | 'ſ' => "s",
        'ß' => "ss",
        'ţ' | 'ť' | 'ŧ' => "t",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use toile_engine::draft::Origin;

    use super::*;

    #[test]
    fn a_spanish_name_folds_to_its_bare_letters() {
        for (name, stem) in [
            ("Begoña", "begona"),
            ("José Ángel", "jose-angel"),
            ("  Ana   María ", "ana-maria"),
            ("Núñez-Ibáñez", "nunez-ibanez"),
            ("MARÍA JOSÉ", "maria-jose"),
            ("Agüero", "aguero"),
            ("talla_42", "talla_42"),
        ] {
            assert_eq!(of(name), stem, "{name}");
        }
    }

    #[test]
    fn a_name_pasted_decomposed_gives_the_stem_it_gives_typed() {
        let decomposed = "Jose\u{301} A\u{301}ngel Begon\u{303}a";
        assert_eq!(of(decomposed), of("José Ángel Begoña"));
        assert_eq!(of(decomposed), "jose-angel-begona");
    }

    #[test]
    fn the_neighbours_fold_too() {
        for (name, stem) in [
            ("João Gonçalves", "joao-goncalves"),
            ("Montserrat Col·lell", "montserrat-collell"),
            ("Aitziber O'Brien", "aitziber-obrien"),
            ("Chloé D’Arcy", "chloe-darcy"),
            ("Straße", "strasse"),
            ("Łucja Øster", "lucja-oster"),
            ("İpek Çağla", "ipek-cagla"),
        ] {
            assert_eq!(of(name), stem, "{name}");
        }
    }

    #[test]
    fn a_name_that_folds_to_nothing_is_still_filed() {
        for name in ["", "   ", "李娜", "---", "¿?"] {
            assert_eq!(of(name), NOBODY, "{name:?}");
        }
        assert_eq!(of("Ана Ana"), "ana");
    }

    #[test]
    fn a_name_that_looks_like_a_path_stays_inside_the_library() {
        assert_eq!(of("../../etc/passwd"), "etc-passwd");
        assert_eq!(of("ana.toile-persona"), "ana-toile-persona");
        assert_eq!(of("C:\\Ana"), "c-ana");
    }

    #[test]
    fn a_long_name_is_cut_and_never_ends_on_a_dash() {
        let long = "Ana ".repeat(40);
        let stem = of(&long);
        assert!(stem.len() <= LONGEST, "{stem}");
        assert!(!stem.ends_with('-'), "{stem}");
        let exact = format!("{} b", "a".repeat(LONGEST - 1));
        assert_eq!(of(&exact), "a".repeat(LONGEST - 1));
    }

    #[test]
    fn every_stem_is_one_a_document_link_accepts() {
        let names = [
            "Begoña",
            "José Ángel",
            "李娜",
            "../x",
            "ẞ Ǆ ﬁ",
            "-_-",
            "Ana 2",
            &"ñ".repeat(90),
        ];
        for name in names {
            let stem = of(name);
            assert!(Origin::is_stem(&stem), "{name:?} gave {stem:?}");
        }
    }
}
