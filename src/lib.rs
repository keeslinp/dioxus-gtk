use bumpalo::collections::Vec as BumpVec;
use dioxus::prelude::*;
use dioxus_core::Attribute;
use dioxus_core::{exports::bumpalo, IntoVNode, Mutations};
use futures::{select, FutureExt, StreamExt};
use gtk::glib::{clone, timeout_future_seconds, MainContext, PRIORITY_LOW};
use gtk::{prelude::*, Fixed};
use gtk::{Application, ApplicationWindow, Label, Widget};
use hashbrown::HashMap;
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use snafu::Whatever;
use taffy::prelude::*;

#[derive(Default)]
struct Widgets {
    main: SlotMap<DefaultKey, ()>,
    gtk: SecondaryMap<DefaultKey, NativeWidget>,
    layout_root: SecondaryMap<DefaultKey, Fixed>,
    layout_parent: SecondaryMap<DefaultKey, DefaultKey>,
    taffy: SecondaryMap<DefaultKey, Node>,
}

struct Renderer {
    widgets: Widgets,
    roots: HashMap<u64, DefaultKey>,
    taffy_nodes: HashMap<Node, DefaultKey>,
    taffy: Taffy,
    app: Application,
}

pub use taffy::*;

#[derive(Props)]
pub struct ViewProps<'a> {
    children: Element<'a>,
    layout: Option<Style>,
}

enum NativeWidget {
    View(gtk::Box),
    Text(Label),
    Window(ApplicationWindow),
}

impl NativeWidget {
    fn upcast(&self) -> Widget {
        match self {
            NativeWidget::View(widget) => widget.clone().upcast::<Widget>(),
            NativeWidget::Text(widget) => widget.clone().upcast::<Widget>(),
            NativeWidget::Window(widget) => widget.clone().upcast::<Widget>(),
        }
    }
}

pub fn View<'a>(cx: Scope<'a, ViewProps<'a>>) -> Element {
    cx.render(LazyNodes::new(move |f| {
        let mut children: BumpVec<VNode> = BumpVec::new_in(f.bump());
        if let Some(ref node) = cx.props.children {
            children.push(node.into_vnode(f));
        }
        let mut attrs = BumpVec::new_in(f.bump());
        if let Some(ref layout) = cx.props.layout {
            attrs.push(f.attr(
                "layout",
                format_args!("{}", serde_json::to_string(layout).unwrap()),
                None,
                false,
            ));
        }
        f.raw_element(
            "gtk_box",
            None,
            &[],
            attrs.into_bump_slice(),
            children.into_bump_slice(),
            None,
        )
    }))
}

#[derive(Props)]
pub struct TextProps<'a> {
    label: &'a str,
}
pub fn Text<'a>(cx: Scope<'a, TextProps<'a>>) -> Element {
    let nf = NodeFactory::new(&cx);
    let mut attrs = BumpVec::new_in(nf.bump());
    attrs.push(nf.attr("text", format_args!("{}", cx.props.label), None, false));
    Some(nf.raw_element("gtk_label", None, &[], attrs.into_bump_slice(), &[], None))
}

#[derive(Props)]
pub struct WindowProps<'a> {
    title: &'a str,
    children: Element<'a>,
    layout: Option<Style>,
}

pub fn Window<'a>(cx: Scope<'a, WindowProps<'a>>) -> Element {
    cx.render(LazyNodes::new(move |f| {
        let mut children: BumpVec<VNode> = BumpVec::new_in(f.bump());
        if let Some(ref node) = cx.props.children {
            children.push(node.into_vnode(f));
        }
        let mut attrs = dioxus::core::exports::bumpalo::collections::Vec::new_in(f.bump());
        attrs.push(f.attr("title", format_args!("{}", cx.props.title), None, false));
        if let Some(ref layout) = cx.props.layout {
            attrs.push(f.attr(
                "layout",
                format_args!("{}", serde_json::to_string(layout).unwrap()),
                None,
                false,
            ));
        }
        f.raw_element(
            "gtk_window",
            None,
            &[],
            attrs.into_bump_slice(),
            children.into_bump_slice(),
            None,
        )
    }))
}

