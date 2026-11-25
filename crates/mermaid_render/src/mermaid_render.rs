// for a very big json! macro
#![recursion_limit = "256"]

//! Crate for rendering Mermaid diagram strings to SVG strings.
//!
//! The entrypoint to this crate is [`render_to_svg`].
//!
//! It takes a `&str` and a [`MermaidTheme`] and returns an SVG that
//! `usvg`/`resvg` can draw, styled to match the theme.
//!
//! Rendering is done by [`naiad`], a Rust port of Mermaid v13. naiad's
//! "flat" SVG writes every theme colour as a literal value instead of a CSS
//! variable and gives the root `<svg>` a fixed size, which is what `usvg`
//! needs. Zed's theme is passed to naiad as Mermaid `themeVariables`, so no
//! post-processing of the SVG is needed.
//!
//! naiad measures text with embedded Arial metrics, so diagrams are drawn
//! with Arial (or the metric-compatible Liberation Sans) to keep labels
//! inside their boxes. The theme's `font_family` is used only as a fallback.

mod color;
mod render;

use anyhow::Result;
use gpui::{Hsla, Rgba, SvgRenderer};

#[derive(Debug, Clone, Copy)]
pub struct AccentColor {
    pub foreground: Hsla,
    pub background: Hsla,
}

#[derive(Debug, Clone)]
pub struct MermaidTheme {
    pub dark_mode: bool,
    pub font_family: String,
    pub background: Hsla,
    pub primary_color: Hsla,
    pub primary_text_color: Hsla,
    pub primary_border_color: Hsla,
    pub secondary_color: Hsla,
    pub tertiary_color: Hsla,
    pub line_color: Hsla,
    pub text_color: Hsla,
    pub edge_label_background: Hsla,
    pub cluster_background: Hsla,
    pub cluster_border: Hsla,
    pub note_background: Hsla,
    pub note_border: Hsla,
    pub actor_background: Hsla,
    pub actor_border: Hsla,
    pub activation_background: Hsla,
    pub activation_border: Hsla,
    pub git_branch_colors: [Hsla; 8],
    pub git_branch_label_colors: [Hsla; 8],
    pub er_attr_bg_odd: Hsla,
    pub er_attr_bg_even: Hsla,
    pub error_color: Hsla,
    pub warning_color: Hsla,
    pub accent_colors: Vec<AccentColor>,
}

/// Default theme for testing.
#[cfg(any(test, feature = "test-support"))]
impl Default for MermaidTheme {
    fn default() -> Self {
        use gpui::{hsla, rgb};
        let git_branch_colors: [Hsla; 8] = [
            hsla(240.0 / 360.0, 1.0, 0.462_745_1, 1.0),
            hsla(60.0 / 360.0, 1.0, 0.435_294_12, 1.0),
            hsla(80.0 / 360.0, 1.0, 0.462_745_1, 1.0),
            hsla(210.0 / 360.0, 1.0, 0.462_745_1, 1.0),
            hsla(180.0 / 360.0, 1.0, 0.462_745_1, 1.0),
            hsla(150.0 / 360.0, 1.0, 0.462_745_1, 1.0),
            hsla(300.0 / 360.0, 1.0, 0.462_745_1, 1.0),
            hsla(0.0, 1.0, 0.462_745_1, 1.0),
        ];
        let git_branch_label_colors: [Hsla; 8] =
            git_branch_colors.map(crate::text_color_for_background);

        Self {
            dark_mode: false,
            font_family: "Inter, ui-sans-serif, system-ui, -apple-system, \"Segoe UI\", \"DejaVu Sans\", \"Liberation Sans\", sans-serif, \"Noto Color Emoji\", \"Apple Color Emoji\", \"Segoe UI Emoji\"".to_string(),
            background: rgb(0xFFFFFF).into(),
            primary_color: rgb(0xF8FAFC).into(),
            primary_text_color: rgb(0x0F172A).into(),
            primary_border_color: rgb(0x94A3B8).into(),
            secondary_color: rgb(0xE2E8F0).into(),
            tertiary_color: rgb(0xFFFFFF).into(),
            line_color: rgb(0x64748B).into(),
            text_color: rgb(0x0F172A).into(),
            edge_label_background: rgb(0xFFFFFF).into(),
            cluster_background: rgb(0xF1F5F9).into(),
            cluster_border: rgb(0xCBD5E1).into(),
            note_background: rgb(0xFFF7ED).into(),
            note_border: rgb(0xFDBA74).into(),
            actor_background: rgb(0xF8FAFC).into(),
            actor_border: rgb(0x94A3B8).into(),
            activation_background: rgb(0xE2E8F0).into(),
            activation_border: rgb(0x94A3B8).into(),
            git_branch_colors,
            git_branch_label_colors,
            er_attr_bg_odd: rgb(0x94A3B8).into(),
            er_attr_bg_even: rgb(0x0F172A).into(),
            error_color: rgb(0xDC2626).into(),
            warning_color: rgb(0xD97706).into(),
            accent_colors: Vec::new(),
        }
    }
}

