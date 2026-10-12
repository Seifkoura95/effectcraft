//! Community and project links (shown in Help ▸, the About dialog, the start screen and the
//! header's Discord button).

/// The product name written into files the engine makes (template manifests, system reports);
/// Simple Motion in the `simplehub` build.
pub const APP_NAME: &str = if cfg!(feature = "simplehub") { "Simple Motion" } else { "EffectCraft" };
pub const APP_SLUG: &str = "effectcraft";
pub const DISCORD: &str = "https://discord.gg/artcraft";
pub const WEBSITE: &str = "https://getartcraft.com";
pub const APP_PAGE: &str = "https://getartcraft.com/apps/effectcraft";
pub const GITHUB: &str = "https://github.com/storytold/effectcraft";
pub const ISSUES: &str = "https://github.com/storytold/effectcraft/issues";

/// Sibling apps of the ArtCraft family: (name, what it is, GitHub repo, app page).
pub const SIBLINGS: &[(&str, &str)] = &[
    ("PhotoCraft", "photocraft"),
    ("VectorCraft", "vectorcraft"),
    ("FilmCraft", "filmcraft"),
    ("LightCraft", "lightcraft"),
    ("PdfCraft", "pdfcraft"),
    ("DesignCraft", "designcraft"),
];

pub fn github_of(slug: &str) -> String {
    format!("https://github.com/storytold/{slug}")
}
pub fn page_of(slug: &str) -> String {
    format!("https://getartcraft.com/apps/{slug}")
}
