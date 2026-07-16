use super::*;
use pretty_assertions::assert_eq;
use syntect::highlighting::Highlighter;
use syntect::parsing::Scope;
use two_face::theme::EmbeddedThemeName;

fn write_minimal_tmtheme(path: &std::path::Path) {
    std::fs::write(
        path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>name</key><string>Test</string>
<key>settings</key><array><dict>
<key>settings</key><dict>
<key>foreground</key><string>#FFFFFF</string>
<key>background</key><string>#000000</string>
</dict></dict></array>
</dict></plist>"#,
    )
    .unwrap();
}

fn assert_rgb(color: Option<syntect::highlighting::Color>, expected: (u8, u8, u8)) {
    let color = color.expect("expected color");
    assert_eq!((color.r, color.g, color.b), expected);
    assert_eq!(color.a, 0xFF);
}

fn scope_foreground(theme: &Theme, scope_name: &str) -> Option<syntect::highlighting::Color> {
    let highlighter = Highlighter::new(theme);
    let scope = Scope::new(scope_name).ok()?;
    highlighter.style_mod_for_stack(&[scope]).foreground
}

#[test]
fn parse_theme_name_covers_all_variants() {
    let known = [
        ("ansi", EmbeddedThemeName::Ansi),
        ("base16", EmbeddedThemeName::Base16),
        (
            "base16-eighties-dark",
            EmbeddedThemeName::Base16EightiesDark,
        ),
        ("base16-mocha-dark", EmbeddedThemeName::Base16MochaDark),
        ("base16-ocean-dark", EmbeddedThemeName::Base16OceanDark),
        ("base16-ocean-light", EmbeddedThemeName::Base16OceanLight),
        ("base16-256", EmbeddedThemeName::Base16_256),
        ("catppuccin-frappe", EmbeddedThemeName::CatppuccinFrappe),
        ("catppuccin-latte", EmbeddedThemeName::CatppuccinLatte),
        (
            "catppuccin-macchiato",
            EmbeddedThemeName::CatppuccinMacchiato,
        ),
        ("catppuccin-mocha", EmbeddedThemeName::CatppuccinMocha),
        ("coldark-cold", EmbeddedThemeName::ColdarkCold),
        ("coldark-dark", EmbeddedThemeName::ColdarkDark),
        ("dark-neon", EmbeddedThemeName::DarkNeon),
        ("dracula", EmbeddedThemeName::Dracula),
        ("github", EmbeddedThemeName::Github),
        ("gruvbox-dark", EmbeddedThemeName::GruvboxDark),
        ("gruvbox-light", EmbeddedThemeName::GruvboxLight),
        ("inspired-github", EmbeddedThemeName::InspiredGithub),
        ("1337", EmbeddedThemeName::Leet),
        ("monokai-extended", EmbeddedThemeName::MonokaiExtended),
        (
            "monokai-extended-bright",
            EmbeddedThemeName::MonokaiExtendedBright,
        ),
        (
            "monokai-extended-light",
            EmbeddedThemeName::MonokaiExtendedLight,
        ),
        (
            "monokai-extended-origin",
            EmbeddedThemeName::MonokaiExtendedOrigin,
        ),
        ("nord", EmbeddedThemeName::Nord),
        ("one-half-dark", EmbeddedThemeName::OneHalfDark),
        ("one-half-light", EmbeddedThemeName::OneHalfLight),
        ("solarized-dark", EmbeddedThemeName::SolarizedDark),
        ("solarized-light", EmbeddedThemeName::SolarizedLight),
        ("sublime-snazzy", EmbeddedThemeName::SublimeSnazzy),
        ("two-dark", EmbeddedThemeName::TwoDark),
        ("zenburn", EmbeddedThemeName::Zenburn),
    ];
    for (kebab, expected) in &known {
        assert_eq!(
            parse_theme_name(kebab),
            Some(*expected),
            "parse_theme_name({kebab:?}) did not return expected variant"
        );
    }
}

