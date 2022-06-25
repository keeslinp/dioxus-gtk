use dioxus::prelude::*;
use gtk_platform::{
    geometry::Size,
    launch,
    style::{AlignItems, Dimension, JustifyContent, Style},
    Button, Text, View, Window,
};
use snafu::Whatever;

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);
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
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            }
            Button {
                label: "-".to_owned()
                on_press: move |_| count.modify(|c| c - 1)
            }
            Text {
                label: format!("Pressed {} times", count)
            }
            Button {
                label: "+".to_owned()
                on_press: move |_| count.modify(|c| c + 1)
            }
        }
    }))
}
pub fn main() -> Result<(), Whatever> {
    launch(app, "org.dioxus-gtk.hello_world")?;
    Ok(())
}