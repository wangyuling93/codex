//! Ghostty terminal palettes adapted to Codex's ANSI syntax theme.

use std::sync::OnceLock;

use syntect::highlighting::Color;
use syntect::highlighting::Theme;
use two_face::theme::EmbeddedThemeName;

const GHOSTTY_THEME_DATA: &str = include_str!("ghostty_themes.tsv");
const ANSI_ALPHA_INDEX: u8 = 0x00;
const ANSI_ALPHA_DEFAULT: u8 = 0x01;
const OPAQUE_ALPHA: u8 = 0xFF;

type Rgb = [u8; 3];

struct GhosttyTheme {
    name: String,
    background: Rgb,
    foreground: Rgb,
    palette: [Rgb; 16],
}

static GHOSTTY_THEMES: OnceLock<Vec<GhosttyTheme>> = OnceLock::new();

fn themes() -> &'static [GhosttyTheme] {
    GHOSTTY_THEMES
        .get_or_init(|| {
            let mut themes: Vec<GhosttyTheme> = GHOSTTY_THEME_DATA
                .lines()
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(|line| {
                    let mut fields = line.split('\t');
                    let name = fields
                        .next()
                        .unwrap_or_else(|| panic!("Ghostty theme row has no name: {line}"));
                    let background = parse_rgb(
                        fields
                            .next()
                            .unwrap_or_else(|| panic!("Ghostty theme has no background: {name}")),
                    )
                    .unwrap_or_else(|| panic!("Ghostty theme has an invalid background: {name}"));
                    let foreground = parse_rgb(
                        fields
                            .next()
                            .unwrap_or_else(|| panic!("Ghostty theme has no foreground: {name}")),
                    )
                    .unwrap_or_else(|| panic!("Ghostty theme has an invalid foreground: {name}"));
                    let palette: Vec<Rgb> = fields
                        .map(|field| {
                            parse_rgb(field).unwrap_or_else(|| {
                                panic!("Ghostty theme has an invalid palette color: {name}")
                            })
                        })
                        .collect();
                    let palette = palette.try_into().unwrap_or_else(|palette: Vec<Rgb>| {
                        panic!(
                            "Ghostty theme must have 16 palette colors, found {}: {name}",
                            palette.len()
                        )
                    });
                    GhosttyTheme {
                        name: name.to_string(),
                        background,
                        foreground,
                        palette,
                    }
                })
                .collect();
            themes.sort_by(|left, right| left.name.cmp(&right.name));
            themes
        })
        .as_slice()
}

fn parse_rgb(value: &str) -> Option<Rgb> {
    if value.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(value.get(0..2)?, 16).ok()?,
        u8::from_str_radix(value.get(2..4)?, 16).ok()?,
        u8::from_str_radix(value.get(4..6)?, 16).ok()?,
    ])
}

fn find(name: &str) -> Option<&'static GhosttyTheme> {
    let themes = themes();
    themes
        .binary_search_by(|theme| theme.name.as_str().cmp(name))
        .ok()
        .map(|index| &themes[index])
}

pub(super) fn names() -> impl Iterator<Item = &'static str> {
    themes().iter().map(|theme| theme.name.as_str())
}

pub(super) fn contains(name: &str) -> bool {
    find(name).is_some()
}

pub(super) fn resolve(name: &str) -> Option<Theme> {
    let definition = find(name)?;
    let mut theme = two_face::theme::extra()
        .get(EmbeddedThemeName::Ansi)
        .clone();
    theme.name = Some(definition.name.clone());
    theme.settings.background = Some(syntect_color(definition.background));
    theme.settings.foreground = Some(syntect_color(definition.foreground));
    for item in &mut theme.scopes {
        item.style.foreground = item
            .style
            .foreground
            .map(|color| resolve_ansi_color(color, definition.foreground, &definition.palette));
        item.style.background = item
            .style
            .background
            .map(|color| resolve_ansi_color(color, definition.background, &definition.palette));
    }
    Some(theme)
}

fn resolve_ansi_color(color: Color, default: Rgb, palette: &[Rgb; 16]) -> Color {
    match color.a {
        ANSI_ALPHA_INDEX => palette
            .get(usize::from(color.r))
            .copied()
            .map(syntect_color)
            .unwrap_or(color),
        ANSI_ALPHA_DEFAULT => syntect_color(default),
        _ => color,
    }
}

fn syntect_color([r, g, b]: Rgb) -> Color {
    Color {
        r,
        g,
        b,
        a: OPAQUE_ALPHA,
    }
}
