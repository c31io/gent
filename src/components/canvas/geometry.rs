use wasm_bindgen::JsCast;

use crate::components::canvas::state::NodeState;

/// Find input port element at given viewport coordinates
pub fn find_input_port_at(x: f64, y: f64) -> Option<u32> {
    let doc = web_sys::window()?.document()?;
    let element = doc.element_from_point(x as f32, y as f32)?;
    let port_type = element.get_attribute("data-port")?;
    if port_type != "input" {
        return None;
    }
    let node_id = element.get_attribute("data-node-id")?.parse().ok()?;
    Some(node_id)
}

/// Check if the mouse event target is an input port
pub fn is_input_port(ev: &web_sys::MouseEvent) -> bool {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            if let Some(port_type) = element.get_attribute("data-port") {
                return port_type == "input";
            }
        }
    }
    false
}

/// Check if the mouse event target is any port (input or output)
pub fn is_port(ev: &web_sys::MouseEvent) -> bool {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            return element.get_attribute("data-port").is_some();
        }
    }
    false
}

/// Check if the mouse event target is the trigger button (or inside it)
pub fn is_trigger_button(ev: &web_sys::MouseEvent) -> bool {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            // Walk up to check if we hit a .trigger-btn
            let mut current: Option<web_sys::Element> = Some(element);
            while let Some(el) = current {
                if el.class_name().contains("trigger-btn") {
                    return true;
                }
                current = el.parent_element();
            }
        }
    }
    false
}

/// Get node_id from mouse event - traverses up to find the node div
pub fn get_node_id_from_event(ev: &web_sys::MouseEvent) -> Option<u32> {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            // Walk up the DOM tree to find the node div
            let mut current: Option<web_sys::Element> = Some(element);
            while let Some(el) = current {
                // If this element has data-node-id AND it's not a port, it's a node
                if el.get_attribute("data-node-id").is_some()
                    && el.get_attribute("data-port").is_none()
                {
                    return el.get_attribute("data-node-id")?.parse().ok();
                }
                // Move to parent element
                current = el.parent_element();
            }
        }
    }
    None
}

/// Get port center position from a nodes slice (pure version for wire drawing)
pub fn get_port_center_static(node_id: u32, port_type: &str, nodes: &[NodeState]) -> (f64, f64) {
    if let Some(node) = nodes.iter().find(|n| n.id == node_id) {
        let port_offset_x = if port_type == "output" { 150.0 } else { 0.0 };
        let port_offset_y = 35.0;
        let x = node.x + port_offset_x;
        let y = node.y + port_offset_y;
        (x, y)
    } else {
        (0.0, 0.0)
    }
}
