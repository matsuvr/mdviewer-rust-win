use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
    sync::{Arc, LazyLock, Mutex},
};

use ab_glyph::{Font, FontArc, OutlineCurve};
use gpui::{
    AnyElement, App, AssetSource, Image, ImageFormat, ImageSource, Pixels, RenderImage, Rgba,
    Window, img, px,
};
use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_parser::parse;
use ratex_types::{Color as MathColor, DisplayItem, DisplayList, MathStyle, PathCommand};
use ui::prelude::*;

use crate::{MarkdownStyle, parser::MarkdownEvent};

#[derive(Clone, Debug)]
pub(crate) struct ParsedMarkdownMath {
    pub(crate) display_mode: bool,
    display_list: DisplayList,
}

#[derive(Clone)]
pub(crate) struct RenderedMarkdownMath {
    pub(crate) display_mode: bool,
    pub(crate) image: Arc<RenderImage>,
    pub(crate) width: Pixels,
    pub(crate) height: Pixels,
    pub(crate) baseline_from_top: Pixels,
}

type OutlineFontCache = HashMap<&'static str, FontArc>;

static OUTLINE_FONT_CACHE: LazyLock<Mutex<Option<Arc<OutlineFontCache>>>> =
    LazyLock::new(|| Mutex::new(None));

const KATEX_FONT_ASSETS: [(&str, &str); 18] = [
    ("AMS-Regular", "fonts/katex/KaTeX_AMS-Regular.ttf"),
    (
        "Caligraphic-Regular",
        "fonts/katex/KaTeX_Caligraphic-Regular.ttf",
    ),
    ("Fraktur-Regular", "fonts/katex/KaTeX_Fraktur-Regular.ttf"),
    ("Main-Bold", "fonts/katex/KaTeX_Main-Bold.ttf"),
    ("Main-BoldItalic", "fonts/katex/KaTeX_Main-BoldItalic.ttf"),
    ("Main-Italic", "fonts/katex/KaTeX_Main-Italic.ttf"),
    ("Main-Regular", "fonts/katex/KaTeX_Main-Regular.ttf"),
    ("Math-BoldItalic", "fonts/katex/KaTeX_Math-BoldItalic.ttf"),
    ("Math-Italic", "fonts/katex/KaTeX_Math-Italic.ttf"),
    ("SansSerif-Bold", "fonts/katex/KaTeX_SansSerif-Bold.ttf"),
    ("SansSerif-Italic", "fonts/katex/KaTeX_SansSerif-Italic.ttf"),
    (
        "SansSerif-Regular",
        "fonts/katex/KaTeX_SansSerif-Regular.ttf",
    ),
    ("Script-Regular", "fonts/katex/KaTeX_Script-Regular.ttf"),
    ("Size1-Regular", "fonts/katex/KaTeX_Size1-Regular.ttf"),
    ("Size2-Regular", "fonts/katex/KaTeX_Size2-Regular.ttf"),
    ("Size3-Regular", "fonts/katex/KaTeX_Size3-Regular.ttf"),
    ("Size4-Regular", "fonts/katex/KaTeX_Size4-Regular.ttf"),
    (
        "Typewriter-Regular",
        "fonts/katex/KaTeX_Typewriter-Regular.ttf",
    ),
];

pub(crate) fn extract_math_formulas(
    events: &[(Range<usize>, MarkdownEvent)],
) -> BTreeMap<usize, ParsedMarkdownMath> {
    let mut formulas = BTreeMap::default();

    for (range, event) in events {
        let MarkdownEvent::Math {
            display_mode,
            content,
        } = event
        else {
            continue;
        };

        let expression = if *display_mode {
            content.trim()
        } else {
            content.as_str()
        };
        let math_style = if *display_mode {
            MathStyle::Display
        } else {
            MathStyle::Text
        };

        match parse(expression) {
            Ok(ast) => {
                let layout_box = layout(&ast, &LayoutOptions::default().with_style(math_style));
                formulas.insert(
                    range.start,
                    ParsedMarkdownMath {
                        display_mode: *display_mode,
                        display_list: to_display_list(&layout_box),
                    },
                );
            }
            Err(error) => {
                log::debug!("failed to parse markdown math at {}: {error}", range.start);
            }
        }
    }

    formulas
}