impl Renderer {
    pub fn apply<'a>(&mut self, mutations: Mutations<'a>) {
        let mut stack = Vec::new();
        use taffy::node::MeasureFunc::*;
        for edit in mutations.edits {
            match edit {
                dioxus_core::DomEdit::PushRoot { root } => stack.push(root),
                dioxus_core::DomEdit::AppendChildren { many } if (many as usize) < stack.len() => {
                    let target_root = stack[stack.len() - many as usize - 1];
                    let target_key = self.roots[&target_root];
                    let target_taffy = self.widgets.taffy[target_key];
                    let layout_root = self.widgets.layout_root.get(target_key).expect(
                        "Trying to add a child to a component which does not have a layout root",
                    );
                    for child_root in stack.drain(stack.len() - many as usize..) {
                        let child_key = self.roots[&child_root];
                        let child_widget = &self.widgets.gtk[child_key];
                        let child_taffy = self.widgets.taffy[child_key];
                        layout_root.put(&child_widget.upcast(), 0., 0.);
                        self.widgets.layout_parent.insert(child_key, target_key);
                        self.taffy
                            .add_child(target_taffy.clone(), child_taffy)
                            .unwrap();
                    }
                }
                dioxus_core::DomEdit::AppendChildren { many } if many == 1 && stack.len() == 1 => {
                    let target_key = self.roots[&stack.pop().unwrap()];
                    let target_widget = &self.widgets.gtk[target_key];
                    match target_widget {
                        NativeWidget::Window(widget) => widget.present(),
                        _ => unreachable!("Only Window can be a top-level component"),
                    }
                }
                dioxus_core::DomEdit::AppendChildren { .. } => {
                    unreachable!("I don't think this possible")
                }
                dioxus_core::DomEdit::ReplaceWith { root, m } => todo!(),
                dioxus_core::DomEdit::InsertAfter { root, n } => todo!(),
                dioxus_core::DomEdit::InsertBefore { root, n } => todo!(),
                dioxus_core::DomEdit::Remove { root } => todo!(),
                dioxus_core::DomEdit::CreateTextNode { root, text } => todo!(),
                dioxus_core::DomEdit::CreateElement { root, tag } => {
                    let key = self.widgets.main.insert(());
                    self.roots.insert(root, key);
                    match tag {
                        "gtk_box" => {
                            let gtk_box = gtk::Box::default();
                            self.widgets
                                .gtk
                                .insert(key, NativeWidget::View(gtk_box.clone()));

                            let layout_root = Fixed::builder().hexpand(true).vexpand(true).build();
                            gtk_box.append(&layout_root);
                            self.widgets.layout_root.insert(key, layout_root);
                            let taffy_node = self.taffy.new_node(Default::default(), &[]).unwrap();
                            self.widgets.taffy.insert(key, taffy_node.clone());
                            self.taffy_nodes.insert(taffy_node, key);
                        }
                        "gtk_label" => {
                            let label = Label::builder().valign(gtk::Align::Start).build();

                            self.widgets
                                .gtk
                                .insert(key, NativeWidget::Text(label.clone()));
                            let taffy_node = self
                                .taffy
                                .new_leaf(
                                    Default::default(),
                                    Boxed(Box::new(move |_| Size {
                                        width: label.width() as f32,
                                        height: label.height() as f32,
                                    })),
                                )
                                .unwrap();
                            self.widgets.taffy.insert(key, taffy_node.clone());
                            self.taffy_nodes.insert(taffy_node, key);
                        }
                        "gtk_window" => {
                            let window =
                                ApplicationWindow::builder().application(&self.app).build();
                            self.widgets
                                .gtk
                                .insert(key, NativeWidget::Window(window.clone()));
                            let layout_root = Fixed::builder().hexpand(true).vexpand(true).build();
                            window.set_child(Some(&layout_root));
                            self.widgets.layout_root.insert(key, layout_root);
                            let taffy_node = self
                                .taffy
                                .new_node(
                                    Style {
                                        size: Size {
                                            width: Dimension::Percent(1.),
                                            height: Dimension::Percent(1.),
                                        },
                                        ..Default::default()
                                    },
                                    &[],
                                )
                                .unwrap();
                            self.widgets.taffy.insert(key, taffy_node.clone());
                            self.taffy_nodes.insert(taffy_node, key);
                        }
                        _ => todo!("Have not built tag {} yet", tag),
                    };
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
                } => {
                    let key = self.roots[&root];
                    match (&self.widgets.gtk[key], self.widgets.taffy.get(key), field) {
                        (_, Some(taffy_node), "layout") => {
                            let layout = serde_json::from_str(value).unwrap();
                            layout;
                            self.taffy
                                .set_style(*taffy_node, layout)
                                .expect("failed to apply justify_content style");
                        }
                        (NativeWidget::Text(ref widget), _, "text") => {
                            widget.set_text(value);
                        }
                        (NativeWidget::Window(widget), _, "title") => {
                            widget.set_title(Some(value));
                        }
                        _ => todo!(),
                    };
                }
                dioxus_core::DomEdit::RemoveAttribute { root, name, ns } => todo!(),
                dioxus_core::DomEdit::PopRoot {} => {
                    stack.pop();
                }
            }
        }
    }

    pub fn recalculate_layout(&mut self) {
        let key = self.roots[&1];
        if let (Some(NativeWidget::Window(widget)), Some(taffy_node)) =
            (self.widgets.gtk.get(key), self.widgets.taffy.get(key))
        {
            self.taffy
                .compute_layout(
                    taffy_node.clone(),
                    Size {
                        width: Number::Defined(widget.default_width() as f32),
                        height: Number::Defined(widget.default_height() as f32),
                    },
                )
                .unwrap();
            self.apply_layout_changes();
        }
    }
    fn apply_layout_changes(&mut self) {
        let mut stack = vec![self.roots[&1]];
        while let Some(node) = stack.pop() {
            let taffy_node = self.widgets.taffy[node];
            if let Ok(children) = self.taffy.children(taffy_node) {
                for key in children.iter().map(|child| self.taffy_nodes[&child]) {
                    stack.push(key);
                }
            }
            let gtk_node = &self.widgets.gtk[node];
            let layout = self.taffy.layout(taffy_node).unwrap();
            if let Some(layout_root) = self
                .widgets
                .layout_parent
                .get(node)
                .and_then(|parent| self.widgets.layout_root.get(*parent))
            {
                layout_root.move_(
                    &gtk_node.upcast(),
                    layout.location.x as f64,
                    layout.location.y as f64,
                );
            }
            gtk_node
                .upcast()
                .set_width_request(layout.size.width as i32);
            gtk_node
                .upcast()
                .set_height_request(layout.size.height as i32);
        }
    }
}

