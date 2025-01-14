#![allow(static_mut_refs)]

mod global;
mod keys;
mod ui;

use global::Global;
use ui::UI;

fn main() {
    let hook = Global::install_hook();

    gpui::App::new().run(|cx| {
        cx.open_window(gpui::WindowOptions::default(), |cx| UI::new(cx))
            .unwrap();
        cx.activate(true);
    });

    Global::delete_hook(hook);
}