pub(crate) fn render_markdown_math(
    math: &ParsedMarkdownMath,
    style: &MarkdownStyle,
    window: &Window,
    cx: &App,
) -> Option<RenderedMarkdownMath> {
    let base_font_size = style.base_text_style.font_size.to_pixels(window.rem_size());
    let font_size_px = base_font_size.to_f64();
    let font_size_px = if math.display_mode {
        font_size_px * 1.12
    } else {
        font_size_px
    };
    let padding_em = if math.display_mode { 0.30 } else { 0.16 };
    let default_color = style.base_text_style.color.to_rgb();
    let outline_font_cache = outline_font_cache(cx);
    let svg = render_display_list_to_svg(
        &math.display_list,
        font_size_px,
        padding_em,
        default_color,
        outline_font_cache.as_deref(),
    );
    let width_px = ((math.display_list.width + (padding_em * 2.0)) * font_size_px).max(1.0);
    let height_px = ((math.display_list.height + math.display_list.depth + (padding_em * 2.0))
        * font_size_px)
        .max(1.0);
    let baseline_from_top_px = (((math.display_list.height + padding_em) * font_size_px)
        + inline_math_baseline_adjustment(
            math.display_mode,
            style,
            base_font_size,
            font_size_px,
            math.display_list.height + math.display_list.depth,
            window,
        ))
    .clamp(0.0, height_px);

    match Image::from_bytes(ImageFormat::Svg, svg.into_bytes()).to_image_data(cx.svg_renderer()) {
        Ok(render_image) => Some(RenderedMarkdownMath {
            display_mode: math.display_mode,
            image: render_image,
            width: px(width_px as f32),
            height: px(height_px as f32),
            baseline_from_top: px(baseline_from_top_px as f32),
        }),
        Err(error) => {
            log::debug!("failed to rasterize markdown math svg: {error}");
            None
        }
    }
}

impl RenderedMarkdownMath {
    pub(crate) fn into_display_element(&self) -> AnyElement {
        let math_image = img(ImageSource::Render(self.image.clone()))
            .w(self.width)
            .h(self.height);

        if self.display_mode {
            div()
                .w_full()
                .flex()
                .justify_center()
                .my_1()
                .child(math_image.max_w_full())
                .into_any_element()
        } else {
            math_image.into_any_element()
        }
    }
}

fn inline_math_baseline_adjustment(
    display_mode: bool,
    style: &MarkdownStyle,
    base_font_size: Pixels,
    math_font_size_px: f64,
    math_total_height_em: f64,
    window: &Window,
) -> f64 {
    if display_mode || math_font_size_px <= 0.0 {
        return 0.0;
    }

    let base_font_id = window
        .text_system()
        .resolve_font(&style.base_text_style.font());
    let text_x_height_px = window
        .text_system()
        .x_height(base_font_id, base_font_size)
        .to_f64();
    let text_cap_height_px = window
        .text_system()
        .cap_height(base_font_id, base_font_size)
        .to_f64();
    let cap_height_bias_px = (text_cap_height_px - text_x_height_px).max(0.0) * 0.40;
    let tallness_bias_factor = inline_math_tallness_bias_factor(math_total_height_em);

    (inline_math_baseline_adjustment_for_x_height(text_x_height_px, math_font_size_px)
        + cap_height_bias_px * tallness_bias_factor)
        .clamp(0.0, math_font_size_px * 0.18)
}

