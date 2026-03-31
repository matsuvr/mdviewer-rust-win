use collections::HashMap;
use gpui::{
    Animation, AnimationExt, AnyElement, Context, ImageSource, RenderImage, StyledText, Task, img,
    pulsating_between,
};
use mermaid_rs_renderer::{LayoutConfig, Theme, compute_layout, parse_mermaid, render_svg};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use theme::Appearance;
use ui::prelude::*;

use crate::parser::{CodeBlockKind, MarkdownEvent, MarkdownTag};

use super::{Markdown, MarkdownStyle, ParsedMarkdown};

type MermaidDiagramCache = HashMap<ParsedMarkdownMermaidDiagramContents, Arc<CachedMermaidDiagram>>;

#[derive(Clone, Debug)]
pub(crate) struct ParsedMarkdownMermaidDiagram {
    pub(crate) content_range: Range<usize>,
    pub(crate) contents: ParsedMarkdownMermaidDiagramContents,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ParsedMarkdownMermaidDiagramContents {
    pub(crate) contents: SharedString,
    pub(crate) scale: u32,
}

#[derive(Default, Clone)]
pub(crate) struct MermaidState {
    cache: MermaidDiagramCache,
    order: Vec<ParsedMarkdownMermaidDiagramContents>,
}

struct CachedMermaidDiagram {
    render_image: Arc<OnceLock<anyhow::Result<Arc<RenderImage>>>>,
    fallback_image: Option<Arc<RenderImage>>,
    _task: Task<()>,
}

impl MermaidState {
    pub(crate) fn clear(&mut self) {
        self.cache.clear();
        self.order.clear();
    }

    fn get_fallback_image(
        idx: usize,
        old_order: &[ParsedMarkdownMermaidDiagramContents],
        new_order_len: usize,
        cache: &MermaidDiagramCache,
    ) -> Option<Arc<RenderImage>> {
        if old_order.len() != new_order_len {
            return None;
        }

        old_order.get(idx).and_then(|old_content| {
            cache.get(old_content).and_then(|old_cached| {
                old_cached
                    .render_image
                    .get()
                    .and_then(|result| result.as_ref().ok().cloned())
                    .or_else(|| old_cached.fallback_image.clone())
            })
        })
    }

    pub(crate) fn update(&mut self, parsed: &ParsedMarkdown, cx: &mut Context<Markdown>) {
        let mut new_order = Vec::new();
        for mermaid_diagram in parsed.mermaid_diagrams.values() {
            new_order.push(mermaid_diagram.contents.clone());
        }

        for (idx, new_content) in new_order.iter().enumerate() {
            if !self.cache.contains_key(new_content) {
                let fallback =
                    Self::get_fallback_image(idx, &self.order, new_order.len(), &self.cache);
                self.cache.insert(
                    new_content.clone(),
                    Arc::new(CachedMermaidDiagram::new(new_content.clone(), fallback, cx)),
                );
            }
        }

        let new_order_set: std::collections::HashSet<_> = new_order.iter().cloned().collect();
        self.cache
            .retain(|content, _| new_order_set.contains(content));
        self.order = new_order;
    }
}

impl CachedMermaidDiagram {
    fn new(
        contents: ParsedMarkdownMermaidDiagramContents,
        fallback_image: Option<Arc<RenderImage>>,
        cx: &mut Context<Markdown>,
    ) -> Self {
        let render_image = Arc::new(OnceLock::<anyhow::Result<Arc<RenderImage>>>::new());
        let render_image_clone = render_image.clone();
        let svg_renderer = cx.svg_renderer();
        let appearance = cx.theme().appearance;

        let task = cx.spawn(async move |this, cx| {
            let value = cx
                .background_spawn(async move {
                    // Use the lower-level pipeline so Zed can match GitHub's default/dark
                    // Mermaid themes and honor `%%{init}%%` directives without leaving pure Rust.
                    let svg_string = render_mermaid_svg(&contents.contents, appearance)?;
                    let scale = contents.scale as f32 / 100.0;
                    svg_renderer
                        .render_single_frame(svg_string.as_bytes(), scale, true)
                        .map_err(|error| anyhow::anyhow!("{error}"))
                })
                .await;
            let _ = render_image_clone.set(value);
            this.update(cx, |_, cx| {
                cx.notify();
            })
            .ok();
        });

        Self {
            render_image,
            fallback_image,
            _task: task,
        }
    }