#[test]
fn parse_theme_name_returns_none_for_unknown() {
    assert_eq!(parse_theme_name("nonexistent-theme"), None);
    assert_eq!(parse_theme_name(""), None);
}

#[test]
fn load_custom_theme_from_tmtheme_file() {
    let dir = tempfile::tempdir().unwrap();
    let themes_dir = dir.path().join("themes");
    std::fs::create_dir(&themes_dir).unwrap();
    write_minimal_tmtheme(&themes_dir.join("test-custom.tmTheme"));
    let theme = load_custom_theme("test-custom", dir.path());
    assert!(theme.is_some(), "should load .tmTheme from themes dir");
}

#[test]
fn load_custom_theme_returns_none_for_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load_custom_theme("nonexistent", dir.path()).is_none());
}

#[test]
fn validate_theme_name_none_for_bundled() {
    assert!(validate_theme_name(Some("dracula"), /*codex_home*/ None).is_none());
    assert!(validate_theme_name(Some("nord"), Some(Path::new("/nonexistent"))).is_none());
    assert!(validate_theme_name(Some("ghostty-3024-day"), /*codex_home*/ None).is_none());
}

#[test]
fn validate_theme_name_none_when_no_override() {
    assert!(validate_theme_name(/*name*/ None, /*codex_home*/ None).is_none());
}

#[test]
fn validate_theme_name_warns_for_missing_custom() {
    let dir = tempfile::tempdir().unwrap();
    let warning = validate_theme_name(Some("my-fancy"), Some(dir.path()));
    assert!(warning.is_some(), "should warn when theme file is absent");
    let msg = warning.unwrap();
    assert!(
        msg.contains("my-fancy"),
        "warning should mention the theme name"
    );
    assert!(
        msg.contains("ghostty-*"),
        "warning should mention Ghostty palettes"
    );
}

#[test]
fn validate_theme_name_none_when_custom_file_is_valid() {
    let dir = tempfile::tempdir().unwrap();
    let themes_dir = dir.path().join("themes");
    std::fs::create_dir(&themes_dir).unwrap();
    write_minimal_tmtheme(&themes_dir.join("my-fancy.tmTheme"));
    assert!(
        validate_theme_name(Some("my-fancy"), Some(dir.path())).is_none(),
        "should not warn when custom .tmTheme file parses successfully"
    );
}

#[test]
fn validate_theme_name_warns_when_custom_file_is_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let themes_dir = dir.path().join("themes");
    std::fs::create_dir(&themes_dir).unwrap();
    std::fs::write(themes_dir.join("my-fancy.tmTheme"), "placeholder").unwrap();
    let warning = validate_theme_name(Some("my-fancy"), Some(dir.path()));
    assert!(
        warning.is_some(),
        "should warn when custom .tmTheme exists but cannot be parsed"
    );
    assert!(
        warning
            .as_deref()
            .is_some_and(|msg| msg.contains("could not be loaded")),
        "warning should explain that the theme file is invalid"
    );
    assert!(
        warning
            .as_deref()
            .is_some_and(|msg| msg.contains("Falling back to the default theme")),
        "warning should describe default fallback when no bundled theme matches"
    );
}

#[test]
fn validate_theme_name_warns_for_invalid_custom_even_when_ghostty_matches() {
    let dir = tempfile::tempdir().unwrap();
    let themes_dir = dir.path().join("themes");
    std::fs::create_dir(&themes_dir).unwrap();
    let name = "ghostty-3024-day";
    std::fs::write(themes_dir.join(format!("{name}.tmTheme")), "not a plist").unwrap();

    let warning = validate_theme_name(Some(name), Some(dir.path()));
    assert!(
        warning.is_some(),
        "invalid custom file must warn even when a Ghostty palette shares the name"
    );
    assert!(
        warning
            .as_deref()
            .is_some_and(|msg| msg.contains("could not be loaded")),
        "warning should explain that the theme file is invalid"
    );
    assert!(
        warning
            .as_deref()
            .is_some_and(|msg| msg.contains("bundled theme")),
        "warning should note that the bundled Ghostty theme will be used"
    );

    let theme = resolve_theme_by_name(name, Some(dir.path()))
        .expect("bundled Ghostty theme should still resolve");
    assert_eq!(theme.name.as_deref(), Some(name));
}

