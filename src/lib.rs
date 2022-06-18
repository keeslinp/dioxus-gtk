use bumpalo::boxed::Box as BumpBox;
use bumpalo::collections::Vec as BumpVec;
use dioxus::prelude::*;
use dioxus_core::{exports::bumpalo, prelude::*, IntoVNode, Mutations};
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Label, Widget};
use snafu::Whatever;
use std::collections::HashMap;

struct Renderer {
    widgets: HashMap<u64, Widget>,
}

#[derive(Props)]
pub struct ViewProps<'a> {
    children: Element<'a>,
}

pub fn View<'a>(cx: Scope<'a, ViewProps<'a>>) -> Element {
    cx.render(LazyNodes::new(move |f| {
        let mut children: BumpVec<VNode> = BumpVec::new_in(f.bump());
        if let Some(ref node) = cx.props.children {
            children.push(node.into_vnode(f));
        }
        f.raw_element("gtk_box", None, &[], &[], children.into_bump_slice(), None)
    }))
}

#[derive(Props)]
pub struct TextProps<'a> {
    label: &'a str,
}
pub fn Text<'a>(cx: Scope<'a, TextProps<'a>>) -> Element {
    let nf = NodeFactory::new(&cx);
    let mut attrs = dioxus::core::exports::bumpalo::collections::Vec::new_in(nf.bump());
    attrs.push(nf.attr("text", format_args!("{}", cx.props.label), None, false));
    Some(nf.raw_element("gtk_label", None, &[], attrs.into_bump_slice(), &[], None))
}

impl Renderer {
    pub fn apply<'a>(&mut self, mutations: Mutations<'a>) {
        let mut stack = Vec::new();
        for edit in mutations.edits {
            match dbg!(edit) {
                dioxus_core::DomEdit::PushRoot { root } => stack.push(root),
                dioxus_core::DomEdit::AppendChildren { many } => {
                    let target_id = if stack.len() > many as usize {
                        stack[stack.len() - many as usize - 1]
                    } else {
                        0 // Fallback to the toplevel window
                    };
                    let target_widget = self.widgets[&target_id].clone();
                    for _ in 0..many {
                        let child_id = stack.pop().unwrap();
                        let child = self.widgets[&child_id].clone();
                        match target_widget.widget_name().as_str() {
                            "GtkBox" => target_widget
                                .clone()
                                .downcast::<gtk::Box>()
                                .unwrap()
                                .append(&child),
                            "GtkApplicationWindow" => target_widget
                                .clone()
                                .downcast::<ApplicationWindow>()
                                .unwrap()
                                .set_child(Some(&child)),
                            name => todo!("Don't know how to add children to {}", name),
                        }
                    }
                }
                dioxus_core::DomEdit::ReplaceWith { root, m } => todo!(),
                dioxus_core::DomEdit::InsertAfter { root, n } => todo!(),
                dioxus_core::DomEdit::InsertBefore { root, n } => todo!(),
                dioxus_core::DomEdit::Remove { root } => todo!(),
                dioxus_core::DomEdit::CreateTextNode { root, text } => todo!(),
                dioxus_core::DomEdit::CreateElement { root, tag } => {
                    self.widgets.insert(
                        root,
                        match tag {
                            "gtk_box" => gtk::Box::default().upcast::<Widget>(),
                            "gtk_label" => Label::default().upcast::<Widget>(),
                            _ => todo!("Have not built tag {} yet", tag),
                        },
                    );
                    stack.push(root);
                }
                dioxus_core::DomEdit::CreateElementNs { root, tag, ns } => todo!(),
                dioxus_core::DomEdit::CreatePlaceholder { root } => todo!(),
                dioxus_core::DomEdit::NewEventListener {
                    event_name,
                    scope,
                    root,
                } => todo!(),
                dioxus_core::DomEdit::RemoveEventListener { root, event } => todo!(),
                dioxus_core::DomEdit::SetText { root, text } => todo!(),
                dioxus_core::DomEdit::SetAttribute {
                    root,
                    field,
                    value,
                    ns,
                } => match (self.widgets[&root].widget_name().as_str(), field) {
                    ("GtkLabel", "text") => self.widgets[&root]
                        .clone()
                        .downcast::<Label>()
                        .unwrap()
                        .set_text(value),
                    _ => todo!(),
                },
                dioxus_core::DomEdit::RemoveAttribute { root, name, ns } => todo!(),
                dioxus_core::DomEdit::PopRoot {} => todo!(),
            }
        }
    }
}

pub fn launch(c: Component, application_id: &str) -> Result<(), Whatever> {
    let app = Application::builder()
        .application_id(application_id)
        .build();
    app.connect_activate(move |app: &Application| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Hello World")
            // .child(&button)
            .build();

        // Present window
        window.present();

        let mut widgets = HashMap::new();
        widgets.insert(0, window.upcast::<Widget>());
        let mut renderer = Renderer { widgets };
        let mut dom = VirtualDom::new(c);
        let mutations = dom.rebuild();
        renderer.apply(mutations);
    });
    app.run();
    Ok(())
}