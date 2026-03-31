#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
compile_error!("markdown_viewer supports only Windows x64.");

mod app;
mod viewer;

fn main() {
    let args = app::Args::parse();
    let paths = args.paths;

    gpui_platform::application()
        .with_assets(assets::Assets)
        .run(move |cx| {
            app::init(cx);
            app::open_initial_windows(paths, cx);
        });
}
