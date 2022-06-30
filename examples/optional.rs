use dioxus::prelude::*;
use gtk_platform::{
    components::{Button, Text, View, Window},
    geometry::{Rect, Size},
    launch,
    style::{AlignItems, Dimension, JustifyContent, Style},
};
use snafu::Whatever;

fn app(cx: Scope) -> Element {
    let visible = use_state(&cx, || false);
    cx.render(rsx!(Window {
        title: "Show / Hide",
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
                label: if *visible.current() { "Press to hide".to_owned() } else { "Press to show".to_owned() },
                on_press: move |_| visible.modify(|c| !c),
            }
            visible.then(|| rsx!{
                Text {
                    layout: Style {
                        margin: Rect {
                            start: Dimension::Points(10.),
                            end: Dimension::Points(10.),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    label: "Hidden message".to_owned(),
                }
            })
        }
    }))
}
pub fn main() -> Result<(), Whatever> {
    launch(app, "org.dioxus-gtk.hello_world")?;
    Ok(())
}
