//! Reusable pieces: icon buttons, segmented controls, switches, the themed dropdown, probes.

use std::rc::Rc;

use gpui::{
    canvas, div, prelude::*, px, svg, AnyElement, Context, Div, ElementId, FontWeight, Hsla, SharedString, Stateful, Svg,
    Window,
};

use super::{
    app::{Clipdat, Opt},
    icons,
    theme::*,
};

pub fn icon(name: &str, size: f32, color: Hsla) -> Svg {
    svg().path(icons::icon(name)).size(px(size)).flex_none().text_color(color)
}

pub fn section_title(text: &'static str) -> Div {
    div().mb(px(8.)).text_size(px(11.)).font_weight(FontWeight::BOLD).text_color(text3()).child(text.to_uppercase())
}

pub fn sub_header(text: &'static str) -> Div {
    div()
        .mt(px(18.))
        .mb(px(10.))
        .flex()
        .items_center()
        .gap(px(8.))
        .child(div().text_size(px(11.)).font_weight(FontWeight::BOLD).text_color(text3()).child(text.to_uppercase()))
        .child(div().flex_1().h(px(1.)).bg(stroke()))
}

pub fn note(icon_name: &str, color: Hsla, text: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(6.))
        .min_w(px(0.))
        .text_size(px(11.5))
        .text_color(color)
        .child(div().pt(px(2.)).child(icon(icon_name, 13., color)))
        .child(div().flex_1().min_w(px(0.)).child(text.into()))
}

pub fn field_label(text: impl Into<SharedString>) -> Div {
    div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(text2()).child(text.into())
}

