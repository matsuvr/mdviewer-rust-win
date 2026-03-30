use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context as _, Result, anyhow};
use gpui::{
    App, Context, Entity, ImageSource, Render, Resource, ScrollHandle, SharedString, SharedUri,
    StatefulInteractiveElement, StyleRefinement, TextStyleRefinement, UnderlineStyle, Window,
    WindowBackgroundAppearance,
};
use markdown::{
    CodeBlockRenderer, HeadingLevelStyles, Markdown, MarkdownElement, MarkdownOptions,
    MarkdownStyle,
};
use ui::{WithScrollbar, div, prelude::*};

use crate::app::APP_TITLE;

const EMPTY_STATE: &str = r#"# Markdown Viewer

Open a Markdown file through the Windows file association, or pass a path on the command line.
"#;

pub struct MarkdownViewer {
    markdown: Entity<Markdown>,
    scroll_handle: ScrollHandle,
    current_path: Option<PathBuf>,
    status_message: Option<String>,
}

impl MarkdownViewer {
    pub fn new(path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_background_appearance(WindowBackgroundAppearance::Opaque);

        let markdown = cx.new(|cx| {
            Markdown::new_with_options(
                SharedString::default(),
                None,
                None,
                MarkdownOptions {
                    parse_html: true,
                    render_math: true,
                    render_mermaid_diagrams: true,
                    ..Default::default()
                },
                cx,
            )
        });

        let mut this = Self {
            markdown,
            scroll_handle: ScrollHandle::new(),
            current_path: None,
            status_message: None,
        };

        match path {
            Some(path) => this.open_path(path, window, cx),
            None => this.show_empty_state(window, cx),
        }

        this
    }

    fn show_empty_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.current_path = None;
        self.status_message = None;
        window.set_window_title(APP_TITLE);
        self.scroll_handle.scroll_to_item(0);
        self.markdown
            .update(cx, |markdown, cx| markdown.reset(EMPTY_STATE.into(), cx));
    }

    fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        match read_markdown_source(&path) {
            Ok(source) => {
                self.current_path = Some(path.clone());
                self.status_message = None;
                window.set_window_title(&window_title_for(&path));
                self.scroll_handle.scroll_to_item(0);
                self.markdown
                    .update(cx, |markdown, cx| markdown.reset(source, cx));
            }
            Err(error) => {
                self.current_path = Some(path.clone());
                self.status_message = Some(format!("{error:#}"));
                window.set_window_title(&window_title_for(&path));
                self.scroll_handle.scroll_to_item(0);
                self.markdown.update(cx, |markdown, cx| {
                    markdown.reset(
                        format!("# Failed to open file\n\n`{}`\n\n{}", path.display(), error)
                            .into(),
                        cx,
                    )
                });
            }
        }

        cx.notify();
    }

    fn handle_url_click(
        &mut self,
        destination: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let destination = destination.as_ref();

        if destination.starts_with('#') {
            self.status_message = Some("In-document anchors are not supported yet.".to_string());
            cx.notify();
            return;
        }

        if let Some(path) = resolve_viewer_path(destination, self.base_directory()) {
            self.open_path(path, window, cx);
            return;
        }

        if is_external_reference(destination) {
            cx.open_url(destination);
            return;
        }

        self.status_message = Some(format!("Linked file not found: {destination}"));
        cx.notify();
    }

    fn render_markdown(&self, window: &mut Window, cx: &mut Context<Self>) -> MarkdownElement {
        let base_directory = self.current_path.as_ref().and_then(|path| {
            path.parent()
                .map(|parent| parent.to_path_buf())
                .filter(|parent| !parent.as_os_str().is_empty())
        });
        let view = cx.entity().downgrade();

        MarkdownElement::new(self.markdown.clone(), viewer_markdown_style(window, cx))
            .code_block_renderer(CodeBlockRenderer::Default {
                copy_button: false,
                copy_button_on_hover: false,
                border: false,
            })
            .scroll_handle(self.scroll_handle.clone())
            .image_resolver({
                let base_directory = base_directory.clone();
                move |dest_url| resolve_viewer_image(dest_url, base_directory.as_deref())
            })
            .on_url_click(move |url, window, cx| {
                if let Some(view) = view.upgrade() {
                    let _ = cx.update_entity(&view, |this, cx| {
                        this.handle_url_click(url, window, cx);
                    });
                }
            })
    }

    fn base_directory(&self) -> Option<&Path> {
        self.current_path.as_ref().and_then(|path| path.parent())
    }
}

