use gpui::{AppContext, Context, IntoElement, Render, TestAppContext, Window, div};
use markdown::{Markdown, MarkdownElement, MarkdownOptions, MarkdownStyle, parser::MarkdownEvent};

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

fn render_markdown_text(
    markdown_source: &str,
    options: MarkdownOptions,
    cx: &mut TestAppContext,
) -> String {
    struct TestWindow;

    impl Render for TestWindow {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    ensure_theme_initialized(cx);

    let (_, mut cx) = cx.add_window_view(|_, _| TestWindow);
    let markdown = cx.new(|cx| {
        Markdown::new_with_options(markdown_source.to_string().into(), None, None, options, cx)
    });
    cx.run_until_parked();
    MarkdownElement::rendered_text(markdown, &mut cx, |_window, _app| MarkdownStyle::default())
}

#[gpui::test]
fn preserves_inline_and_display_math_events(cx: &mut TestAppContext) {
    let inline = cx.new(|cx| {
        Markdown::new_with_options(
            "inline $x^2$ math".into(),
            None,
            None,
            MarkdownOptions {
                render_math: true,
                ..Default::default()
            },
            cx,
        )
    });
    cx.run_until_parked();
    inline.read_with(cx, |markdown, _| {
        assert!(
            markdown
                .parsed_markdown()
                .events()
                .iter()
                .any(|(_, event)| {
                    matches!(
                        event,
                        MarkdownEvent::Math {
                            display_mode: false,
                            content,
                        } if content == "x^2"
                    )
                })
        );
    });

    let display = cx.new(|cx| {
        Markdown::new_with_options(
            "$$y = x^2$$".into(),
            None,
            None,
            MarkdownOptions {
                render_math: true,
                ..Default::default()
            },
            cx,
        )
    });
    cx.run_until_parked();
    display.read_with(cx, |markdown, _| {
        assert!(
            markdown
                .parsed_markdown()
                .events()
                .iter()
                .any(|(_, event)| {
                    matches!(
                        event,
                        MarkdownEvent::Math {
                            display_mode: true,
                            content,
                        } if content == "y = x^2"
                    )
                })
        );
    });
}

#[gpui::test]
fn falls_back_to_source_text_when_math_rendering_is_disabled(cx: &mut TestAppContext) {
    let rendered = render_markdown_text("inline $x^2$ math", MarkdownOptions::default(), cx);
    assert_eq!(rendered, "inline $x^2$ math");
}

#[gpui::test]
fn render_math_enabled_hides_raw_inline_delimiters(cx: &mut TestAppContext) {
    let rendered = render_markdown_text(
        "inline $x^2$ math",
        MarkdownOptions {
            render_math: true,
            ..Default::default()
        },
        cx,
    );
    assert!(!rendered.contains("$x^2$"));
}
