use dioxus::prelude::*;
use gtk_platform::{launch, Text, View, Window};
use snafu::Whatever;

fn app(cx: Scope) -> Element {
    cx.render(rsx!(Window {
        title: "Hello World",
        View {
            Text {
                label: "Hello World!"
            }
        }
    }))
}
pub fn main() -> Result<(), Whatever> {
    launch(app, "org.dioxus-gtk.hello_world")?;
    Ok(())
}