fn inline_math_baseline_adjustment_for_x_height(
    text_x_height_px: f64,
    math_font_size_px: f64,
) -> f64 {
    let math_x_height_px = LayoutOptions::default()
        .with_style(MathStyle::Text)
        .metrics()
        .x_height
        * math_font_size_px;

    // TeX math fonts sit lower than the UI text font on Windows. Raising inline
    // math by the x-height delta keeps punctuation and surrounding prose on a
    // more stable optical baseline.
    (text_x_height_px - math_x_height_px).clamp(0.0, math_font_size_px * 0.12)
}

fn inline_math_tallness_bias_factor(math_total_height_em: f64) -> f64 {
    (0.2 + ((math_total_height_em - 0.70) / 0.35).clamp(0.0, 1.0) * 0.8).clamp(0.0, 1.0)
}

fn render_display_list_to_svg(
    display_list: &DisplayList,
    font_size_px: f64,
    padding_em: f64,
    default_color: Rgba,
    outline_font_cache: Option<&OutlineFontCache>,
) -> String {
    let padding_px = padding_em * font_size_px;
    let width_px = ((display_list.width + (padding_em * 2.0)) * font_size_px).max(1.0);
    let height_px =
        ((display_list.height + display_list.depth + (padding_em * 2.0)) * font_size_px).max(1.0);
    let mut body = String::with_capacity(display_list.items.len() * 96);

    for item in &display_list.items {
        match item {
            DisplayItem::GlyphPath {
                x,
                y,
                scale,
                font,
                char_code,
                color,
                ..
            } => emit_glyph(
                &mut body,
                *x,
                *y,
                *scale,
                font,
                *char_code,
                color,
                font_size_px,
                padding_px,
                default_color,
                outline_font_cache,
            ),
            DisplayItem::Line {
                x,
                y,
                width,
                thickness,
                color,
                ..
            } => emit_line(
                &mut body,
                *x,
                *y,
                *width,
                *thickness,
                color,
                font_size_px,
                padding_px,
                default_color,
            ),
            DisplayItem::Rect {
                x,
                y,
                width,
                height,
                color,
            } => emit_rect(
                &mut body,
                *x,
                *y,
                *width,
                *height,
                color,
                font_size_px,
                padding_px,
                default_color,
            ),
            DisplayItem::Path {
                x,
                y,
                commands,
                fill,
                color,
            } => emit_path(
                &mut body,
                *x,
                *y,
                commands,
                *fill,
                color,
                font_size_px,
                padding_px,
                default_color,
            ),
        }
    }

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">{body}</svg>"#,
        width = fmt_num(width_px),
        height = fmt_num(height_px),
        body = body,
    )
}

#[allow(clippy::too_many_arguments)]
fn emit_glyph(
    output: &mut String,
    x_em: f64,
    y_em: f64,
    scale: f64,
    font: &str,
    char_code: u32,
    color: &MathColor,
    font_size_px: f64,
    padding_px: f64,
    default_color: Rgba,
    outline_font_cache: Option<&OutlineFontCache>,
) {
    let fill = svg_color(color, default_color);
    let x_px = ((x_em * font_size_px) + padding_px) as f32;
    let y_px = ((y_em * font_size_px) + padding_px) as f32;
    let glyph_em = (scale * font_size_px) as f32;

    if let Some(outline_font_cache) = outline_font_cache
        && let Some(path_data) =
            glyph_outline_path(x_px, y_px, glyph_em, font, char_code, outline_font_cache)
    {
        use std::fmt::Write as _;
        let _ = write!(
            output,
            r#"<path d="{path}" fill="{fill}" fill-rule="evenodd" stroke="none"/>"#,
            path = path_data,
            fill = fill,
        );
        return;
    }

    let character = char::from_u32(char_code).unwrap_or('\u{fffd}');
    let (family, weight, font_style) = font_face(font);
    use std::fmt::Write as _;
    let _ = write!(
        output,
        r#"<text x="{x}" y="{y}" font-family="{family}" font-size="{font_size}" font-weight="{weight}" font-style="{font_style}" fill="{fill}" dominant-baseline="alphabetic">{text}</text>"#,
        x = fmt_num(x_px as f64),
        y = fmt_num(y_px as f64),
        family = family,
        font_size = fmt_num(scale * font_size_px),
        weight = weight,
        font_style = font_style,
        fill = fill,
        text = escape_xml(&character.to_string()),
    );
}

