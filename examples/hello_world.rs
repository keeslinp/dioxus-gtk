use dioxus::prelude::*;
use gtk_platform::{
    components::{Text, View, Window},
    geometry::Size,
    launch,
    style::{AlignItems, Dimension, JustifyContent, Style},
};
use snafu::Whatever;

fn app(cx: Scope) -> Element {
    cx.render(rsx!(Window {
        title: "Hello World",
        layout: Style {
            size: Size {
                width: Dimension::Percent(1.),
                height: Dimension::Percent(1.),
            },
            ..Default::default()
        }
        View {
            layout: Style {
                size: Size {
                    width: Dimension::Percent(1.),
                    height: Dimension::Percent(1.),
                },
                justify_content: JustifyContent::SpaceAround,
                align_items: AlignItems::Center,
                ..Default::default()
            }
            Text {
                label: "Hello"
            }
            Text {
                label: " World!"
            }
        }
    }))
}
pub fn main() -> Result<(), Whatever> {
    launch(app, "org.dioxus-gtk.hello_world")?;
    Ok(())
}