impl Clipdat {
    /// Records an element's window-space bounds (for drag math and menu anchors).
    pub fn probe_el(&self, id: &'static str) -> impl IntoElement {
        let probes = self.probes.clone();
        let me = self.me.clone();
        canvas(
            move |bounds, _window, cx| {
                let mut p = probes.borrow_mut();
                if p.get(id) != Some(&bounds) {
                    p.insert(id, bounds);
                    // Layout-dependent content (timeline tiles, menus) needs one more frame.
                    let me = me.clone();
                    cx.defer(move |cx| {
                        let _ = me.update(cx, |_, cx| cx.notify());
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full()
    }

    pub fn ib(
        &self,
        id: impl Into<ElementId>,
        name: &str,
        size: f32,
        box_size: f32,
        cx: &mut Context<Self>,
        on: impl Fn(&mut Clipdat, &mut Window, &mut Context<Clipdat>) + 'static,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .size(px(box_size))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(9.))
            .cursor_pointer()
            .opacity(0.86)
            .hover(|s| s.bg(hover()).opacity(1.))
            .child(icon(name, size, text()))
            .on_click(cx.listener(move |this, _, window, cx| {
                on(this, window, cx);
                cx.notify();
            }))
    }

    pub fn button(
        &self,
        id: impl Into<ElementId>,
        icon_name: Option<&str>,
        label: impl Into<SharedString>,
        primary: bool,
        cx: &mut Context<Self>,
        on: impl Fn(&mut Clipdat, &mut Window, &mut Context<Clipdat>) + 'static,
    ) -> Stateful<Div> {
        let fg = if primary { on_clip() } else { text() };
        div()
            .id(id)
            .h(px(32.))
            .px(px(14.))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(7.))
            .rounded(px(9.))
            .text_size(px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .whitespace_nowrap()
            .cursor_pointer()
            .text_color(fg)
            .when(primary, |d| d.bg(clip()).hover(|s| s.opacity(0.92)))
            .when(!primary, |d| d.border_1().border_color(stroke2()).hover(|s| s.bg(soft())))
            .when_some(icon_name, |d, n| d.child(icon(n, 16., fg)))
            .child(label.into())
            .on_click(cx.listener(move |this, _, window, cx| {
                on(this, window, cx);
                cx.notify();
            }))
    }

    pub fn seg<T: Copy + PartialEq + 'static>(
        &self,
        id: &'static str,
        value: T,
        opts: Vec<(T, SharedString, bool)>,
        full: bool,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Clipdat, T) + 'static,
    ) -> Div {
        let set = Rc::new(set);
        div()
            .flex()
            .p(px(3.))
            .rounded(px(10.))
            .bg(soft())
            .when(full, |d| d.w_full())
            .children(opts.into_iter().enumerate().map(|(i, (v, label, disabled))| {
                let on = v == value;
                let set = set.clone();
                div()
                    .id((id, i))
                    .h(px(26.))
                    .px(px(11.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(full, |d| d.flex_1())
                    .rounded(px(7.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .whitespace_nowrap()
                    .text_color(if on { text() } else { text2() })
                    .when(on, |d| d.bg(rgba_white(0x21)))
                    .when(disabled, |d| d.opacity(0.35))
                    .when(!disabled, |d| {
                        d.cursor_pointer().hover(|s| s.text_color(text())).on_click(cx.listener(move |this, _, _, cx| {
                            set(this, v);
                            this.changed(cx);
                        }))
                    })
                    .child(label)
            }))
    }

    pub fn switch(
        &self,
        id: impl Into<ElementId>,
        on: bool,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Clipdat, bool) + 'static,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .w(px(36.))
            .h(px(21.))
            .flex_none()
            .rounded(px(11.))
            .bg(if on { accent() } else { rgba_white(0x29) })
            .cursor_pointer()
            .relative()
            .child(
                div()
                    .absolute()
                    .top(px(2.5))
                    .left(px(if on { 17.5 } else { 2.5 }))
                    .size(px(16.))
                    .rounded(px(8.))
                    .bg(if on { on_clip() } else { text() }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                set(this, !on);
                this.changed(cx);
            }))
    }

    /// A label + hint on the left, a switch on the right.
    pub fn toggle_row(
        &self,
        id: &'static str,
        label: &'static str,
        hint: Option<&'static str>,
        on: bool,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Clipdat, bool) + 'static,
    ) -> Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .py(px(7.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(label)
                    .when_some(hint, |d, h| {
                        d.child(div().text_size(px(11.5)).font_weight(FontWeight::MEDIUM).text_color(text3()).child(h))
                    }),
            )
            .child(self.switch(id, on, cx, set))
    }

    /// Themed dropdown: a field-style button that opens a floating glass menu.
    pub fn dd<T: Clone + PartialEq + 'static>(
        &self,
        id: &'static str,
        items: Vec<(T, Opt)>,
        current: T,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Clipdat, T) + 'static,
    ) -> AnyElement {
        let sel = items.iter().position(|(v, _)| *v == current).unwrap_or(0);
        let label = items.get(sel).map(|(_, o)| o.label.clone()).unwrap_or_else(|| "—".into());
        let open = self.menu.as_ref().is_some_and(|m| m.id == id);
        let opts: Vec<Opt> = items.iter().map(|(_, o)| o.clone()).collect();
        let values: Rc<Vec<T>> = Rc::new(items.into_iter().map(|(v, _)| v).collect());
        let set = Rc::new(set);
        div()
            .id(id)
            .relative()
            .h(px(34.))
            .w_full()
            .min_w(px(0.))
            .px(px(11.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(px(9.))
            .border_1()
            .border_color(if open { accent() } else { stroke2() })
            .bg(field())
            .cursor_pointer()
            .hover(|s| s.border_color(rgba_white(0x3d)))
            .child(self.probe_el(id))
            .child(div().flex_1().min_w(px(0.)).overflow_hidden().whitespace_nowrap().text_ellipsis().child(label))
            .child(icon("chev", 15., if open { text() } else { text3() }))
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.menu.as_ref().is_some_and(|m| m.id == id) {
                    this.close_menu();
                } else {
                    let (values, set) = (values.clone(), set.clone());
                    this.open_menu(
                        id,
                        opts.clone(),
                        sel,
                        Rc::new(move |this, i, _w, _cx| {
                            if let Some(v) = values.get(i) {
                                set(this, v.clone());
                            }
                        }),
                    );
                }
                cx.notify();
            }))
            .into_any_element()
    }

    pub fn dd_field<T: Clone + PartialEq + 'static>(
        &self,
        label: impl Into<SharedString>,
        id: &'static str,
        items: Vec<(T, Opt)>,
        current: T,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Clipdat, T) + 'static,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .min_w(px(0.))
            .child(field_label(label))
            .child(self.dd(id, items, current, cx, set))
    }
}

pub fn rgba_white(alpha: u8) -> Hsla {
    gpui::rgba(0xffffff00 | alpha as u32).into()
}