/// Formats a color as a CSS hex color for embedding in SVG/CSS.
///
/// Emits `#rrggbb` for fully opaque colors and `#rrggbbaa` when the input
/// has any transparency, so translucent theme colors (e.g. `ghost_element_hover`
/// from Zed's UI palette) round-trip without silently losing their alpha.
pub(crate) fn css_color(color: Hsla) -> String {
    let rgba = Rgba::from(color);
    let r = (rgba.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (rgba.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (rgba.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = (rgba.a.clamp(0.0, 1.0) * 255.0).round() as u8;
    if a == 0xff {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

pub use color::text_color_for_background;

/// Whether `source` is a Mermaid diagram that [`render_to_svg`] can draw: naiad
/// detects its type (front matter and directives included).
pub fn is_supported_diagram(source: &str) -> bool {
    naiad::detect(source).is_some()
}

/// See the [module-level docs][crate] for more info.
#[ztracing::instrument(skip_all)]
pub fn render_to_svg(
    source: &str,
    theme: &MermaidTheme,
    svg_renderer: &SvgRenderer,
) -> Result<String> {
    render::render_mermaid(source, theme, svg_renderer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mermaid_diagram_with_mixed_weight_combining_marks_does_not_panic() {
        const IBM_PLEX_REGULAR: &[u8] =
            include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf");
        const IBM_PLEX_SEMIBOLD: &[u8] =
            include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf");

        let zalgo = "Ne\u{0301}\u{0302}\u{0303}\u{0304}\u{0306}\u{0307}\u{0308}\u{030a}d";
        let source = format!("flowchart TD\n  A[\"**{zalgo}** {zalgo}\"]");
        let svg = render_to_svg(&source, &MermaidTheme::default(), &svg_renderer())
            .expect("mermaid diagram should render to SVG");

        let mut db = usvg::fontdb::Database::new();
        db.load_font_data(IBM_PLEX_REGULAR.to_vec());
        db.load_font_data(IBM_PLEX_SEMIBOLD.to_vec());
        db.set_sans_serif_family("IBM Plex Sans");
        let options = usvg::Options {
            fontdb: std::sync::Arc::new(db),
            ..Default::default()
        };

        usvg::Tree::from_data(svg.as_bytes(), &options)
            .expect("rasterizing mermaid text should not panic");
    }

    /// An ER diagram whose attribute-block tokens begin with a multibyte
    /// UTF-8 character (e.g. CJK type/field names) must not panic while the
    /// lexer probes for the two-character `PK`/`FK`/`UK` keys.
    #[test]
    fn er_multibyte_attribute_does_not_crash() {
        let source = "erDiagram\n顧客 {\n  文字列 名前\n}";
        let _ = render_to_svg(source, &MermaidTheme::default(), &svg_renderer());
    }

    /// A flowchart with mutually nested subgraphs (`A` contains `B` and `B`
    /// contains `A`) is an invalid containment cycle. Rendering it must return
    /// gracefully rather than overflowing the stack and aborting the process.
    #[test]
    fn cyclic_subgraphs_do_not_crash() {
        let source = "flowchart TD\n  subgraph A\n    B\n  end\n  subgraph B\n    A\n  end";
        let result = render_to_svg(source, &MermaidTheme::default(), &svg_renderer());
        if let Err(err) = result {
            let message = format!("{err:#}");
            assert!(
                message.contains("cycle"),
                "expected a cycle-related error, got: {message}"
            );
        }
    }

    fn svg_renderer() -> SvgRenderer {
        SvgRenderer::new(std::sync::Arc::new(()))
    }
}
