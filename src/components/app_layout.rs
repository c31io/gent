use leptos::prelude::*;

use crate::components::left_panel::LeftPanel;
use crate::components::right_panel::RightPanel;
use crate::components::canvas::Canvas;

/// Main application layout with left panel, canvas, and right panel
#[component]
pub fn AppLayout() -> impl IntoView {
    // Shared state for panel sizes (in pixels)
    let (left_width, set_left_width) = signal(260i32);
    let (right_width, set_right_width) = signal(300i32);

    // Track if dragging divider
    let (dragging_left, set_dragging_left) = signal(false);
    let (dragging_right, set_dragging_right) = signal(false);

    let handle_left_divider_mouse_down = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_dragging_left.set(true);
    };

    let handle_right_divider_mouse_down = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_dragging_right.set(true);
    };

    let handle_mouse_move = move |ev: web_sys::MouseEvent| {
        if dragging_left.get() {
            let new_width = ev.client_x();
            if new_width >= 180 && new_width <= 500 {
                set_left_width.set(new_width);
            }
        }
        if dragging_right.get() {
            let window = web_sys::window().unwrap();
            let inner_width = window.inner_width().unwrap().as_f64().unwrap() as i32;
            let new_width = inner_width - ev.client_x();
            if new_width >= 180 && new_width <= 500 {
                set_right_width.set(new_width);
            }
        }
    };

    let handle_mouse_up = move |_ev: web_sys::MouseEvent| {
        set_dragging_left.set(false);
        set_dragging_right.set(false);
    };

    view! {
        <div
            class="app-layout"
            on:mousemove=handle_mouse_move
            on:mouseup=handle_mouse_up
            on:mouseleave=handle_mouse_up
        >
            {/* Left Panel */}
            <div
                class="panel"
                style:width=move || format!("{}px", left_width.get())
            >
                <LeftPanel />
            </div>

            {/* Left Divider */}
            <div
                class="divider"
                on:mousedown=handle_left_divider_mouse_down
            ></div>

            {/* Canvas */}
            <Canvas />

            {/* Right Divider */}
            <div
                class="divider"
                on:mousedown=handle_right_divider_mouse_down
            ></div>

            {/* Right Panel */}
            <div
                class="panel panel-right"
                style:width=move || format!("{}px", right_width.get())
            >
                <RightPanel />
            </div>
        </div>
    }
}
