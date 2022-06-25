#![allow(non_snake_case)]

use bumpalo::{boxed::Box as BumpBox, collections::Vec as BumpVec};
use dioxus::prelude::Props;
use dioxus_core::{exports::bumpalo, prelude::*, IntoVNode};
use taffy::style::Style;

use crate::events::{PressData, PressEvent};

#[derive(Props)]
pub struct ViewProps<'a> {
    children: Element<'a>,
    layout: Option<Style>,
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
pub fn Text(cx: Scope<'_, TextProps>) -> Element {
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
