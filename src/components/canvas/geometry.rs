use wasm_bindgen::JsCast;

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

/// Check if the mouse event target is any port (input or output)
pub fn is_port(ev: &web_sys::MouseEvent) -> bool {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            return element.get_attribute("data-port").is_some();
        }
    }
    false
}

/// Check if the mouse event target is a text input or textarea (or inside it)
pub fn is_text_input(ev: &web_sys::MouseEvent) -> bool {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            // Walk up to check if we hit a text input or textarea
            let mut current: Option<web_sys::Element> = Some(element);
            while let Some(el) = current {
                let class_name = el.class_name();
                if class_name.contains("node-variant-input") || class_name.contains("node-variant-textarea") {
                    return true;
                }
                current = el.parent_element();
            }
        }
    }
    false
}

/// Check if keyboard event target is a text input (INPUT/TEXTAREA or node-variant-input/textarea)
pub fn is_text_input_keyboard(ev: &web_sys::KeyboardEvent) -> bool {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            let tag_name = element.tag_name();
            // Check standard HTML inputs
            if tag_name == "INPUT" || tag_name == "TEXTAREA" {
                return true;
            }
            // Walk up to check for node-variant text inputs
            let mut current: Option<web_sys::Element> = Some(element);
            while let Some(el) = current {
                let class_name = el.class_name();
                if class_name.contains("node-variant-input") || class_name.contains("node-variant-textarea") {
                    return true;
                }
                current = el.parent_element();
            }
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
