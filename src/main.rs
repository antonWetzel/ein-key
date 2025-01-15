#![allow(static_mut_refs)]

mod assets;
mod global;
mod keys;
mod theme;
mod title_bar;
mod ui;
mod vk_table;

use assets::BundledAssets;
use global::Global;
use gpui::*;
use ui::UI;

fn main() {
    let hook = Global::install_hook();

    App::new().with_assets(BundledAssets).run(|cx| {
        let mut options = WindowOptions::default();
        options.show = true;
        options.titlebar = Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        });
        options.focus = true;
        cx.open_window(options, |cx| UI::new(cx)).unwrap();
    });

    Global::delete_hook(hook);
}
