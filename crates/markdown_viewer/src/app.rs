use gpui::{App, AppContext, KeyBinding, WindowOptions};
use std::{env, path::PathBuf};
use theme::{GlobalTheme, LoadThemes, ThemeRegistry};

use crate::viewer::MarkdownViewer;

pub const APP_TITLE: &str = "Markdown Viewer";
const LIGHT_THEME_NAME: &str = "One Light";

#[derive(Debug)]
pub struct Args {
    pub paths: Vec<PathBuf>,
}

impl Args {
    pub fn parse() -> Self {
        Self {
            paths: env::args_os().skip(1).map(PathBuf::from).collect(),
        }
    }
}

pub fn init(cx: &mut App) {
    settings::init(cx);
    theme_settings::init(LoadThemes::All(Box::new(assets::Assets)), cx);
    assets::Assets
        .load_fonts(cx)
        .expect("failed to load bundled fonts");
    bind_viewer_key_bindings(cx);
    force_light_theme(cx);
}

pub fn open_initial_windows(paths: Vec<PathBuf>, cx: &mut App) {
    cx.activate(true);

    if paths.is_empty() {
        open_window(None, cx);
        return;
    }

    for path in paths {
        open_window(Some(path), cx);
    }
}

fn open_window(path: Option<PathBuf>, cx: &mut App) {
    cx.open_window(WindowOptions::default(), move |window, cx| {
        cx.new(|cx| MarkdownViewer::new(path.clone(), window, cx))
    })
    .expect("failed to open markdown viewer window");
}

fn bind_viewer_key_bindings(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("secondary-c", markdown::Copy, None)]);
}

fn force_light_theme(cx: &mut App) {
    let registry = ThemeRegistry::global(cx);
    let theme = registry
        .get(LIGHT_THEME_NAME)
        .expect("One Light theme should be available in bundled assets");
    let icon_theme = registry
        .default_icon_theme()
        .expect("default icon theme should be available");

    GlobalTheme::update_theme(cx, theme);
    GlobalTheme::update_icon_theme(cx, icon_theme);
}

#[cfg(test)]
mod tests {
    use gpui::{Keystroke, TestAppContext};

    use super::bind_viewer_key_bindings;

    #[gpui::test]
    fn binds_ctrl_c_to_markdown_copy_action(cx: &mut TestAppContext) {
        cx.update(bind_viewer_key_bindings);

        let ctrl_c_bindings = cx.read(|app: &gpui::App| {
            app.all_bindings_for_input(&[Keystroke::parse("ctrl-c").unwrap()])
        });
        assert_eq!(ctrl_c_bindings.len(), 1);
        assert_eq!(ctrl_c_bindings[0].action().name(), "markdown::Copy");
        assert_eq!(
            ctrl_c_bindings[0].keystrokes()[0].inner().unparse(),
            "ctrl-c"
        );

        let windows_key_c_bindings = cx.read(|app: &gpui::App| {
            app.all_bindings_for_input(&[Keystroke::parse("cmd-c").unwrap()])
        });
        assert!(windows_key_c_bindings.is_empty());
    }
}