#[test]
fn list_available_themes_excludes_invalid_custom_files() {
    let dir = tempfile::tempdir().unwrap();
    let themes_dir = dir.path().join("themes");
    std::fs::create_dir(&themes_dir).unwrap();
    write_minimal_tmtheme(&themes_dir.join("valid-custom.tmTheme"));
    std::fs::write(themes_dir.join("broken-custom.tmTheme"), "not a plist").unwrap();

    let entries = list_available_themes(Some(dir.path()));

    assert!(
        entries
            .iter()
            .any(|entry| entry.name == "valid-custom" && entry.is_custom()),
        "expected valid custom theme to be listed"
    );
    assert!(
        !entries
            .iter()
            .any(|entry| entry.name == "broken-custom" && entry.is_custom()),
        "expected invalid custom theme to be excluded from list"
    );
}

#[test]
fn custom_theme_takes_precedence_over_ghostty_theme_with_same_name() {
    let dir = tempfile::tempdir().unwrap();
    let themes_dir = dir.path().join("themes");
    std::fs::create_dir(&themes_dir).unwrap();
    let name = "ghostty-3024-day";
    write_minimal_tmtheme(&themes_dir.join(format!("{name}.tmTheme")));
    let expected = load_custom_theme(name, dir.path()).expect("expected custom theme to load");

    assert!(validate_theme_name(Some(name), Some(dir.path())).is_none());
    assert_eq!(
        resolve_theme_with_override(Some(name), Some(dir.path())),
        expected
    );
    assert_eq!(
        resolve_theme_by_name(name, Some(dir.path())),
        Some(expected)
    );
    let matching_entries: Vec<ThemeEntry> = list_available_themes(Some(dir.path()))
        .into_iter()
        .filter(|entry| entry.name == name)
        .collect();
    assert_eq!(
        matching_entries,
        vec![ThemeEntry {
            name: name.to_string(),
            origin: ThemeOrigin::Custom,
        }]
    );
}

#[test]
fn list_available_themes_preserves_existing_order_before_ghostty_group() {
    let dir = tempfile::tempdir().unwrap();
    let themes_dir = dir.path().join("themes");
    std::fs::create_dir(&themes_dir).unwrap();
    write_minimal_tmtheme(&themes_dir.join("zzz-custom.tmTheme"));
    write_minimal_tmtheme(&themes_dir.join("Aaa-custom.tmTheme"));
    write_minimal_tmtheme(&themes_dir.join("mmm-custom.tmTheme"));

    let entries = list_available_themes(Some(dir.path()));
    let mut expected: Vec<ThemeEntry> = BUILTIN_THEME_NAMES
        .iter()
        .map(|name| ThemeEntry {
            name: (*name).to_string(),
            origin: ThemeOrigin::Builtin,
        })
        .collect();
    expected.extend(
        ["Aaa-custom", "mmm-custom", "zzz-custom"].map(|name| ThemeEntry {
            name: name.to_string(),
            origin: ThemeOrigin::Custom,
        }),
    );
    expected.sort_by_cached_key(|entry| (entry.name.to_ascii_lowercase(), entry.name.clone()));
    expected.extend(ghostty_themes::names().map(|name| ThemeEntry {
        name: name.to_string(),
        origin: ThemeOrigin::Ghostty,
    }));

    assert_eq!(entries, expected);
}