enum MainEvent {
    Resize,
    Render,
}

pub fn launch(c: Component, application_id: &str) -> Result<(), Whatever> {
    let app = Application::builder()
        .application_id(application_id)
        .build();
    app.connect_activate(move |app: &Application| {
        let mut renderer = Renderer {
            widgets: Widgets::default(),
            taffy: Taffy::new(),
            roots: HashMap::new(),
            taffy_nodes: HashMap::new(),
            app: app.clone(),
        };
        let mut dom = VirtualDom::new(c);
        let mutations = dom.rebuild();
        renderer.apply(mutations);
        let (sender, mut receiver) = futures::channel::mpsc::unbounded::<MainEvent>();
        if let NativeWidget::Window(window) = &renderer.widgets.gtk[renderer.roots[&1]] {
            window.connect_default_height_notify(move |_window| {
                sender.unbounded_send(MainEvent::Resize).unwrap();
            });
        }
        let main_context = MainContext::default();
        main_context.spawn_local(clone!(@strong app => async move {
            loop {
                match select!(
                    _ = receiver.next() => MainEvent::Resize,
                    _ = dom.wait_for_work().fuse() => MainEvent::Render,
                ) {
                    MainEvent::Resize => {
                        renderer.recalculate_layout();
                    },
                    MainEvent::Render => {
                        let mutations = dom.rebuild();
                        renderer.apply(mutations);
                        renderer.recalculate_layout();
                    }
                }
            }
        }));
    });
    app.run();
    Ok(())
}
