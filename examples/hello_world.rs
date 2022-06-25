use dioxus::prelude::*;
use gtk_platform::{launch, AlignItems, JustifyContent, LayoutProps, Text, View, Window};
use snafu::Whatever;

fn app(cx: Scope) -> Element {
    cx.render(rsx!(Window {
        title: "Hello World",
        layout: LayoutProps {
            justify_content: Some(JustifyContent::Center),
            ..Default::default()
        }
        View {
            layout: LayoutProps {
                justify_content: Some(JustifyContent::Center),
                align_items: Some(AlignItems::Center),
                ..Default::default()
            }
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