    #[cfg(test)]
    fn new_for_test(
        render_image: Option<Arc<RenderImage>>,
        fallback_image: Option<Arc<RenderImage>>,
    ) -> Self {
        let result = Arc::new(OnceLock::new());
        if let Some(render_image) = render_image {
            let _ = result.set(Ok(render_image));
        }
        Self {
            render_image: result,
            fallback_image,
            _task: Task::ready(()),
        }
    }
}

fn render_mermaid_svg(source: &str, appearance: Appearance) -> anyhow::Result<String> {
    let parsed = parse_mermaid(source)?;
    let mut theme = github_mermaid_theme(appearance);
    let mut layout_config = LayoutConfig::default();

    if let Some(init) = parsed.init_config.as_ref() {
        apply_init_config(init, appearance, &mut theme, &mut layout_config);
    }

    let layout = compute_layout(&parsed.graph, &theme, &layout_config);
    Ok(render_svg(&layout, &theme, &layout_config))
}

fn github_mermaid_theme(appearance: Appearance) -> Theme {
    if appearance.is_light() {
        Theme::mermaid_default()
    } else {
        github_dark_mermaid_theme()
    }
}

fn github_dark_mermaid_theme() -> Theme {
    let mut theme = Theme::mermaid_default();
    theme.background = "#333333".to_string();
    theme.primary_color = "#1F2020".to_string();
    theme.primary_text_color = "#CCCCCC".to_string();
    theme.primary_border_color = "#CCCCCC".to_string();
    theme.line_color = "#CCCCCC".to_string();
    theme.secondary_color = "#494949".to_string();
    theme.tertiary_color = "#3F5258".to_string();
    theme.edge_label_background = "#181818".to_string();
    theme.cluster_background = "#302F3D".to_string();
    theme.cluster_border = "rgba(255, 255, 255, 0.25)".to_string();
    theme.sequence_actor_fill = "#1F2020".to_string();
    theme.sequence_actor_border = "#CCCCCC".to_string();
    theme.sequence_actor_line = "#CCCCCC".to_string();
    theme.sequence_note_fill = "#494949".to_string();
    theme.sequence_note_border = "rgba(255, 255, 255, 0.25)".to_string();
    theme.sequence_activation_fill = "#494949".to_string();
    theme.sequence_activation_border = "#CCCCCC".to_string();
    theme.text_color = "#CCCCCC".to_string();
    theme.git_commit_label_color = "#CCCCCC".to_string();
    theme.git_commit_label_background = "#494949".to_string();
    theme.git_tag_label_color = "#CCCCCC".to_string();
    theme.git_tag_label_background = "#1F2020".to_string();
    theme.git_tag_label_border = "#CCCCCC".to_string();
    theme.pie_title_text_color = "#F9FFFE".to_string();
    theme.pie_section_text_color = "#CCCCCC".to_string();
    theme.pie_legend_text_color = "#F9FFFE".to_string();
    theme
}

fn apply_init_config(
    init: &Value,
    appearance: Appearance,
    theme: &mut Theme,
    layout: &mut LayoutConfig,
) {
    if let Some(theme_name) = init.get("theme").and_then(Value::as_str) {
        *theme = match theme_name.to_ascii_lowercase().as_str() {
            "dark" => github_dark_mermaid_theme(),
            "modern" => Theme::modern(),
            "base" | "default" | "mermaid" | "neutral" => Theme::mermaid_default(),
            _ => github_mermaid_theme(appearance),
        };
    }

    if let Some(theme_vars) = init.get("themeVariables") {
        apply_theme_variables(theme, theme_vars);
    }

    if let Some(flowchart) = init.get("flowchart") {
        apply_flowchart_config(layout, flowchart);
    }
}

fn apply_theme_variables(theme: &mut Theme, theme_vars: &Value) {
    const GIT_COLOR_KEYS: [&str; 8] = [
        "git0", "git1", "git2", "git3", "git4", "git5", "git6", "git7",
    ];
    const GIT_INV_KEYS: [&str; 8] = [
        "gitInv0", "gitInv1", "gitInv2", "gitInv3", "gitInv4", "gitInv5", "gitInv6", "gitInv7",
    ];
    const GIT_BRANCH_LABEL_KEYS: [&str; 8] = [
        "gitBranchLabel0",
        "gitBranchLabel1",
        "gitBranchLabel2",
        "gitBranchLabel3",
        "gitBranchLabel4",
        "gitBranchLabel5",
        "gitBranchLabel6",
        "gitBranchLabel7",
    ];
    const PIE_KEYS: [&str; 12] = [
        "pie1", "pie2", "pie3", "pie4", "pie5", "pie6", "pie7", "pie8", "pie9", "pie10", "pie11",
        "pie12",
    ];

    let tag_label_border_explicit = json_string(theme_vars, "tagLabelBorder").is_some();
    let primary_border_override = json_string(theme_vars, "primaryBorderColor");

    if let Some(value) = json_string(theme_vars, "fontFamily") {
        theme.font_family = value;
    }
    if let Some(value) = json_f32(theme_vars, "fontSize") {
        theme.font_size = value;
    }
    if let Some(value) = json_string(theme_vars, "primaryColor") {
        theme.primary_color = value;
    }
    if let Some(value) = json_string(theme_vars, "primaryTextColor") {
        theme.primary_text_color = value;
    }
    if let Some(value) = json_string(theme_vars, "primaryBorderColor") {
        theme.primary_border_color = value;
    }
    if let Some(value) = json_string(theme_vars, "lineColor") {
        theme.line_color = value;
    }
    if let Some(value) = json_string(theme_vars, "secondaryColor") {
        theme.secondary_color = value;
    }
    if let Some(value) = json_string(theme_vars, "tertiaryColor") {
        theme.tertiary_color = value;
    }
    if let Some(value) = json_string(theme_vars, "textColor") {
        theme.text_color = value;
    }
    if let Some(value) = json_string(theme_vars, "edgeLabelBackground") {
        theme.edge_label_background = value;
    }
    if let Some(value) = json_string(theme_vars, "clusterBkg") {
        theme.cluster_background = value;
    }
    if let Some(value) = json_string(theme_vars, "clusterBorder") {
        theme.cluster_border = value;
    }
    if let Some(value) = json_string(theme_vars, "background") {
        theme.background = value;
    }
    if let Some(value) = json_string(theme_vars, "actorBkg") {
        theme.sequence_actor_fill = value;
    }
    if let Some(value) = json_string(theme_vars, "actorBorder") {
        theme.sequence_actor_border = value;
    }
    if let Some(value) = json_string(theme_vars, "actorLine") {
        theme.sequence_actor_line = value;
    }
    if let Some(value) = json_string(theme_vars, "noteBkg") {
        theme.sequence_note_fill = value;
    }
    if let Some(value) = json_string(theme_vars, "noteBorderColor") {
        theme.sequence_note_border = value;
    }
    if let Some(value) = json_string(theme_vars, "activationBkgColor") {
        theme.sequence_activation_fill = value;
    }
    if let Some(value) = json_string(theme_vars, "activationBorderColor") {
        theme.sequence_activation_border = value;
    }

    for (index, key) in GIT_COLOR_KEYS.iter().enumerate() {
        if let Some(value) = json_string(theme_vars, key) {
            theme.git_colors[index] = value;
        }
    }

    for (index, key) in GIT_INV_KEYS.iter().enumerate() {
        if let Some(value) = json_string(theme_vars, key) {
            theme.git_inv_colors[index] = value;
        }
    }

    for (index, key) in GIT_BRANCH_LABEL_KEYS.iter().enumerate() {
        if let Some(value) = json_string(theme_vars, key) {
            theme.git_branch_label_colors[index] = value;
        }
    }

    if let Some(value) = json_string(theme_vars, "commitLabelColor") {
        theme.git_commit_label_color = value;
    }
    if let Some(value) = json_string(theme_vars, "commitLabelBackground") {
        theme.git_commit_label_background = value;
    }
    if let Some(value) = json_string(theme_vars, "tagLabelColor") {
        theme.git_tag_label_color = value;
    }
    if let Some(value) = json_string(theme_vars, "tagLabelBackground") {
        theme.git_tag_label_background = value;
    }
    if let Some(value) = json_string(theme_vars, "tagLabelBorder") {
        theme.git_tag_label_border = value;
    }
    if !tag_label_border_explicit && primary_border_override.is_some() {
        theme.git_tag_label_border = theme.primary_border_color.clone();
    }

    for (index, key) in PIE_KEYS.iter().enumerate() {
        if let Some(value) = json_string(theme_vars, key) {
            theme.pie_colors[index] = value;
        }
    }

    if let Some(value) = json_f32(theme_vars, "pieTitleTextSize") {
        theme.pie_title_text_size = value;
    }
    if let Some(value) = json_string(theme_vars, "pieTitleTextColor") {
        theme.pie_title_text_color = value;
    }
    if let Some(value) = json_f32(theme_vars, "pieSectionTextSize") {
        theme.pie_section_text_size = value;
    }
    if let Some(value) = json_string(theme_vars, "pieSectionTextColor") {
        theme.pie_section_text_color = value;
    }
    if let Some(value) = json_f32(theme_vars, "pieLegendTextSize") {
        theme.pie_legend_text_size = value;
    }
    if let Some(value) = json_string(theme_vars, "pieLegendTextColor") {
        theme.pie_legend_text_color = value;
    }
    if let Some(value) = json_string(theme_vars, "pieStrokeColor") {
        theme.pie_stroke_color = value;
    }
    if let Some(value) = json_f32(theme_vars, "pieStrokeWidth") {
        theme.pie_stroke_width = value;
    }
    if let Some(value) = json_f32(theme_vars, "pieOuterStrokeWidth") {
        theme.pie_outer_stroke_width = value;
    }
    if let Some(value) = json_string(theme_vars, "pieOuterStrokeColor") {
        theme.pie_outer_stroke_color = value;
    }
    if let Some(value) = json_f32(theme_vars, "pieOpacity") {
        theme.pie_opacity = value;
    }
}

fn apply_flowchart_config(layout: &mut LayoutConfig, flowchart: &Value) {
    if let Some(value) = json_f32(flowchart, "nodeSpacing") {
        layout.node_spacing = value;
    }
    if let Some(value) = json_f32(flowchart, "rankSpacing") {
        layout.rank_spacing = value;
    }
    if let Some(value) = json_usize(flowchart, "orderPasses") {
        layout.flowchart.order_passes = value;
    }
    if let Some(value) = json_f32(flowchart, "portPadRatio") {
        layout.flowchart.port_pad_ratio = value;
    }
    if let Some(value) = json_f32(flowchart, "portPadMin") {
        layout.flowchart.port_pad_min = value;
    }
    if let Some(value) = json_f32(flowchart, "portPadMax") {
        layout.flowchart.port_pad_max = value;
    }
    if let Some(value) = json_f32(flowchart, "portSideBias") {
        layout.flowchart.port_side_bias = value;
    }

    if let Some(auto_spacing) = flowchart.get("autoSpacing") {
        if let Some(value) = json_bool(auto_spacing, "enabled") {
            layout.flowchart.auto_spacing.enabled = value;
        }
        if let Some(value) = json_f32(auto_spacing, "minSpacing") {
            layout.flowchart.auto_spacing.min_spacing = value;
        }
        if let Some(value) = json_f32(auto_spacing, "densityThreshold") {
            layout.flowchart.auto_spacing.density_threshold = value;
        }
        if let Some(value) = json_f32(auto_spacing, "denseScaleFloor") {
            layout.flowchart.auto_spacing.dense_scale_floor = value;
        }
        if let Some(buckets) = auto_spacing.get("buckets").and_then(Value::as_array) {
            let mut parsed_buckets = Vec::with_capacity(buckets.len());
            for bucket in buckets {
                let Some(min_nodes) = bucket.get("minNodes").and_then(Value::as_u64) else {
                    continue;
                };
                let Some(scale) = json_f32(bucket, "scale") else {
                    continue;
                };
                parsed_buckets.push(mermaid_rs_renderer::config::FlowchartAutoSpacingBucket {
                    min_nodes: min_nodes as usize,
                    scale,
                });
            }
            if !parsed_buckets.is_empty() {
                layout.flowchart.auto_spacing.buckets = parsed_buckets;
            }
        }
    }

    if let Some(routing) = flowchart.get("routing") {
        if let Some(value) = json_bool(routing, "enableGridRouter") {
            layout.flowchart.routing.enable_grid_router = value;
        }
        if let Some(value) = json_f32(routing, "gridCell") {
            layout.flowchart.routing.grid_cell = value;
        }
        if let Some(value) = json_f32(routing, "turnPenalty") {
            layout.flowchart.routing.turn_penalty = value;
        }
        if let Some(value) = json_f32(routing, "occupancyWeight") {
            layout.flowchart.routing.occupancy_weight = value;
        }
        if let Some(value) = json_usize(routing, "maxSteps") {
            layout.flowchart.routing.max_steps = value;
        }
        if let Some(value) = json_bool(routing, "snapPortsToGrid") {
            layout.flowchart.routing.snap_ports_to_grid = value;
        }
    }

    if let Some(objective) = flowchart.get("objective") {
        if let Some(value) = json_bool(objective, "enabled") {
            layout.flowchart.objective.enabled = value;
        }
        if let Some(value) = json_f32(objective, "maxAspectRatio") {
            layout.flowchart.objective.max_aspect_ratio = value;
        }
        if let Some(value) = json_usize(objective, "wrapMinGroups") {
            layout.flowchart.objective.wrap_min_groups = value;
        }
        if let Some(value) = json_f32(objective, "wrapMainGapScale") {
            layout.flowchart.objective.wrap_main_gap_scale = value;
        }
        if let Some(value) = json_f32(objective, "wrapCrossGapScale") {
            layout.flowchart.objective.wrap_cross_gap_scale = value;
        }
        if let Some(value) = json_usize(objective, "edgeRelaxPasses") {
            layout.flowchart.objective.edge_relax_passes = value;
        }
        if let Some(value) = json_f32(objective, "edgeGapFloorRatio") {
            layout.flowchart.objective.edge_gap_floor_ratio = value;
        }
        if let Some(value) = json_f32(objective, "edgeLabelWeight") {
            layout.flowchart.objective.edge_label_weight = value;
        }
        if let Some(value) = json_f32(objective, "endpointLabelWeight") {
            layout.flowchart.objective.endpoint_label_weight = value;
        }
        if let Some(value) = json_f32(objective, "backedgeCrossWeight") {
            layout.flowchart.objective.backedge_cross_weight = value;
        }
    }
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_f32(value: &Value, key: &str) -> Option<f32> {
    let value = value.get(key)?;
    value.as_f64().map(|value| value as f32).or_else(|| {
        value
            .as_str()?
            .trim()
            .trim_end_matches("px")
            .parse::<f32>()
            .ok()
    })
}

fn json_usize(value: &Value, key: &str) -> Option<usize> {
    let value = value.get(key)?;
    value
        .as_u64()
        .map(|value| value as usize)
        .or_else(|| value.as_str()?.trim().parse::<usize>().ok())
}

fn json_bool(value: &Value, key: &str) -> Option<bool> {
    let value = value.get(key)?;
    value.as_bool().or_else(|| match value.as_str()?.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    })
}

fn parse_mermaid_info(info: &str) -> Option<u32> {
    let mut parts = info.split_whitespace();
    if parts.next()? != "mermaid" {
        return None;
    }

    Some(
        parts
            .next()
            .and_then(|scale| scale.parse().ok())
            .unwrap_or(100)
            .clamp(10, 500),
    )
}

pub(crate) fn extract_mermaid_diagrams(
    source: &str,
    events: &[(Range<usize>, MarkdownEvent)],
) -> BTreeMap<usize, ParsedMarkdownMermaidDiagram> {
    let mut mermaid_diagrams = BTreeMap::default();

    for (source_range, event) in events {
        let MarkdownEvent::Start(MarkdownTag::CodeBlock { kind, metadata }) = event else {
            continue;
        };
        let CodeBlockKind::FencedLang(info) = kind else {
            continue;
        };
        let Some(scale) = parse_mermaid_info(info.as_ref()) else {
            continue;
        };

        let contents = source[metadata.content_range.clone()]
            .strip_suffix('\n')
            .unwrap_or(&source[metadata.content_range.clone()])
            .to_string();
        mermaid_diagrams.insert(
            source_range.start,
            ParsedMarkdownMermaidDiagram {
                content_range: metadata.content_range.clone(),
                contents: ParsedMarkdownMermaidDiagramContents {
                    contents: contents.into(),
                    scale,
                },
            },
        );
    }

    mermaid_diagrams
}

pub(crate) fn render_mermaid_diagram(
    parsed: &ParsedMarkdownMermaidDiagram,
    mermaid_state: &MermaidState,
    style: &MarkdownStyle,
) -> AnyElement {
    let cached = mermaid_state.cache.get(&parsed.contents);
    let mut container = div().w_full();
    container.style().refine(&style.code_block);

    if let Some(result) = cached.and_then(|cached| cached.render_image.get()) {
        match result {
            Ok(render_image) => container
                .child(
                    div().w_full().child(
                        img(ImageSource::Render(render_image.clone()))
                            .max_w_full()
                            .with_fallback(|| {
                                div()
                                    .child(Label::new("Failed to load mermaid diagram"))
                                    .into_any_element()
                            }),
                    ),
                )
                .into_any_element(),
            Err(_) => container
                .child(StyledText::new(parsed.contents.contents.clone()))
                .into_any_element(),
        }
    } else if let Some(fallback) = cached.and_then(|cached| cached.fallback_image.as_ref()) {
        container
            .child(
                div()
                    .w_full()
                    .child(
                        img(ImageSource::Render(fallback.clone()))
                            .max_w_full()
                            .with_fallback(|| {
                                div()
                                    .child(Label::new("Failed to load mermaid diagram"))
                                    .into_any_element()
                            }),
                    )
                    .with_animation(
                        "mermaid-fallback-pulse",
                        Animation::new(Duration::from_secs(2))
                            .repeat()
                            .with_easing(pulsating_between(0.6, 1.0)),
                        |element, delta| element.opacity(delta),
                    ),
            )
            .into_any_element()
    } else {
        container
            .child(
                Label::new("Rendering mermaid diagram...")
                    .color(Color::Muted)
                    .with_animation(
                        "mermaid-loading-pulse",
                        Animation::new(Duration::from_secs(2))
                            .repeat()
                            .with_easing(pulsating_between(0.4, 0.8)),
                        |label, delta| label.alpha(delta),
                    ),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CachedMermaidDiagram, MermaidDiagramCache, MermaidState,
        ParsedMarkdownMermaidDiagramContents, apply_init_config, extract_mermaid_diagrams,
        github_mermaid_theme, parse_mermaid_info, render_mermaid_svg,
    };
    use crate::{CodeBlockRenderer, Markdown, MarkdownElement, MarkdownOptions, MarkdownStyle};
    use collections::HashMap;
    use gpui::{Context, IntoElement, Render, RenderImage, TestAppContext, Window, size};
    use mermaid_rs_renderer::LayoutConfig;
    use serde_json::json;
    use std::sync::Arc;
    use theme::Appearance;
    use ui::prelude::*;

    fn ensure_theme_initialized(cx: &mut TestAppContext) {
        cx.update(|cx| {
            if !cx.has_global::<settings::SettingsStore>() {
                settings::init(cx);
            }
            if !cx.has_global::<theme::GlobalTheme>() {
                theme_settings::init(theme::LoadThemes::JustBase, cx);
            }
        });
    }

    fn render_markdown_with_options(
        markdown: &str,
        options: MarkdownOptions,
        cx: &mut TestAppContext,
    ) -> crate::RenderedText {
        struct TestWindow;

        impl Render for TestWindow {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
            }
        }

        ensure_theme_initialized(cx);

        let (_, cx) = cx.add_window_view(|_, _| TestWindow);
        let markdown = cx.new(|cx| {
            Markdown::new_with_options(markdown.to_string().into(), None, None, options, cx)
        });
        cx.run_until_parked();
        let (rendered, _) = cx.draw(
            Default::default(),
            size(px(600.0), px(600.0)),
            |_window, _cx| {
                MarkdownElement::new(markdown, MarkdownStyle::default()).code_block_renderer(
                    CodeBlockRenderer::Default {
                        copy_button: false,
                        copy_button_on_hover: false,
                        border: false,
                    },
                )
            },
        );
        rendered.text
    }

    fn mock_render_image(cx: &mut TestAppContext) -> Arc<RenderImage> {
        cx.update(|cx| {
            cx.svg_renderer()
                .render_single_frame(
                    br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"></svg>"#,
                    1.0,
                    true,
                )
                .unwrap()
        })
    }

    fn mermaid_contents(contents: &str) -> ParsedMarkdownMermaidDiagramContents {
        ParsedMarkdownMermaidDiagramContents {
            contents: contents.to_string().into(),
            scale: 100,
        }
    }

    fn mermaid_sequence(diagrams: &[&str]) -> Vec<ParsedMarkdownMermaidDiagramContents> {
        diagrams
            .iter()
            .map(|diagram| mermaid_contents(diagram))
            .collect()
    }

    fn mermaid_fallback(
        new_diagram: &str,
        new_full_order: &[ParsedMarkdownMermaidDiagramContents],
        old_full_order: &[ParsedMarkdownMermaidDiagramContents],
        cache: &MermaidDiagramCache,
    ) -> Option<Arc<RenderImage>> {
        let new_content = mermaid_contents(new_diagram);
        let idx = new_full_order
            .iter()
            .position(|diagram| diagram == &new_content)?;
        MermaidState::get_fallback_image(idx, old_full_order, new_full_order.len(), cache)
    }

    #[test]
    fn test_parse_mermaid_info() {
        assert_eq!(parse_mermaid_info("mermaid"), Some(100));
        assert_eq!(parse_mermaid_info("mermaid 150"), Some(150));
        assert_eq!(parse_mermaid_info("mermaid 5"), Some(10));
        assert_eq!(parse_mermaid_info("mermaid 999"), Some(500));
        assert_eq!(parse_mermaid_info("rust"), None);
    }

    #[test]
    fn test_extract_mermaid_diagrams_parses_scale() {
        let markdown = "```mermaid 150\ngraph TD;\n```\n\n```rust\nfn main() {}\n```";
        let events = crate::parser::parse_markdown_with_options(markdown, false).events;
        let diagrams = extract_mermaid_diagrams(markdown, &events);

        assert_eq!(diagrams.len(), 1);
        let diagram = diagrams.values().next().unwrap();
        assert_eq!(diagram.contents.contents, "graph TD;");
        assert_eq!(diagram.contents.scale, 150);
    }

    #[test]
    fn test_render_mermaid_svg_uses_github_dark_theme_for_dark_appearance() {
        let svg = render_mermaid_svg("flowchart LR\nA-->B", Appearance::Dark).unwrap();

        assert!(svg.contains("fill=\"#333333\""));
        assert!(svg.contains("fill=\"#1F2020\""));
        assert!(svg.contains("stroke=\"#CCCCCC\""));
    }

    #[test]
    fn test_render_mermaid_svg_honors_theme_override_in_init() {
        let svg = render_mermaid_svg(
            r#"%%{init: {"theme":"dark"}}%%
flowchart LR
A-->B"#,
            Appearance::Light,
        )
        .unwrap();

        assert!(svg.contains("fill=\"#333333\""));
        assert!(svg.contains("fill=\"#1F2020\""));
    }

    #[test]
    fn test_render_mermaid_svg_honors_theme_variable_overrides() {
        let svg = render_mermaid_svg(
            r##"%%{init: {"themeVariables":{"primaryColor":"#ff00ff","lineColor":"#00ff00","background":"#010203"}}}%%
flowchart LR
A-->B"##,
            Appearance::Light,
        )
        .unwrap();

        assert!(svg.contains("fill=\"#010203\""));
        assert!(svg.contains("fill=\"#ff00ff\""));
        assert!(svg.contains("stroke=\"#00ff00\""));
    }

    #[test]
    fn test_apply_init_config_honors_flowchart_spacing_overrides() {
        let init = json!({
            "flowchart": {
                "nodeSpacing": 120,
                "rankSpacing": 120,
                "orderPasses": 8,
                "routing": {
                    "gridCell": 24,
                    "snapPortsToGrid": false
                }
            }
        });
        let mut theme = github_mermaid_theme(Appearance::Light);
        let mut layout = LayoutConfig::default();

        apply_init_config(&init, Appearance::Light, &mut theme, &mut layout);

        assert_eq!(layout.node_spacing, 120.0);
        assert_eq!(layout.rank_spacing, 120.0);
        assert_eq!(layout.flowchart.order_passes, 8);
        assert_eq!(layout.flowchart.routing.grid_cell, 24.0);
        assert!(!layout.flowchart.routing.snap_ports_to_grid);
    }

    #[gpui::test]
    fn test_mermaid_fallback_on_edit(cx: &mut TestAppContext) {
        let old_full_order = mermaid_sequence(&["graph A", "graph B", "graph C"]);
        let new_full_order = mermaid_sequence(&["graph A", "graph B modified", "graph C"]);

        let svg_b = mock_render_image(cx);

        let mut cache: MermaidDiagramCache = HashMap::default();
        cache.insert(
            mermaid_contents("graph A"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                Some(mock_render_image(cx)),
                None,
            )),
        );
        cache.insert(
            mermaid_contents("graph B"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                Some(svg_b.clone()),
                None,
            )),
        );
        cache.insert(
            mermaid_contents("graph C"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                Some(mock_render_image(cx)),
                None,
            )),
        );

        let fallback =
            mermaid_fallback("graph B modified", &new_full_order, &old_full_order, &cache);

        assert_eq!(fallback.as_ref().map(|image| image.id), Some(svg_b.id));
    }

    #[gpui::test]
    fn test_mermaid_no_fallback_on_add_in_middle(cx: &mut TestAppContext) {
        let old_full_order = mermaid_sequence(&["graph A", "graph C"]);
        let new_full_order = mermaid_sequence(&["graph A", "graph NEW", "graph C"]);

        let mut cache: MermaidDiagramCache = HashMap::default();
        cache.insert(
            mermaid_contents("graph A"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                Some(mock_render_image(cx)),
                None,
            )),
        );
        cache.insert(
            mermaid_contents("graph C"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                Some(mock_render_image(cx)),
                None,
            )),
        );

        let fallback = mermaid_fallback("graph NEW", &new_full_order, &old_full_order, &cache);

        assert!(fallback.is_none());
    }

    #[gpui::test]
    fn test_mermaid_fallback_chains_on_rapid_edits(cx: &mut TestAppContext) {
        let old_full_order = mermaid_sequence(&["graph A", "graph B modified", "graph C"]);
        let new_full_order = mermaid_sequence(&["graph A", "graph B modified again", "graph C"]);

        let original_svg = mock_render_image(cx);

        let mut cache: MermaidDiagramCache = HashMap::default();
        cache.insert(
            mermaid_contents("graph A"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                Some(mock_render_image(cx)),
                None,
            )),
        );
        cache.insert(
            mermaid_contents("graph B modified"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                None,
                Some(original_svg.clone()),
            )),
        );
        cache.insert(
            mermaid_contents("graph C"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                Some(mock_render_image(cx)),
                None,
            )),
        );

        let fallback = mermaid_fallback(
            "graph B modified again",
            &new_full_order,
            &old_full_order,
            &cache,
        );

        assert_eq!(
            fallback.as_ref().map(|image| image.id),
            Some(original_svg.id)
        );
    }

    #[gpui::test]
    fn test_mermaid_fallback_with_duplicate_blocks_edit_second(cx: &mut TestAppContext) {
        let old_full_order = mermaid_sequence(&["graph A", "graph A", "graph B"]);
        let new_full_order = mermaid_sequence(&["graph A", "graph A edited", "graph B"]);

        let svg_a = mock_render_image(cx);

        let mut cache: MermaidDiagramCache = HashMap::default();
        cache.insert(
            mermaid_contents("graph A"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                Some(svg_a.clone()),
                None,
            )),
        );
        cache.insert(
            mermaid_contents("graph B"),
            Arc::new(CachedMermaidDiagram::new_for_test(
                Some(mock_render_image(cx)),
                None,
            )),
        );

        let fallback = mermaid_fallback("graph A edited", &new_full_order, &old_full_order, &cache);

        assert_eq!(fallback.as_ref().map(|image| image.id), Some(svg_a.id));
    }

    #[gpui::test]
    fn test_mermaid_rendering_replaces_code_block_text(cx: &mut TestAppContext) {
        let rendered = render_markdown_with_options(
            "```mermaid\ngraph TD;\n```",
            MarkdownOptions {
                render_mermaid_diagrams: true,
                ..Default::default()
            },
            cx,
        );

        let text = rendered
            .lines
            .iter()
            .map(|line| line.layout.wrapped_text())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(!text.contains("graph TD;"));
    }

    #[gpui::test]
    fn test_mermaid_source_anchor_maps_inside_block(cx: &mut TestAppContext) {
        struct TestWindow;

        impl Render for TestWindow {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
            }
        }

        ensure_theme_initialized(cx);

        let (_, cx) = cx.add_window_view(|_, _| TestWindow);
        let markdown = cx.new(|cx| {
            Markdown::new_with_options(
                "```mermaid\ngraph TD;\n```".into(),
                None,
                None,
                MarkdownOptions {
                    render_mermaid_diagrams: true,
                    ..Default::default()
                },
                cx,
            )
        });
        cx.run_until_parked();
        let render_image = mock_render_image(cx);
        markdown.update(cx, |markdown, _| {
            let contents = markdown
                .parsed_markdown
                .mermaid_diagrams
                .values()
                .next()
                .unwrap()
                .contents
                .clone();
            markdown.mermaid_state.cache.insert(
                contents.clone(),
                Arc::new(CachedMermaidDiagram::new_for_test(Some(render_image), None)),
            );
            markdown.mermaid_state.order = vec![contents];
        });

        let (rendered, _) = cx.draw(
            Default::default(),
            size(px(600.0), px(600.0)),
            |_window, _cx| {
                MarkdownElement::new(markdown.clone(), MarkdownStyle::default())
                    .code_block_renderer(CodeBlockRenderer::Default {
                        copy_button: false,
                        copy_button_on_hover: false,
                        border: false,
                    })
            },
        );

        let mermaid_diagram = markdown.update(cx, |markdown, _| {
            markdown
                .parsed_markdown
                .mermaid_diagrams
                .values()
                .next()
                .unwrap()
                .clone()
        });
        assert!(
            rendered
                .text
                .position_for_source_index(mermaid_diagram.content_range.start)
                .is_some()
        );
        assert!(
            rendered
                .text
                .position_for_source_index(mermaid_diagram.content_range.end.saturating_sub(1))
                .is_some()
        );
    }
}
