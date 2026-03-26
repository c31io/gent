use std::collections::HashMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::UnwrapThrowExt;

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
    port_positions: &HashMap<(u32, String), (f64, f64)>,
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
) {
    let canvas = ctx.canvas().unwrap();
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    ctx.clear_rect(0.0, 0.0, width, height);

    // Helper to get first output port name for a node
    let get_first_output_port = |node_id: u32| -> String {
        nodes.iter()
            .find(|n| n.id == node_id)
            .and_then(|n| {
                use crate::components::canvas::state::get_output_ports;
                get_output_ports(&n.node_type, &n.variant)
                    .into_iter()
                    .next()
            })
            .map(|p| p.name)
            .unwrap_or_else(|| "output".to_string())
    };

    // Helper to get first input port name for a node
    let get_first_input_port = |node_id: u32| -> String {
        nodes.iter()
            .find(|n| n.id == node_id)
            .and_then(|n| {
                n.ports.iter()
                    .filter(|p| p.direction == crate::components::canvas::state::PortDirection::In)
                    .next()
            })
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "input".to_string())
    };

    // Apply transform for pan/zoom
    ctx.set_transform(zoom, 0.0, 0.0, zoom, pan_x, pan_y).unwrap_throw();

    // Draw established connections
    for conn in connections {
        let src_port = get_first_output_port(conn.source_node_id);
        let tgt_port = get_first_input_port(conn.target_node_id);
        let (sx, sy) = port_positions.get(&(conn.source_node_id, src_port)).copied().unwrap_or((0.0, 0.0));
        let (ex, ey) = port_positions.get(&(conn.target_node_id, tgt_port)).copied().unwrap_or((0.0, 0.0));
        let dimmed = rerouting_from == Some(conn.target_node_id);
        draw_bezier(ctx, sx, sy, ex, ey, conn.selected, dimmed);
    }

    // Draw preview connection while dragging
    if let Some(ref dc) = dragging {
        if dc.is_dragging {
            // For dragging, we don't have a specific port name, use first output
            let src_port = get_first_output_port(dc.source_node_id);
            let (sx, sy) = port_positions.get(&(dc.source_node_id, src_port)).copied().unwrap_or((0.0, 0.0));
            draw_bezier(ctx, sx, sy, dc.current_x, dc.current_y, false, false);
        }
    }

    // Reset transform
    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap_throw();
}