fn outline_font_cache(cx: &App) -> Option<Arc<OutlineFontCache>> {
    let mut cache = OUTLINE_FONT_CACHE.lock().ok()?;
    if let Some(font_cache) = cache.as_ref() {
        return Some(font_cache.clone());
    }

    let font_cache = load_outline_font_cache(cx.asset_source().as_ref())?;
    let font_cache = Arc::new(font_cache);
    *cache = Some(font_cache.clone());
    Some(font_cache)
}

fn load_outline_font_cache(asset_source: &dyn AssetSource) -> Option<OutlineFontCache> {
    let mut font_cache = HashMap::new();

    for (font_name, asset_path) in KATEX_FONT_ASSETS {
        let bytes = match asset_source.load(asset_path) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => {
                log::warn!("missing math font asset: {asset_path}");
                continue;
            }
            Err(error) => {
                log::warn!("failed to load math font asset {asset_path}: {error:#}");
                continue;
            }
        };

        let parsed_font = match bytes {
            std::borrow::Cow::Borrowed(bytes) => FontArc::try_from_slice(bytes),
            std::borrow::Cow::Owned(bytes) => FontArc::try_from_vec(bytes),
        };

        match parsed_font {
            Ok(font) => {
                font_cache.insert(font_name, font);
            }
            Err(error) => {
                log::warn!("failed to parse math font asset {asset_path}: {error}");
            }
        }
    }

    if font_cache.is_empty() {
        None
    } else {
        Some(font_cache)
    }
}

fn glyph_outline_path(
    x_px: f32,
    y_px: f32,
    glyph_em: f32,
    font_name: &str,
    char_code: u32,
    font_cache: &OutlineFontCache,
) -> Option<String> {
    let character = char::from_u32(char_code).unwrap_or('?');
    let font = font_cache
        .get(font_name)
        .or_else(|| font_cache.get("Main-Regular"))?;
    let glyph_id = font.glyph_id(character);

    if glyph_id.0 != 0 {
        return outline_to_svg_path(x_px, y_px, glyph_em, font, glyph_id);
    }

    let fallback_font = font_cache.get("Main-Regular")?;
    let fallback_glyph_id = fallback_font.glyph_id(character);
    if fallback_glyph_id.0 == 0 {
        None
    } else {
        outline_to_svg_path(x_px, y_px, glyph_em, fallback_font, fallback_glyph_id)
    }
}

