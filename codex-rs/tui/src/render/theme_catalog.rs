//! Theme discovery and resolution for the TUI.
//!
//! Resolution order (first match wins):
//! 1. two-face built-in themes (`parse_theme_name`)
//! 2. custom `{CODEX_HOME}/themes/{name}.tmTheme` when it parses
//! 3. bundled Ghostty terminal palettes (`ghostty-*`)
//!
//! Listing preserves the historical built-in + custom group (case-insensitive
//! sort), then appends Ghostty palettes that are not overridden by a custom
//! file of the same name.

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use syntect::highlighting::Theme;
use syntect::highlighting::ThemeSet;
use two_face::theme::EmbeddedThemeName;

use super::ghostty_themes;

/// Where a picker entry / resolved theme came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ThemeOrigin {
    /// Embedded two-face theme.
    Builtin,
    /// User `.tmTheme` under `{CODEX_HOME}/themes/`.
    Custom,
    /// Bundled Ghostty terminal palette adapted to the ANSI syntax map.
    Ghostty,
}

/// A theme available in the picker.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ThemeEntry {
    /// Kebab-case identifier used for config persistence and theme resolution.
    pub name: String,
    /// Provenance of this entry.
    pub origin: ThemeOrigin,
}

impl ThemeEntry {
    /// `true` when this entry was discovered from a `.tmTheme` file on disk.
    pub fn is_custom(&self) -> bool {
        matches!(self.origin, ThemeOrigin::Custom)
    }
}

/// All 32 two-face theme names in kebab-case, ordered alphabetically.
pub(super) const BUILTIN_THEME_NAMES: &[&str] = &[
    "1337",
    "ansi",
    "base16",
    "base16-256",
    "base16-eighties-dark",
    "base16-mocha-dark",
    "base16-ocean-dark",
    "base16-ocean-light",
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

/// Map a kebab-case theme name to the corresponding `EmbeddedThemeName`.
pub(super) fn parse_theme_name(name: &str) -> Option<EmbeddedThemeName> {
    match name {
        "ansi" => Some(EmbeddedThemeName::Ansi),
        "base16" => Some(EmbeddedThemeName::Base16),
        "base16-eighties-dark" => Some(EmbeddedThemeName::Base16EightiesDark),
        "base16-mocha-dark" => Some(EmbeddedThemeName::Base16MochaDark),
        "base16-ocean-dark" => Some(EmbeddedThemeName::Base16OceanDark),
        "base16-ocean-light" => Some(EmbeddedThemeName::Base16OceanLight),
        "base16-256" => Some(EmbeddedThemeName::Base16_256),
        "catppuccin-frappe" => Some(EmbeddedThemeName::CatppuccinFrappe),
        "catppuccin-latte" => Some(EmbeddedThemeName::CatppuccinLatte),
        "catppuccin-macchiato" => Some(EmbeddedThemeName::CatppuccinMacchiato),
        "catppuccin-mocha" => Some(EmbeddedThemeName::CatppuccinMocha),
        "coldark-cold" => Some(EmbeddedThemeName::ColdarkCold),
        "coldark-dark" => Some(EmbeddedThemeName::ColdarkDark),
        "dark-neon" => Some(EmbeddedThemeName::DarkNeon),
        "dracula" => Some(EmbeddedThemeName::Dracula),
        "github" => Some(EmbeddedThemeName::Github),
        "gruvbox-dark" => Some(EmbeddedThemeName::GruvboxDark),
        "gruvbox-light" => Some(EmbeddedThemeName::GruvboxLight),
        "inspired-github" => Some(EmbeddedThemeName::InspiredGithub),
        "1337" => Some(EmbeddedThemeName::Leet),
        "monokai-extended" => Some(EmbeddedThemeName::MonokaiExtended),
        "monokai-extended-bright" => Some(EmbeddedThemeName::MonokaiExtendedBright),
        "monokai-extended-light" => Some(EmbeddedThemeName::MonokaiExtendedLight),
        "monokai-extended-origin" => Some(EmbeddedThemeName::MonokaiExtendedOrigin),
        "nord" => Some(EmbeddedThemeName::Nord),
        "one-half-dark" => Some(EmbeddedThemeName::OneHalfDark),
        "one-half-light" => Some(EmbeddedThemeName::OneHalfLight),
        "solarized-dark" => Some(EmbeddedThemeName::SolarizedDark),
        "solarized-light" => Some(EmbeddedThemeName::SolarizedLight),
        "sublime-snazzy" => Some(EmbeddedThemeName::SublimeSnazzy),
        "two-dark" => Some(EmbeddedThemeName::TwoDark),
        "zenburn" => Some(EmbeddedThemeName::Zenburn),
        _ => None,
    }
}

/// Build the expected path for a custom theme file.
fn custom_theme_path(name: &str, codex_home: &Path) -> PathBuf {
    codex_home.join("themes").join(format!("{name}.tmTheme"))
}

/// Try to load a custom `.tmTheme` file from `{codex_home}/themes/{name}.tmTheme`.
pub(super) fn load_custom_theme(name: &str, codex_home: &Path) -> Option<Theme> {
    ThemeSet::get_theme(custom_theme_path(name, codex_home)).ok()
}

fn adaptive_default_theme_selection() -> (EmbeddedThemeName, &'static str) {
    match crate::terminal_palette::default_bg() {
        Some(bg) if crate::color::is_light(bg) => {
            (EmbeddedThemeName::CatppuccinLatte, "catppuccin-latte")
        }
        _ => (EmbeddedThemeName::CatppuccinMocha, "catppuccin-mocha"),
    }
}

fn adaptive_default_embedded_theme_name() -> EmbeddedThemeName {
    adaptive_default_theme_selection().0
}

/// Return the kebab-case name of the adaptive default syntax theme selected
/// from terminal background lightness.
pub(crate) fn adaptive_default_theme_name() -> &'static str {
    adaptive_default_theme_selection().1
}

