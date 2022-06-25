use std::sync::Arc;

use bumpalo::{boxed::Box as BumpBox, collections::Vec as BumpVec};
use dioxus::prelude::Props;
use dioxus_core::{
    exports::bumpalo, prelude::*, IntoVNode, Mutations, SchedulerMsg, UiEvent, UserEvent,
};
use dioxus_core::{ElementId, EventPriority};
use futures::channel::mpsc::UnboundedSender;
use futures::{select, FutureExt, StreamExt};
use gtk::glib::{clone, MainContext};
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
    sender: UnboundedSender<MainEvent>,
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
    Button(gtk::Button),
}

impl NativeWidget {
    fn upcast(&self) -> Widget {
        match self {
            NativeWidget::View(widget) => widget.clone().upcast::<Widget>(),
            NativeWidget::Text(widget) => widget.clone().upcast::<Widget>(),
            NativeWidget::Window(widget) => widget.clone().upcast::<Widget>(),
            NativeWidget::Button(widget) => widget.clone().upcast::<Widget>(),
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

#[derive(Props, PartialEq)]
pub struct TextProps {
    label: String,
    layout: Option<Style>,
}
pub fn Text<'a>(cx: Scope<'a, TextProps>) -> Element {
    cx.render(LazyNodes::new(move |f| {
        let mut attrs = BumpVec::new_in(f.bump());
        attrs.push(f.attr("text", format_args!("{}", cx.props.label), None, false));
        if let Some(ref layout) = cx.props.layout {
            attrs.push(f.attr(
                "layout",
                format_args!("{}", serde_json::to_string(layout).unwrap()),
                None,
                false,
            ));
        }
        f.raw_element("gtk_label", None, &[], attrs.into_bump_slice(), &[], None)
    }))
}

// TODO: Add values
pub struct PressData {}

type PressEvent = UiEvent<PressData>;

#[derive(Props)]
pub struct ButtonProps<'a> {
    label: String,
    layout: Option<Style>,
    on_press: EventHandler<'a, PressEvent>,
}
pub fn Button<'a>(cx: Scope<'a, ButtonProps<'a>>) -> Element {
    cx.render(LazyNodes::new(move |f| {
        let bump = &f.bump();
        let mut attrs = BumpVec::new_in(bump);
        attrs.push(f.attr("label", format_args!("{}", cx.props.label), None, false));
        if let Some(ref layout) = cx.props.layout {
            attrs.push(f.attr(
                "layout",
                format_args!("{}", serde_json::to_string(layout).unwrap()),
                None,
                false,
            ));
        }
        let mut listeners = BumpVec::new_in(bump);

        use dioxus_core::AnyEvent;
        // we can't allocate unsized in bumpalo's box, so we need to craft the box manually
        // safety: this is essentially the same as calling Box::new() but manually
        // The box is attached to the lifetime of the bumpalo allocator
        let cb: &mut dyn FnMut(AnyEvent) = bump.alloc(move |evt: AnyEvent| {
            let event = evt.downcast::<PressData>().unwrap();
            cx.props.on_press.call(event);
        });

        let callback: BumpBox<dyn FnMut(AnyEvent) + 'a> = unsafe { BumpBox::from_raw(cb) };

        let handler = bump.alloc(std::cell::RefCell::new(Some(callback)));
        listeners.push(f.listener("press", handler));
        f.raw_element(
            "gtk_button",
            None,
            listeners.into_bump_slice(),
            attrs.into_bump_slice(),
            &[],
            None,
        )
    }))
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
                        "gtk_button" => {
                            let button = gtk::Button::builder().valign(gtk::Align::Start).build();
                            self.widgets
                                .gtk
                                .insert(key, NativeWidget::Button(button.clone()));
                            let taffy_node = self
                                .taffy
                                .new_leaf(
                                    Default::default(),
                                    Boxed(Box::new(move |_| Size {
                                        width: button.allocated_width() as f32,
                                        height: button.allocated_height() as f32,
                                    })),
                                )
                                .unwrap();
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
                                        width: label.allocated_width() as f32,
                                        height: label.allocated_height() as f32,
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
                } => {
                    let key = self.roots[&root];
                    match (&self.widgets.gtk[key], event_name) {
                        (NativeWidget::Button(widget), "press") => {
                            let sender = self.sender.clone();
                            widget.connect_clicked(move |_| {
                                sender
                                    .unbounded_send(MainEvent::UserEvent(UserEvent {
                                        scope_id: Some(scope),
                                        priority: EventPriority::High,
                                        element: Some(ElementId(root as usize)),
                                        name: event_name,
                                        data: Arc::new(PressData {}),
                                    }))
                                    .unwrap();
                            });
                        }
                        _ => unimplemented!(),
                    }
                }
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
                            self.taffy
                                .set_style(*taffy_node, layout)
                                .expect("failed to apply justify_content style");
                        }
                        (NativeWidget::Text(ref widget), Some(taffy_node), "text") => {
                            widget.set_text(value);
                            self.taffy.mark_dirty(*taffy_node).unwrap();
                        }
                        (NativeWidget::Window(widget), _, "title") => {
                            widget.set_title(Some(value));
                        }
                        (NativeWidget::Button(widget), Some(taffy_node), "label") => {
                            widget.set_label(value);
                            self.taffy.mark_dirty(*taffy_node).unwrap();
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
        }
    }
}

enum MainEvent {
    Resize,
    Render,
    UserEvent(UserEvent),
}

pub fn launch(c: Component, application_id: &str) -> Result<(), Whatever> {
    let app = Application::builder()
        .application_id(application_id)
        .build();
    app.connect_activate(move |app: &Application| {
        let (sender, mut receiver) = futures::channel::mpsc::unbounded::<MainEvent>();
        let mut renderer = Renderer {
            widgets: Widgets::default(),
            taffy: Taffy::new(),
            roots: HashMap::new(),
            taffy_nodes: HashMap::new(),
            app: app.clone(),
            sender: sender.clone(),
        };
        let mut dom = VirtualDom::new(c);
        let mutations = dom.rebuild();
        renderer.apply(mutations);
        if let NativeWidget::Window(window) = &renderer.widgets.gtk[renderer.roots[&1]] {
            window.connect_default_height_notify(clone!(@strong sender => move |_window| {
                sender.unbounded_send(MainEvent::Resize).unwrap();
            }));
            window.connect_default_width_notify(clone!(@strong sender => move |_window| {
                sender.unbounded_send(MainEvent::Resize).unwrap();
            }));
        }
        let main_context = MainContext::default();
        main_context.spawn_local(clone!(@strong app => async move {
            loop {
                match select!(
                    evt = receiver.next() => evt.unwrap(),
                    _ = dom.wait_for_work().fuse() => MainEvent::Render,
                ) {
                    MainEvent::Resize => {
                        renderer.recalculate_layout();
                    },
                    MainEvent::Render => {
                        for edits in dom.work_with_deadline(|| false) {
                            renderer.apply(edits);
                        }
                        renderer.recalculate_layout();
                    },
                    MainEvent::UserEvent(evt) => {
                        dom.handle_message(SchedulerMsg::Event(evt));
                    },
                }
            }
        }));
    });
    app.run();
    Ok(())
}
