use crate::{events, MainEvent};
use dioxus_core::{ElementId, EventPriority, Mutations, UserEvent};
use futures::channel::mpsc::UnboundedSender;
use gtk::{prelude::*, Application, ApplicationWindow, Fixed, Label, Widget};
use hashbrown::HashMap;
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::sync::Arc;
use taffy::prelude::*;

#[derive(Default)]
pub struct Widgets {
    pub main: SlotMap<DefaultKey, ()>,
    pub gtk: SecondaryMap<DefaultKey, NativeWidget>,
    pub layout_root: SecondaryMap<DefaultKey, Fixed>,
    pub layout_parent: SecondaryMap<DefaultKey, DefaultKey>,
    pub taffy: SecondaryMap<DefaultKey, Node>,
}

pub struct Renderer {
    pub widgets: Widgets,
    pub roots: HashMap<u64, DefaultKey>,
    pub taffy_nodes: HashMap<Node, DefaultKey>,
    pub taffy: Taffy,
    pub app: Application,
    pub(crate) sender: UnboundedSender<MainEvent>,
}

pub enum NativeWidget {
    View(gtk::Box),
    Text(Label),
    Window(ApplicationWindow),
    Button(gtk::Button),
    TextField(gtk::Entry),
}

impl NativeWidget {
    fn upcast(&self) -> Widget {
        match self {
            NativeWidget::View(widget) => widget.clone().upcast::<Widget>(),
            NativeWidget::Text(widget) => widget.clone().upcast::<Widget>(),
            NativeWidget::Window(widget) => widget.clone().upcast::<Widget>(),
            NativeWidget::Button(widget) => widget.clone().upcast::<Widget>(),
            NativeWidget::TextField(widget) => widget.clone().upcast::<Widget>(),
        }
    }
}

impl Renderer {
    pub fn apply(&mut self, mutations: Mutations) {
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
                        self.widgets.layout_parent.insert(child_key, target_key);
                        if let Some(child_widget) = self.widgets.gtk.get(child_key) {
                            layout_root.put(&child_widget.upcast(), 0., 0.);
                        }
                        if let Some(child_taffy) = self.widgets.taffy.get(child_key) {
                            self.taffy.add_child(target_taffy, *child_taffy).unwrap();
                        }
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
                dioxus_core::DomEdit::ReplaceWith { root, m } => {
                    let replace_key = self.roots[&root];
                    let parent = self.widgets.layout_parent.remove(replace_key);
                    if let (Some(widget), Some(parent_widget)) = (
                        self.widgets.gtk.remove(replace_key),
                        parent.and_then(|key| self.widgets.layout_root.get(key)),
                    ) {
                        parent_widget.remove(&widget.upcast());
                    }
                    if let (Some(child_node), Some(parent_node)) = (
                        self.widgets.taffy.remove(replace_key),
                        parent.and_then(|key| self.widgets.taffy.get(key)),
                    ) {
                        self.taffy.remove_child(*parent_node, child_node).unwrap();
                    }
                    self.widgets.main.remove(replace_key);
                    self.widgets.layout_root.remove(replace_key);
                    for child_root in stack.drain(stack.len() - m as usize..) {
                        let child_key = self.roots[&child_root];
                        if let Some(parent) = parent {
                            self.widgets.layout_parent.insert(child_key, parent);
                            if let (Some(parent_node), Some(child_node)) = (
                                self.widgets.taffy.get(parent),
                                self.widgets.taffy.get(child_key),
                            ) {
                                self.taffy.add_child(*parent_node, *child_node).unwrap();
                            }

                            if let (Some(parent_layout), Some(child_widget)) = (
                                self.widgets.layout_root.get(parent),
                                self.widgets.gtk.get(child_key),
                            ) {
                                parent_layout.put(&child_widget.upcast(), 0., 0.);
                            }
                        }
                    }
                }
                dioxus_core::DomEdit::InsertAfter { .. } => todo!(),
                dioxus_core::DomEdit::InsertBefore { .. } => todo!(),
                dioxus_core::DomEdit::Remove { .. } => todo!(),
                dioxus_core::DomEdit::CreateTextNode { .. } => todo!(),
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
                            self.widgets.taffy.insert(key, taffy_node);
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
                            self.widgets.taffy.insert(key, taffy_node);
                            self.taffy_nodes.insert(taffy_node, key);
                        }
                        "gtk_text_field" => {
                            let text_field =
                                gtk::Entry::builder().valign(gtk::Align::Start).build();
                            text_field.set_width_chars(50);
                            self.widgets
                                .gtk
                                .insert(key, NativeWidget::TextField(text_field.clone()));
                            let taffy_node = self
                                .taffy
                                .new_leaf(
                                    Default::default(),
                                    Boxed(Box::new(move |_| Size {
                                        width: text_field.allocated_width() as f32,
                                        height: text_field.allocated_height() as f32,
                                    })),
                                )
                                .unwrap();
                            self.widgets.taffy.insert(key, taffy_node);
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
                            self.widgets.taffy.insert(key, taffy_node);
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
                            self.widgets.taffy.insert(key, taffy_node);
                            self.taffy_nodes.insert(taffy_node, key);
                        }
                        _ => todo!("Have not built tag {} yet", tag),
                    };
                    stack.push(root);
                }
                dioxus_core::DomEdit::CreateElementNs { .. } => todo!(),
                dioxus_core::DomEdit::CreatePlaceholder { root } => {
                    let key = self.widgets.main.insert(());
                    self.roots.insert(root, key);
                    stack.push(root);
                }
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
                                        data: Arc::new(events::PressData {}),
                                    }))
                                    .unwrap();
                            });
                        }
                        (NativeWidget::TextField(widget), "text_change") => {
                            let sender = self.sender.clone();
                            widget.connect_text_notify(move |field| {
                                sender
                                    .unbounded_send(MainEvent::UserEvent(UserEvent {
                                        scope_id: Some(scope),
                                        priority: EventPriority::High,
                                        element: Some(ElementId(root as usize)),
                                        name: event_name,
                                        data: Arc::new(events::TextChangeData {
                                            value: field.text().into(),
                                        }),
                                    }))
                                    .unwrap();
                            });
                        }
                        (_, evt) => todo!("Event not implemented for that component: {}", evt),
                    }
                }
                dioxus_core::DomEdit::RemoveEventListener { .. } => todo!(),
                dioxus_core::DomEdit::SetText { .. } => todo!(),
                dioxus_core::DomEdit::SetAttribute {
                    root, field, value, ..
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
                        (NativeWidget::TextField(widget), _, "place_holder") => {
                            widget.set_placeholder_text(Some(value));
                        }
                        (NativeWidget::TextField(widget), _, "value") => {
                            if value != widget.text().as_str() {
                                widget.set_text(value);
                            }
                        }
                        _ => todo!(),
                    };
                }
                dioxus_core::DomEdit::RemoveAttribute { .. } => todo!(),
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
                    *taffy_node,
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
                for key in children.iter().map(|child| self.taffy_nodes[child]) {
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
