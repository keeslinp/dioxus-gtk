use dioxus::prelude::*;
use gtk_platform::{
    components::{Button, Text, TextField, View, Window},
    events::TextChangeEvent,
    geometry::{Rect, Size},
    launch,
    style::{AlignItems, Dimension, FlexDirection, JustifyContent, Style},
};
use snafu::Whatever;

fn app(cx: Scope) -> Element {
    let text = use_state(&cx, || "".to_owned());
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
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            }
            View {
                TextField {
                    place_holder: "Input".to_owned(),
                    value: text.current().as_ref().clone(),
                    on_text_change: move |evt: TextChangeEvent| text.modify(|_| evt.data.value.clone()),
                    layout: Style {
                        margin: Rect {
                            end: Dimension::Points(10.),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                }
                Button {
                    label: "Clear".to_owned(),
                    on_press: move |_| text.modify(|_| "".to_owned())
                }
            }
            Text {
                layout: Style {
                    margin: Rect {
                        top: Dimension::Points(10.),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                label: format!("Entered Text: {}", text.current())
            }
        }
    }))
}
pub fn main() -> Result<(), Whatever> {
    launch(app, "org.dioxus-gtk.hello_world")?;
    Ok(())
}
