use wasm_bindgen::JsValue;
use wasm_bindgen::UnwrapThrowExt;

use crate::components::canvas::geometry::get_port_center_static;
use crate::components::canvas::state::{ConnectionState, DraggingConnection, NodeState};

/// Draw a bezier wire on the canvas context
pub fn draw_bezier(
    ctx: &web_sys::CanvasRenderingContext2d,
    sx: f64,
    sy: f64,
    ex: f64,
    ey: f64,
    selected: bool,
    dimmed: bool,
) {
    let mid_x = (sx + ex) / 2.0;
    ctx.begin_path();
    ctx.move_to(sx, sy);
    ctx.bezier_curve_to(mid_x, sy, mid_x, ey, ex, ey);
    let color = if selected {
        "#6366f1"
    } else if dimmed {
        "#505050"
    } else {
        "#a0a0a0"
    };
    #[allow(deprecated)]
    ctx.set_stroke_style(&JsValue::from_str(color));
    ctx.set_line_width(2.0);
    ctx.stroke();
}

/// Draw all connections on the canvas
pub fn draw_connections(
    ctx: &web_sys::CanvasRenderingContext2d,
    connections: &[ConnectionState],
    dragging: &Option<DraggingConnection>,
    rerouting_from: Option<u32>,
    nodes: &[NodeState],
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
) {
    let canvas = ctx.canvas().unwrap();
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    ctx.clear_rect(0.0, 0.0, width, height);

    // Apply transform for pan/zoom
    ctx.set_transform(zoom, 0.0, 0.0, zoom, pan_x, pan_y).unwrap_throw();

    // Draw established connections
    for conn in connections {
        let (sx, sy) = get_port_center_static(conn.source_node_id, "output", nodes);
        let (ex, ey) = get_port_center_static(conn.target_node_id, "input", nodes);
        let dimmed = rerouting_from == Some(conn.target_node_id);
        draw_bezier(ctx, sx, sy, ex, ey, conn.selected, dimmed);
    }

    // Draw preview connection while dragging
    if let Some(ref dc) = dragging {
        if dc.is_dragging {
            let (sx, sy) = get_port_center_static(dc.source_node_id, "output", nodes);
            draw_bezier(ctx, sx, sy, dc.current_x, dc.current_y, false, false);
        }
    }

    // Reset transform
    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap_throw();
}