impl Render for MarkdownViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_message = self.status_message.clone();

        div()
            .id("markdown-viewer")
            .size_full()
            .bg(cx.theme().colors().background)
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .when_some(status_message, |this, message| {
                        this.child(
                            div()
                                .w_full()
                                .px_4()
                                .py_2()
                                .bg(cx.theme().colors().status_bar_background)
                                .border_b_1()
                                .border_color(cx.theme().colors().border)
                                .text_color(gpui::red())
                                .child(message),
                        )
                    })
                    .child(
                        div()
                            .id("markdown-viewer-scroll-container")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .p_4()
                            .child(self.render_markdown(window, cx)),
                    ),
            )
            .vertical_scrollbar_for(&self.scroll_handle, window, cx)
    }
}

fn read_markdown_source(path: &Path) -> Result<SharedString> {
    if !path.exists() {
        return Err(anyhow!("file does not exist: {}", path.display()));
    }
    if !path.is_file() {
        return Err(anyhow!("path is not a file: {}", path.display()));
    }

    let bytes = fs::read(path)
        .with_context(|| format!("failed to read Markdown file {}", path.display()))?;
    let text = String::from_utf8(bytes)
        .with_context(|| format!("Markdown file is not valid UTF-8: {}", path.display()))?;

    Ok(text.into())
}

fn viewer_markdown_style(window: &Window, cx: &App) -> MarkdownStyle {
    let mut base_text_style = window.text_style();
    base_text_style.color = cx.theme().colors().text;

    let font_family = base_text_style.font_family.clone();
    let font_fallbacks = base_text_style.font_fallbacks.clone();
    let font_features = base_text_style.font_features.clone();
    let colors = cx.theme().colors();
    let mut code_block = StyleRefinement::default()
        .bg(colors.editor_background)
        .border_1()
        .border_color(colors.border_variant)
        .rounded_md()
        .p_3()
        .mb_3();
    code_block.text = TextStyleRefinement {
        font_family: Some(font_family.clone()),
        font_fallbacks: font_fallbacks.clone(),
        font_features: Some(font_features.clone()),
        color: Some(colors.text),
        ..Default::default()
    };

    MarkdownStyle {
        base_text_style,
        code_block_overflow_x_scroll: true,
        syntax: cx.theme().syntax().clone(),
        selection_background_color: colors.element_selection_background,
        heading_level_styles: Some(HeadingLevelStyles {
            h1: Some(TextStyleRefinement {
                font_size: Some(rems(1.4).into()),
                ..Default::default()
            }),
            h2: Some(TextStyleRefinement {
                font_size: Some(rems(1.25).into()),
                ..Default::default()
            }),
            h3: Some(TextStyleRefinement {
                font_size: Some(rems(1.15).into()),
                ..Default::default()
            }),
            h4: Some(TextStyleRefinement {
                font_size: Some(rems(1.05).into()),
                ..Default::default()
            }),
            h5: Some(TextStyleRefinement {
                font_size: Some(rems(1.0).into()),
                ..Default::default()
            }),
            h6: Some(TextStyleRefinement {
                font_size: Some(rems(0.95).into()),
                ..Default::default()
            }),
        }),
        code_block,
        inline_code: TextStyleRefinement {
            font_family: Some(font_family.clone()),
            font_fallbacks: font_fallbacks.clone(),
            font_features: Some(font_features.clone()),
            background_color: Some(colors.editor_foreground.opacity(0.06)),
            color: Some(colors.text),
            ..Default::default()
        },
        block_quote: TextStyleRefinement {
            color: Some(colors.text_muted),
            ..Default::default()
        },
        link: TextStyleRefinement {
            color: Some(colors.text_accent),
            underline: Some(UnderlineStyle {
                color: Some(colors.text_accent.opacity(0.5)),
                thickness: px(1.),
                ..Default::default()
            }),
            ..Default::default()
        },
        rule_color: colors.border_variant,
        block_quote_border_color: colors.border,
        ..Default::default()
    }
}

