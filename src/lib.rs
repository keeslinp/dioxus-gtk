use dioxus_core::{prelude::*, SchedulerMsg, UserEvent};
use futures::{select, FutureExt, StreamExt};
use gtk::glib::{clone, MainContext};
use gtk::prelude::*;
use gtk::Application;
use hashbrown::HashMap;
use renderer::{NativeWidget, Renderer, Widgets};
use snafu::Whatever;

pub mod components;
pub mod events;
mod renderer;
pub use taffy::*;

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
