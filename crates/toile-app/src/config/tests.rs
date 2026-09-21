use super::*;
use crate::library::tests::Scratch;

/// An environment holding only `pairs`.
fn env(pairs: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
    let pairs = pairs.to_vec();
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| OsString::from(value))
    }
}

/// A preferences file as a build from before the default person wrote it.
const OLD: &str = r#"{
  "version": 1,
  "window": [
    10.0,
    20.0,
    1320.0,
    780.0
  ],
  "maximized": false,
  "recents": [
    "/Users/ana/Documents/Toile/falda.toile"
  ],
  "last_dir": "/Users/ana/Documents/Toile"
}"#;

#[test]
fn with_no_home_there_is_no_base_directory() {
    assert_eq!(base_dir(Base::Config, env(&[])), None);
    assert_eq!(base_dir(Base::Data, env(&[])), None);
}

#[cfg(target_os = "macos")]
#[test]
fn on_macos_settings_and_data_share_application_support() {
    let home = env(&[("HOME", "/Users/ana"), ("XDG_DATA_HOME", "/elsewhere")]);
    let support = PathBuf::from("/Users/ana/Library/Application Support");
    assert_eq!(base_dir(Base::Config, &home), Some(support.clone()));
    assert_eq!(base_dir(Base::Data, &home), Some(support));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn elsewhere_data_follows_xdg_and_falls_back_under_home() {
    let bare = env(&[("HOME", "/home/ana")]);
    let data = base_dir(Base::Data, &bare);
    assert_eq!(data, Some(PathBuf::from("/home/ana/.local/share")));
    let config = base_dir(Base::Config, &bare);
    assert_eq!(config, Some(PathBuf::from("/home/ana/.config")));
    let set = env(&[("HOME", "/home/ana"), ("XDG_DATA_HOME", "/datos")]);
    assert_eq!(base_dir(Base::Data, &set), Some(PathBuf::from("/datos")));
    let relative = env(&[("HOME", "/home/ana"), ("XDG_DATA_HOME", "datos")]);
    let ignored = base_dir(Base::Data, &relative);
    assert_eq!(ignored, Some(PathBuf::from("/home/ana/.local/share")));
}

#[test]
fn a_file_from_before_the_default_person_reads_and_writes_back_byte_for_byte() {
    let scratch = Scratch::new("prefs-old");
    let file = scratch.beside("prefs.json");
    std::fs::write(&file, OLD).expect("a scratch file");

    let prefs = Prefs::load_from(&file);
    assert_eq!(prefs.default_persona, None);
    assert_eq!(prefs.window, Some([10.0, 20.0, 1320.0, 780.0]));
    assert_eq!(prefs.maximized, Some(false));
    assert_eq!(prefs.recents.len(), 1);
    prefs.save();
    let again = std::fs::read_to_string(&file).expect("written back");
    assert_eq!(again, OLD);
}

#[test]
fn a_default_person_survives_the_file_and_leaves_it_once_cleared() {
    let scratch = Scratch::new("prefs-default");
    let file = scratch.beside("prefs.json");
    std::fs::write(&file, OLD).expect("a scratch file");

    let mut prefs = Prefs::load_from(&file);
    prefs.toggle_default("etienne");
    prefs.save();
    let text = std::fs::read_to_string(&file).expect("written");
    assert!(text.contains("\"default_persona\": \"etienne\""), "{text}");
    let mut read = Prefs::load_from(&file);
    assert_eq!(read.default_persona.as_deref(), Some("etienne"));
    assert_eq!(read.recents, prefs.recents, "nothing else moved");

    read.toggle_default("etienne");
    read.save();
    assert_eq!(std::fs::read_to_string(&file).expect("written"), OLD);
}

#[test]
fn the_default_person_is_one_at_most_and_the_same_choice_twice_clears_it() {
    let mut prefs = Prefs::default();
    prefs.toggle_default("ana");
    assert_eq!(prefs.default_persona.as_deref(), Some("ana"));
    prefs.toggle_default("bea");
    assert_eq!(prefs.default_persona.as_deref(), Some("bea"), "it moves");
    prefs.toggle_default("bea");
    assert_eq!(prefs.default_persona, None);
}