fn window_title_for(path: &Path) -> String {
    let file_name = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    format!("{file_name} - {APP_TITLE}")
}

fn resolve_viewer_path(url: &str, base_directory: Option<&Path>) -> Option<PathBuf> {
    if is_external_reference(url) || url.starts_with('#') {
        return None;
    }

    let decoded_url = strip_fragment_and_query(url);
    if decoded_url.is_empty() {
        return None;
    }

    let decoded_url = urlencoding::decode(decoded_url)
        .map(|decoded| decoded.into_owned())
        .unwrap_or_else(|_| decoded_url.to_string());
    let candidate = PathBuf::from(&decoded_url);

    if candidate.is_absolute() && candidate.exists() {
        return Some(candidate);
    }

    let base_directory = base_directory?;
    let resolved = base_directory.join(decoded_url);
    resolved.exists().then_some(resolved)
}

fn resolve_viewer_image(dest_url: &str, base_directory: Option<&Path>) -> Option<ImageSource> {
    if dest_url.starts_with("data:") {
        return None;
    }

    if is_remote_http_resource(dest_url) {
        return Some(ImageSource::Resource(Resource::Uri(SharedUri::from(
            dest_url.to_string(),
        ))));
    }

    let decoded = strip_fragment_and_query(dest_url);
    let decoded = urlencoding::decode(decoded)
        .map(|decoded| decoded.into_owned())
        .unwrap_or_else(|_| decoded.to_string());

    let path = if Path::new(&decoded).is_absolute() {
        PathBuf::from(decoded)
    } else {
        base_directory?.join(decoded)
    };

    Some(ImageSource::Resource(Resource::Path(Arc::from(
        path.as_path(),
    ))))
}

fn strip_fragment_and_query(target: &str) -> &str {
    let target = target.split_once('#').map_or(target, |(path, _)| path);
    target.split_once('?').map_or(target, |(path, _)| path)
}

fn is_remote_http_resource(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

fn is_external_reference(target: &str) -> bool {
    if is_remote_http_resource(target) || target.starts_with("mailto:") {
        return true;
    }

    let Some(colon_index) = target.find(':') else {
        return false;
    };

    if colon_index == 1
        && target.as_bytes()[0].is_ascii_alphabetic()
        && target
            .as_bytes()
            .get(2)
            .is_some_and(|separator| *separator == b'\\' || *separator == b'/')
    {
        return false;
    }

    target[..colon_index]
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Result;
    use tempfile::TempDir;

    use super::{is_external_reference, resolve_viewer_path};

    #[test]
    fn resolves_relative_preview_paths() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_directory = temp_dir.path();
        let file = base_directory.join("notes.md");
        fs::write(&file, "# Notes")?;

        assert_eq!(
            resolve_viewer_path("notes.md", Some(base_directory)),
            Some(file)
        );
        assert_eq!(resolve_viewer_path("notes.md", None), None);

        Ok(())
    }

    #[test]
    fn resolves_urlencoded_preview_paths() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_directory = temp_dir.path();
        let file = base_directory.join("release notes.md");
        fs::write(&file, "# Release Notes")?;

        assert_eq!(
            resolve_viewer_path("release%20notes.md", Some(base_directory)),
            Some(file)
        );

        Ok(())
    }

    #[test]
    fn strips_fragments_from_local_paths() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_directory = temp_dir.path();
        let file = base_directory.join("guide.md");
        fs::write(&file, "# Guide")?;

        assert_eq!(
            resolve_viewer_path("guide.md#intro", Some(base_directory)),
            Some(file)
        );

        Ok(())
    }

    #[test]
    fn treats_windows_drive_paths_as_local() {
        assert!(!is_external_reference(r"C:\docs\notes.md"));
    }

    #[test]
    fn treats_http_and_mailto_links_as_external() {
        assert!(is_external_reference("https://zed.dev"));
        assert!(is_external_reference("mailto:test@example.com"));
    }
}
