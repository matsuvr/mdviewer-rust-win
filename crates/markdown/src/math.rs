use std::{collections::BTreeMap, ops::Range, sync::Arc};

use gpui::{
    AnyElement, App, Image, ImageFormat, ImageSource, Pixels, RenderImage, Rgba, Window, img, px,
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
    let font_size_px = style
        .base_text_style
        .font_size
        .to_pixels(window.rem_size())
        .to_f64();
    let font_size_px = if math.display_mode {
        font_size_px * 1.12
    } else {
        font_size_px
    };
    let padding_em = if math.display_mode { 0.30 } else { 0.16 };
    let default_color = style.base_text_style.color.to_rgb();
    let svg =
        render_display_list_to_svg(&math.display_list, font_size_px, padding_em, default_color);
    let width_px = ((math.display_list.width + (padding_em * 2.0)) * font_size_px).max(1.0);
    let height_px = ((math.display_list.height + math.display_list.depth + (padding_em * 2.0))
        * font_size_px)
        .max(1.0);
    let baseline_from_top_px = ((math.display_list.height + padding_em) * font_size_px).max(0.0);

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

fn render_display_list_to_svg(
    display_list: &DisplayList,
    font_size_px: f64,
    padding_em: f64,
    default_color: Rgba,
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
) {
    let character = char::from_u32(char_code).unwrap_or('\u{fffd}');
    let (family, weight, font_style) = font_face(font);
    let fill = svg_color(color, default_color);

    use std::fmt::Write as _;
    let _ = write!(
        output,
        r#"<text x="{x}" y="{y}" font-family="{family}" font-size="{font_size}" font-weight="{weight}" font-style="{font_style}" fill="{fill}" dominant-baseline="alphabetic">{text}</text>"#,
        x = fmt_num((x_em * font_size_px) + padding_px),
        y = fmt_num((y_em * font_size_px) + padding_px),
        family = family,
        font_size = fmt_num(scale * font_size_px),
        weight = weight,
        font_style = font_style,
        fill = fill,
        text = escape_xml(&character.to_string()),
    );
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

fn font_face(font: &str) -> (&'static str, &'static str, &'static str) {
    match font {
        "SansSerif-Regular" => ("Segoe UI, Inter, Arial, sans-serif", "normal", "normal"),
        "SansSerif-Bold" => ("Segoe UI, Inter, Arial, sans-serif", "bold", "normal"),
        "SansSerif-Italic" => ("Segoe UI, Inter, Arial, sans-serif", "normal", "italic"),
        "Typewriter-Regular" => ("Consolas, Courier New, monospace", "normal", "normal"),
        "Math-BoldItalic" => (
            "Cambria Math, STIX Two Math, Latin Modern Math, Noto Serif Math, serif",
            "bold",
            "italic",
        ),
        "Math-Italic" => (
            "Cambria Math, STIX Two Math, Latin Modern Math, Noto Serif Math, serif",
            "normal",
            "italic",
        ),
        "Main-Bold" => (
            "Cambria Math, STIX Two Text, Latin Modern Roman, Times New Roman, serif",
            "bold",
            "normal",
        ),
        "Main-BoldItalic" => (
            "Cambria Math, STIX Two Text, Latin Modern Roman, Times New Roman, serif",
            "bold",
            "italic",
        ),
        "Main-Italic" => (
            "Cambria Math, STIX Two Text, Latin Modern Roman, Times New Roman, serif",
            "normal",
            "italic",
        ),
        _ => (
            "Cambria Math, STIX Two Math, Latin Modern Math, Noto Serif Math, Times New Roman, serif",
            "normal",
            "normal",
        ),
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
        );

        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("font-family=") || svg.contains("<path "));
    }
}