#[test]
fn resolve_theme_with_override_delegates_to_resolve_theme_by_name() {
    let theme = resolve_theme_with_override(Some("ghostty-3024-day"), /*codex_home*/ None);
    let expected =
        resolve_theme_by_name("ghostty-3024-day", /*codex_home*/ None).expect("ghostty theme");
    assert_eq!(theme, expected);

    // Unknown names fall back to the adaptive default theme object.
    let unknown = resolve_theme_with_override(Some("no-such-theme"), /*codex_home*/ None);
    let theme_set = two_face::theme::extra();
    let default = theme_set.get(adaptive_default_embedded_theme_name());
    assert_eq!(unknown, default.clone());
}

#[test]
fn ghostty_theme_maps_known_palette_to_rgb_colors() {
    let name = "ghostty-3024-day";
    let theme = resolve_theme_by_name(name, /*codex_home*/ None)
        .unwrap_or_else(|| panic!("expected bundled Ghostty theme to resolve: {name}"));

    assert_eq!(theme.name.as_deref(), Some(name));
    assert_rgb(theme.settings.background, (0xf7, 0xf7, 0xf7));
    assert_rgb(theme.settings.foreground, (0x4a, 0x45, 0x43));
    let expected_scope_colors = [
        ("keyword", (0xa1, 0x6a, 0x94)),
        ("entity.name.function", (0x01, 0xa0, 0xe4)),
        ("constant.numeric", (0xca, 0xba, 0x00)),
        ("string", (0x01, 0xa2, 0x52)),
    ];
    for (scope, expected) in expected_scope_colors {
        assert_rgb(scope_foreground(&theme, scope), expected);
    }
}

#[test]
fn ghostty_theme_names_are_prefixed_and_unique() {
    let names: Vec<&str> = ghostty_themes::names().collect();
    assert!(names.iter().all(|name| name.starts_with("ghostty-")));
    let unique_names: std::collections::HashSet<&str> = names.iter().copied().collect();
    assert_eq!(unique_names.len(), names.len());
}

#[test]
fn ghostty_resolve_returns_cached_equal_themes() {
    let name = "ghostty-3024-day";
    let first = resolve_theme_by_name(name, /*codex_home*/ None).expect("first resolve");
    let second = resolve_theme_by_name(name, /*codex_home*/ None).expect("second resolve");
    assert_eq!(first, second);
}

#[test]
fn parse_theme_name_is_exhaustive() {
    use two_face::theme::EmbeddedLazyThemeSet;

    // Every variant in the embedded set must be reachable via parse_theme_name.
    let all_variants = EmbeddedLazyThemeSet::theme_names();

    // Guard: if two-face adds themes, this test forces us to update the mapping.
    assert_eq!(
        all_variants.len(),
        32,
        "two-face theme count changed — update parse_theme_name"
    );

    // Build the set of variants reachable through our kebab-case mapping.
    let kebab_names = [
        "ansi",
        "base16",
        "base16-eighties-dark",
        "base16-mocha-dark",
        "base16-ocean-dark",
        "base16-ocean-light",
        "base16-256",
        "catppuccin-frappe",
        "catppuccin-latte",
        "catppuccin-macchiato",
        "catppuccin-mocha",
        "coldark-cold",
        "coldark-dark",
        "dark-neon",
        "dracula",
        "github",
        "gruvbox-dark",
        "gruvbox-light",
        "inspired-github",
        "1337",
        "monokai-extended",
        "monokai-extended-bright",
        "monokai-extended-light",
        "monokai-extended-origin",
        "nord",
        "one-half-dark",
        "one-half-light",
        "solarized-dark",
        "solarized-light",
        "sublime-snazzy",
        "two-dark",
        "zenburn",
    ];
    let mapped: Vec<EmbeddedThemeName> = kebab_names
        .iter()
        .map(|k| parse_theme_name(k).unwrap_or_else(|| panic!("unmapped kebab name: {k}")))
        .collect();

    // Every variant from two-face must appear in our mapped set.
    for variant in all_variants {
        assert!(
            mapped.contains(variant),
            "EmbeddedThemeName::{variant:?} has no kebab-case mapping in parse_theme_name"
        );
    }
}
