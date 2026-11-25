use anyhow::{Result, anyhow};
use gpui::SvgRenderer;

use crate::{MermaidTheme, css_color};

/// Text is measured by naiad with Arial's metrics, so it is drawn with Arial or
/// the metric-compatible Liberation Sans; the theme's font is the fallback.
const MEASURED_FONT_FAMILY: &str = "arial, \"Liberation Sans\"";

pub(super) fn render_mermaid(
    source: &str,
    theme: &MermaidTheme,
    _svg_renderer: &SvgRenderer,
) -> Result<String> {
    let options = naiad::Options::from_json_value(to_naiad_options(theme))
        .map_err(|error| anyhow!("invalid naiad options: {error}"))?;
    naiad::render_flat_svg(source, &options).map_err(|error| anyhow!("{error}"))
}

fn to_naiad_options(theme: &MermaidTheme) -> serde_json::Value {
    let primary = css_color(theme.primary_color);
    let primary_text = css_color(theme.primary_text_color);
    let primary_border = css_color(theme.primary_border_color);
    let line = css_color(theme.line_color);
    let secondary = css_color(theme.secondary_color);
    let tertiary = css_color(theme.tertiary_color);
    let background = css_color(theme.background);
    let cluster_bg = css_color(theme.cluster_background);
    let cluster_border = css_color(theme.cluster_border);
    let edge_label_bg = css_color(theme.edge_label_background);
    let text = css_color(theme.text_color);
    let note_bg = css_color(theme.note_background);
    let note_border = css_color(theme.note_border);
    let actor_bg = css_color(theme.actor_background);
    let actor_border = css_color(theme.actor_border);
    let activation_bg = css_color(theme.activation_background);
    let activation_border = css_color(theme.activation_border);
    let er_odd = css_color(theme.er_attr_bg_odd);
    let er_even = css_color(theme.er_attr_bg_even);
    let git: [String; 8] = theme.git_branch_colors.map(css_color);
    let git_lbl: [String; 8] = theme.git_branch_label_colors.map(css_color);

    let mut theme_vars = serde_json::json!({
        "primaryColor": primary,
        "primaryTextColor": primary_text,
        "primaryBorderColor": primary_border,
        "lineColor": line,
        "secondaryColor": secondary,
        "secondaryTextColor": text,
        "tertiaryColor": tertiary,
        "tertiaryTextColor": text,
        "background": background,
        "mainBkg": primary,
        "nodeBorder": primary_border,
        "nodeTextColor": primary_text,
        "clusterBkg": cluster_bg,
        "clusterBorder": cluster_border,
        "titleColor": text,
        "edgeLabelBackground": edge_label_bg,
        "textColor": text,
        "noteBkgColor": note_bg,
        "noteBorderColor": note_border,
        "noteTextColor": text,
        "actorBkg": actor_bg,
        "actorBorder": actor_border,
        "actorTextColor": primary_text,
        "labelTextColor": text,
        "loopTextColor": text,
        "signalColor": text,
        "signalTextColor": text,
        "activationBkgColor": activation_bg,
        "activationBorderColor": activation_border,
        "classText": text,
        "labelColor": primary_text,
        "attributeBackgroundColorOdd": er_odd,
        "attributeBackgroundColorEven": er_even,
        "pieTitleTextColor": text,
        "pieSectionTextColor": text,
        "pieLegendTextColor": text,
        "pieStrokeColor": primary_border,
        "pieOuterStrokeColor": primary_border,
        "quadrant1Fill": primary,
        "quadrant2Fill": primary,
        "quadrant3Fill": primary,
        "quadrant4Fill": primary,
        "quadrant1TextFill": text,
        "quadrant2TextFill": text,
        "quadrant3TextFill": text,
        "quadrant4TextFill": text,
        "quadrantPointFill": line,
        "quadrantPointTextFill": text,
        "quadrantTitleFill": text,
        "quadrantXAxisTextFill": text,
        "quadrantYAxisTextFill": text,
        "quadrantExternalBorderStrokeFill": primary_border,
        "quadrantInternalBorderStrokeFill": primary_border,
    });

    if let Some(map) = theme_vars.as_object_mut() {
        for (((i, color), label), pie_number) in git.iter().enumerate().zip(&git_lbl).zip(1..) {
            map.insert(format!("cScale{i}"), color.clone().into());
            map.insert(format!("cScaleLabel{i}"), label.clone().into());
            map.insert(format!("pie{pie_number}"), color.clone().into());
        }
    }

    let font_family = format!("{MEASURED_FONT_FAMILY}, {}", theme.font_family);
    if let Some(map) = theme_vars.as_object_mut() {
        map.insert("fontFamily".into(), font_family.clone().into());
    }

    serde_json::json!({
        "id": "zed-mermaid",
        "theme": "base",
        "fontFamily": font_family,
        // Links are not followed in the preview, so none are needed.
        "securityLevel": "strict",
        "themeVariables": theme_vars,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIAGRAMS: &[&str] = &[
        "flowchart TD\n    A[Hello] --> B[World]\n    B --> C{Decision}\n    C -->|Yes| D[OK]",
        "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi\n    Note over Alice,Bob: A note",
        "classDiagram\n    class Shelter {\n        -List~Animal~ animals\n    }",
        "stateDiagram-v2\n    [*] --> Active\n    Active --> [*]",
        "erDiagram\n    CUSTOMER ||--o{ ORDER : places",
        "gantt\n    title Plan\n    section A\n    Task :a1, 2024-01-01, 3d",
        "pie title Pets\n    \"Dogs\" : 386\n    \"Cats\" : 85",
        "gitGraph\n    commit\n    branch dev\n    commit\n    checkout main\n    merge dev",
        "mindmap\n  root((Root))\n    A\n    B",
        "timeline\n    title History\n    2020 : One\n    2021 : Two",
        "quadrantChart\n    title Q\n    A: [0.3, 0.6]",
        "xychart-beta\n    x-axis [a, b]\n    bar [1, 2]",
        "journey\n    title Day\n    section Work\n      Code: 5: Me",
        "architecture-beta\n    service db(database)[Database]",
        "block-beta\n    a b c",
        "kanban\n  Todo\n    [Write docs]",
    ];

    #[test]
    fn renders_every_diagram_type_as_drawable_svg() {
        let theme = MermaidTheme::default();
        for source in DIAGRAMS {
            let svg = render_mermaid(source, &theme, &svg_renderer())
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            assert!(!svg.contains("var("), "{source}: {svg}");
            let tree = usvg::Tree::from_str(&svg, &usvg::Options::default())
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            assert!(tree.root().has_children(), "{source}: {svg}");
            assert!(tree.size().width() > 0.0, "{source}");
        }
    }

    #[test]
    fn theme_colors_are_used() {
        let theme = MermaidTheme {
            primary_color: gpui::rgb(0x123456).into(),
            ..MermaidTheme::default()
        };
        let svg = render_mermaid("flowchart LR\n  A --> B", &theme, &svg_renderer()).unwrap();
        assert!(svg.contains("#123456"), "{svg}");
    }

    #[test]
    fn text_is_drawn_with_the_measured_font() {
        let svg = render_mermaid(
            "flowchart LR\n  A --> B",
            &MermaidTheme::default(),
            &svg_renderer(),
        )
        .unwrap();
        assert!(svg.contains("arial, &quot;Liberation Sans&quot;"), "{svg}");
    }

    #[test]
    fn invalid_diagrams_are_errors() {
        let theme = MermaidTheme::default();
        assert!(render_mermaid("not a diagram", &theme, &svg_renderer()).is_err());
        assert!(render_mermaid("flowchart LR\n  A -->", &theme, &svg_renderer()).is_err());
    }

    fn svg_renderer() -> SvgRenderer {
        SvgRenderer::new(std::sync::Arc::new(()))
    }
}