fn outline_to_svg_path(
    x_px: f32,
    y_px: f32,
    glyph_em: f32,
    font: &FontArc,
    glyph_id: ab_glyph::GlyphId,
) -> Option<String> {
    let outline = font.outline(glyph_id)?;
    let units_per_em = font.units_per_em().unwrap_or(1000.0);
    let scale = glyph_em / units_per_em;
    let mut path = String::new();
    let mut last_end: Option<(f32, f32)> = None;

    for curve in &outline.curves {
        let (start, end) = match curve {
            OutlineCurve::Line(p0, p1) => {
                let start = (x_px + p0.x * scale, y_px - p0.y * scale);
                let end = (x_px + p1.x * scale, y_px - p1.y * scale);
                (start, end)
            }
            OutlineCurve::Quad(p0, _, p2) => {
                let start = (x_px + p0.x * scale, y_px - p0.y * scale);
                let end = (x_px + p2.x * scale, y_px - p2.y * scale);
                (start, end)
            }
            OutlineCurve::Cubic(p0, _, _, p3) => {
                let start = (x_px + p0.x * scale, y_px - p0.y * scale);
                let end = (x_px + p3.x * scale, y_px - p3.y * scale);
                (start, end)
            }
        };

        let needs_move = match last_end {
            None => true,
            Some((last_x, last_y)) => {
                (last_x - start.0).abs() > 0.01 || (last_y - start.1).abs() > 0.01
            }
        };

        if needs_move {
            if last_end.is_some() {
                path.push('Z');
                path.push(' ');
            }
            use std::fmt::Write as _;
            let _ = write!(
                path,
                "M{} {} ",
                fmt_num(start.0 as f64),
                fmt_num(start.1 as f64)
            );
        }

        match curve {
            OutlineCurve::Line(_, p1) => {
                use std::fmt::Write as _;
                let _ = write!(
                    path,
                    "L{} {} ",
                    fmt_num((x_px + p1.x * scale) as f64),
                    fmt_num((y_px - p1.y * scale) as f64)
                );
            }
            OutlineCurve::Quad(_, p1, p2) => {
                use std::fmt::Write as _;
                let _ = write!(
                    path,
                    "Q{} {} {} {} ",
                    fmt_num((x_px + p1.x * scale) as f64),
                    fmt_num((y_px - p1.y * scale) as f64),
                    fmt_num((x_px + p2.x * scale) as f64),
                    fmt_num((y_px - p2.y * scale) as f64)
                );
            }
            OutlineCurve::Cubic(_, p1, p2, p3) => {
                use std::fmt::Write as _;
                let _ = write!(
                    path,
                    "C{} {} {} {} {} {} ",
                    fmt_num((x_px + p1.x * scale) as f64),
                    fmt_num((y_px - p1.y * scale) as f64),
                    fmt_num((x_px + p2.x * scale) as f64),
                    fmt_num((y_px - p2.y * scale) as f64),
                    fmt_num((x_px + p3.x * scale) as f64),
                    fmt_num((y_px - p3.y * scale) as f64)
                );
            }
        }

        last_end = Some(end);
    }

    if last_end.is_some() {
        path.push('Z');
    }

    let path = path.trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

fn emit_line(
    output: &mut String,
    x_em: f64,
    y_em: f64,
    width_em: f64,
    thickness_em: f64,
    color: &MathColor,
    font_size_px: f64,
    padding_px: f64,
    default_color: Rgba,
) {
    let thickness_px = (thickness_em * font_size_px).max(0.5);
    let x = (x_em * font_size_px) + padding_px;
    let y = ((y_em * font_size_px) + padding_px) - (thickness_px / 2.0);
    let width = width_em * font_size_px;
    let fill = svg_color(color, default_color);

    use std::fmt::Write as _;
    let _ = write!(
        output,
        r#"<rect x="{x}" y="{y}" width="{width}" height="{height}" fill="{fill}"/>"#,
        x = fmt_num(x),
        y = fmt_num(y),
        width = fmt_num(width),
        height = fmt_num(thickness_px),
        fill = fill,
    );
}

fn emit_rect(
    output: &mut String,
    x_em: f64,
    y_em: f64,
    width_em: f64,
    height_em: f64,
    color: &MathColor,
    font_size_px: f64,
    padding_px: f64,
    default_color: Rgba,
) {
    let fill = svg_color(color, default_color);

    use std::fmt::Write as _;
    let _ = write!(
        output,
        r#"<rect x="{x}" y="{y}" width="{width}" height="{height}" fill="{fill}"/>"#,
        x = fmt_num((x_em * font_size_px) + padding_px),
        y = fmt_num((y_em * font_size_px) + padding_px),
        width = fmt_num(width_em * font_size_px),
        height = fmt_num(height_em * font_size_px),
        fill = fill,
    );
}

fn emit_path(
    output: &mut String,
    x_em: f64,
    y_em: f64,
    commands: &[PathCommand],
    fill_shape: bool,
    color: &MathColor,
    font_size_px: f64,
    padding_px: f64,
    default_color: Rgba,
) {
    let color = svg_color(color, default_color);
    let path = path_commands_to_d(
        (x_em * font_size_px) + padding_px,
        (y_em * font_size_px) + padding_px,
        font_size_px,
        commands,
    );

    use std::fmt::Write as _;
    if fill_shape {
        let _ = write!(
            output,
            r#"<path d="{path}" fill="{color}" stroke="none"/>"#,
            path = path,
            color = color,
        );
    } else {
        let _ = write!(
            output,
            r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="{width}" stroke-linecap="round" stroke-linejoin="round"/>"#,
            path = path,
            color = color,
            width = fmt_num((font_size_px * 0.035).max(1.0)),
        );
    }
}

fn path_commands_to_d(
    origin_x: f64,
    origin_y: f64,
    em_px: f64,
    commands: &[PathCommand],
) -> String {
    let mut output = String::new();

    for command in commands {
        match command {
            PathCommand::MoveTo { x, y } => {
                output.push('M');
                output.push_str(&fmt_num(origin_x + (x * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_y + (y * em_px)));
            }
            PathCommand::LineTo { x, y } => {
                output.push('L');
                output.push_str(&fmt_num(origin_x + (x * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_y + (y * em_px)));
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                output.push('C');
                output.push_str(&fmt_num(origin_x + (x1 * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_y + (y1 * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_x + (x2 * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_y + (y2 * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_x + (x * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_y + (y * em_px)));
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                output.push('Q');
                output.push_str(&fmt_num(origin_x + (x1 * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_y + (y1 * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_x + (x * em_px)));
                output.push(' ');
                output.push_str(&fmt_num(origin_y + (y * em_px)));
            }
            PathCommand::Close => output.push('Z'),
        }
        output.push(' ');
    }

    output.trim_end().to_string()
}

const SANS_SERIF_FONT_STACK: &str = "KaTeX_SansSerif, Segoe UI, Inter, Arial, sans-serif";
const TYPEWRITER_FONT_STACK: &str =
    "KaTeX_Typewriter, Latin Modern Mono, CMU Typewriter Text, Consolas, Courier New, monospace";
const MATH_FONT_STACK: &str =
    "KaTeX_Math, Latin Modern Math, XITS Math, Cambria Math, STIX Two Math, Noto Serif Math, serif";
const MAIN_FONT_STACK: &str = "KaTeX_Main, Latin Modern Roman, CMU Serif, Computer Modern Serif, Cambria Math, STIX Two Text, Times New Roman, serif";
const AMS_FONT_STACK: &str = "KaTeX_AMS, KaTeX_Main, Latin Modern Math, XITS Math, Cambria Math, STIX Two Math, Noto Serif Math, serif";
const CALIGRAPHIC_FONT_STACK: &str = "KaTeX_Caligraphic, KaTeX_Main, Latin Modern Math, XITS Math, Cambria Math, STIX Two Math, Noto Serif Math, serif";
const FRAKTUR_FONT_STACK: &str = "KaTeX_Fraktur, KaTeX_Main, Latin Modern Math, XITS Math, Cambria Math, STIX Two Math, Noto Serif Math, serif";
const SCRIPT_FONT_STACK: &str = "KaTeX_Script, KaTeX_Main, Latin Modern Math, XITS Math, Cambria Math, STIX Two Math, Noto Serif Math, serif";
const SIZE1_FONT_STACK: &str = "KaTeX_Size1, KaTeX_Main, Latin Modern Math, XITS Math, Cambria Math, STIX Two Math, Noto Serif Math, serif";
const SIZE2_FONT_STACK: &str = "KaTeX_Size2, KaTeX_Main, Latin Modern Math, XITS Math, Cambria Math, STIX Two Math, Noto Serif Math, serif";
const SIZE3_FONT_STACK: &str = "KaTeX_Size3, KaTeX_Main, Latin Modern Math, XITS Math, Cambria Math, STIX Two Math, Noto Serif Math, serif";
const SIZE4_FONT_STACK: &str = "KaTeX_Size4, KaTeX_Main, Latin Modern Math, XITS Math, Cambria Math, STIX Two Math, Noto Serif Math, serif";

fn font_face(font: &str) -> (&'static str, &'static str, &'static str) {
    match font {
        "Main-Regular" => (MAIN_FONT_STACK, "normal", "normal"),
        "SansSerif-Regular" => (SANS_SERIF_FONT_STACK, "normal", "normal"),
        "SansSerif-Bold" => (SANS_SERIF_FONT_STACK, "bold", "normal"),
        "SansSerif-Italic" => (SANS_SERIF_FONT_STACK, "normal", "italic"),
        "Typewriter-Regular" => (TYPEWRITER_FONT_STACK, "normal", "normal"),
        "Math-BoldItalic" => (MATH_FONT_STACK, "bold", "italic"),
        "Math-Italic" => (MATH_FONT_STACK, "normal", "italic"),
        "AMS-Regular" => (AMS_FONT_STACK, "normal", "normal"),
        "Caligraphic-Regular" => (CALIGRAPHIC_FONT_STACK, "normal", "normal"),
        "Fraktur-Regular" => (FRAKTUR_FONT_STACK, "normal", "normal"),
        "Main-Bold" => (MAIN_FONT_STACK, "bold", "normal"),
        "Main-BoldItalic" => (MAIN_FONT_STACK, "bold", "italic"),
        "Main-Italic" => (MAIN_FONT_STACK, "normal", "italic"),
        "Script-Regular" => (SCRIPT_FONT_STACK, "normal", "normal"),
        "Size1-Regular" => (SIZE1_FONT_STACK, "normal", "normal"),
        "Size2-Regular" => (SIZE2_FONT_STACK, "normal", "normal"),
        "Size3-Regular" => (SIZE3_FONT_STACK, "normal", "normal"),
        "Size4-Regular" => (SIZE4_FONT_STACK, "normal", "normal"),
        _ => (MAIN_FONT_STACK, "normal", "normal"),
    }
}

fn svg_color(color: &MathColor, default_color: Rgba) -> String {
    let rgba = if *color == MathColor::BLACK {
        default_color
    } else {
        Rgba {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    };

    format!(
        "rgba({},{},{},{})",
        (rgba.r * 255.0).round() as u8,
        (rgba.g * 255.0).round() as u8,
        (rgba.b * 255.0).round() as u8,
        fmt_num(rgba.a as f64),
    )
}

fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn fmt_num(number: f64) -> String {
    let formatted = format!("{number:.4}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assets::Assets;

    #[test]
    fn test_extract_math_formulas_collects_supported_entries() {
        let formulas = extract_math_formulas(&[
            (
                0..5,
                MarkdownEvent::Math {
                    display_mode: false,
                    content: "x^2".into(),
                },
            ),
            (
                6..15,
                MarkdownEvent::Math {
                    display_mode: true,
                    content: "y = x^2".into(),
                },
            ),
        ]);

        assert_eq!(formulas.len(), 2);
        assert!(!formulas.get(&0).unwrap().display_mode);
        assert!(formulas.get(&6).unwrap().display_mode);
    }

    #[test]
    fn test_render_display_list_to_svg_outputs_svg_document() {
        let formulas = extract_math_formulas(&[(
            0..13,
            MarkdownEvent::Math {
                display_mode: false,
                content: "\\frac{a}{b}".into(),
            },
        )]);
        let formula = formulas.get(&0).unwrap();
        let svg = render_display_list_to_svg(
            &formula.display_list,
            16.0,
            0.16,
            gpui::Hsla::black().to_rgb(),
            None,
        );

        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("font-family=") || svg.contains("<path "));
    }

    #[test]
    fn test_render_display_list_to_svg_prefers_glyph_outlines_when_fonts_are_available() {
        let formulas = extract_math_formulas(&[(
            0..24,
            MarkdownEvent::Math {
                display_mode: false,
                content: "\\sqrt{\\pi} \\times 2".into(),
            },
        )]);
        let formula = formulas.get(&0).unwrap();
        let font_cache = load_outline_font_cache(&Assets).unwrap();
        let svg = render_display_list_to_svg(
            &formula.display_list,
            16.0,
            0.16,
            gpui::Hsla::black().to_rgb(),
            Some(&font_cache),
        );

        assert!(svg.contains("<path "));
        assert!(!svg.contains("<text "));
    }

    #[test]
    fn test_font_face_prefers_katex_families() {
        let (math_family, math_weight, math_style) = font_face("Math-Italic");
        assert!(math_family.starts_with("KaTeX_Math,"));
        assert_eq!(math_weight, "normal");
        assert_eq!(math_style, "italic");

        let (ams_family, ams_weight, ams_style) = font_face("AMS-Regular");
        assert!(ams_family.starts_with("KaTeX_AMS,"));
        assert_eq!(ams_weight, "normal");
        assert_eq!(ams_style, "normal");

        let (main_family, main_weight, main_style) = font_face("Main-BoldItalic");
        assert!(main_family.starts_with("KaTeX_Main,"));
        assert_eq!(main_weight, "bold");
        assert_eq!(main_style, "italic");

        let (caligraphic_family, _, _) = font_face("Caligraphic-Regular");
        assert!(caligraphic_family.starts_with("KaTeX_Caligraphic,"));

        let (fraktur_family, _, _) = font_face("Fraktur-Regular");
        assert!(fraktur_family.starts_with("KaTeX_Fraktur,"));

        let (sans_family, _, _) = font_face("SansSerif-Regular");
        assert!(sans_family.starts_with("KaTeX_SansSerif,"));

        let (script_family, _, _) = font_face("Script-Regular");
        assert!(script_family.starts_with("KaTeX_Script,"));

        let (size1_family, _, _) = font_face("Size1-Regular");
        assert!(size1_family.starts_with("KaTeX_Size1,"));

        let (size4_family, _, _) = font_face("Size4-Regular");
        assert!(size4_family.starts_with("KaTeX_Size4,"));

        let (typewriter_family, _, _) = font_face("Typewriter-Regular");
        assert!(typewriter_family.starts_with("KaTeX_Typewriter,"));
    }

    #[test]
    fn test_inline_math_baseline_adjustment_uses_x_height_delta() {
        let font_size_px = 16.0;
        let math_x_height_px = LayoutOptions::default()
            .with_style(MathStyle::Text)
            .metrics()
            .x_height
            * font_size_px;
        let adjustment =
            inline_math_baseline_adjustment_for_x_height(math_x_height_px + 1.5, font_size_px);

        assert!((adjustment - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_inline_math_baseline_adjustment_clamps_to_zero() {
        let font_size_px = 16.0;
        let math_x_height_px = LayoutOptions::default()
            .with_style(MathStyle::Text)
            .metrics()
            .x_height
            * font_size_px;
        let adjustment =
            inline_math_baseline_adjustment_for_x_height(math_x_height_px - 2.0, font_size_px);

        assert_eq!(adjustment, 0.0);
    }

    #[test]
    fn test_inline_math_baseline_adjustment_caps_large_deltas() {
        let font_size_px = 16.0;
        let math_x_height_px = LayoutOptions::default()
            .with_style(MathStyle::Text)
            .metrics()
            .x_height
            * font_size_px;
        let adjustment =
            inline_math_baseline_adjustment_for_x_height(math_x_height_px + 8.0, font_size_px);

        assert_eq!(adjustment, font_size_px * 0.12);
    }

    #[test]
    fn test_inline_math_tallness_bias_factor_keeps_small_math_low() {
        assert!((inline_math_tallness_bias_factor(0.55) - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_inline_math_tallness_bias_factor_reaches_full_bias_for_tall_math() {
        assert_eq!(inline_math_tallness_bias_factor(1.05), 1.0);
    }
}