/// Whether `name` resolves via builtin, valid custom file, or Ghostty catalog.
pub(super) fn theme_name_is_known(name: &str, codex_home: Option<&Path>) -> bool {
    if parse_theme_name(name).is_some() {
        return true;
    }
    if let Some(home) = codex_home
        && load_custom_theme(name, home).is_some()
    {
        return true;
    }
    ghostty_themes::contains(name)
}

/// Resolve a theme name to a `Theme`.
///
/// Order: two-face builtin → valid custom `.tmTheme` → Ghostty palette.
/// Returns `None` when the name is unknown.
pub(crate) fn resolve_theme_by_name(name: &str, codex_home: Option<&Path>) -> Option<Theme> {
    if let Some(embedded) = parse_theme_name(name) {
        return Some(two_face::theme::extra().get(embedded).clone());
    }
    if let Some(home) = codex_home
        && let Some(theme) = load_custom_theme(name, home)
    {
        return Some(theme);
    }
    ghostty_themes::resolve(name)
}

/// Resolve an optional configured name, falling back to the adaptive default.
pub(super) fn resolve_theme_with_override(name: Option<&str>, codex_home: Option<&Path>) -> Theme {
    if let Some(name) = name {
        if let Some(theme) = resolve_theme_by_name(name, codex_home) {
            return theme;
        }
        tracing::debug!("Theme \"{name}\" not recognized; using default theme");
    }
    two_face::theme::extra()
        .get(adaptive_default_embedded_theme_name())
        .clone()
}

/// Check whether a theme name resolves. Returns a user-facing warning when it
/// does not, or when a custom file exists but is invalid.
///
/// An invalid on-disk `.tmTheme` always produces a warning, even when a
/// bundled (two-face or Ghostty) theme of the same name can still be used.
pub(crate) fn validate_theme_name(name: Option<&str>, codex_home: Option<&Path>) -> Option<String> {
    let name = name?;
    let custom_theme_path_display = codex_home
        .map(|home| custom_theme_path(name, home).display().to_string())
        .unwrap_or_else(|| format!("$CODEX_HOME/themes/{name}.tmTheme"));

    if let Some(home) = codex_home {
        let custom_path = custom_theme_path(name, home);
        if custom_path.is_file() && load_custom_theme(name, home).is_none() {
            let fallback_hint =
                if parse_theme_name(name).is_some() || ghostty_themes::contains(name) {
                    "A bundled theme with this name will be used instead."
                } else {
                    "Falling back to the default theme."
                };
            return Some(format!(
                "Custom theme \"{name}\" at {custom_theme_path_display} could not \
                 be loaded (invalid .tmTheme format). {fallback_hint}"
            ));
        }
    }

    if theme_name_is_known(name, codex_home) {
        return None;
    }

    Some(format!(
        "Theme \"{name}\" not found. Using the default theme. \
         Available names include the built-in two-face themes and ghostty-* \
         palettes. To use a custom theme, place a .tmTheme file at \
         {custom_theme_path_display}."
    ))
}

/// List picker themes: sorted built-in + custom group, then Ghostty palettes.
pub(crate) fn list_available_themes(codex_home: Option<&Path>) -> Vec<ThemeEntry> {
    let mut entries: Vec<ThemeEntry> = BUILTIN_THEME_NAMES
        .iter()
        .map(|name| ThemeEntry {
            name: (*name).to_string(),
            origin: ThemeOrigin::Builtin,
        })
        .collect();

    if let Some(home) = codex_home {
        let themes_dir = home.join("themes");
        if let Ok(read_dir) = std::fs::read_dir(&themes_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("tmTheme")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    let name = stem.to_string();
                    let is_valid_theme = ThemeSet::get_theme(&path).is_ok();
                    if is_valid_theme && !entries.iter().any(|entry| entry.name == name) {
                        entries.push(ThemeEntry {
                            name,
                            origin: ThemeOrigin::Custom,
                        });
                    }
                }
            }
        }
    }

    // Preserve the original stable ordering of built-in and custom themes,
    // then place every bundled Ghostty palette after that existing group.
    entries.sort_by_cached_key(|entry| (entry.name.to_ascii_lowercase(), entry.name.clone()));
    let mut seen: HashSet<String> = entries.iter().map(|entry| entry.name.clone()).collect();
    for name in ghostty_themes::names() {
        if seen.insert(name.to_string()) {
            entries.push(ThemeEntry {
                name: name.to_string(),
                origin: ThemeOrigin::Ghostty,
            });
        }
    }

    entries
}

#[cfg(test)]
#[path = "theme_catalog_tests.rs"]
mod tests;
