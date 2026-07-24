//! Browser-native username field for wasm so mobile soft keyboards can open.

use std::sync::Mutex;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{HtmlInputElement, InputEvent};

use crate::app::MAX_USERNAME_LEN;

const INPUT_ID: &str = "inventory-jam-username";

static USERNAME: Mutex<String> = Mutex::new(String::new());

pub fn mount_browser_username(initial: &str) {
    remove_dom_input();

    if let Ok(mut username) = USERNAME.lock() {
        *username = initial.to_owned();
    }

    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(element) = document.create_element("input") else {
        return;
    };
    let Ok(input) = element.dyn_into::<HtmlInputElement>() else {
        return;
    };

    input.set_id(INPUT_ID);
    input.set_type("text");
    input.set_max_length(MAX_USERNAME_LEN as i32);
    input.set_spellcheck(false);
    input.set_autocomplete("username");
    input.set_placeholder("Username");
    input.set_value(initial);
    let _ = input.set_attribute("enterkeyhint", "done");
    let _ = input.set_attribute("autocapitalize", "none");
    let _ = input.set_attribute("autocorrect", "off");

    let on_input = Closure::wrap(Box::new(move |_event: InputEvent| {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Some(element) = document.get_element_by_id(INPUT_ID) else {
            return;
        };
        let Ok(input) = element.dyn_into::<HtmlInputElement>() else {
            return;
        };

        let mut text = input.value();
        if text.chars().count() > MAX_USERNAME_LEN {
            text = text.chars().take(MAX_USERNAME_LEN).collect();
            input.set_value(&text);
        }
        if let Ok(mut username) = USERNAME.lock() {
            *username = text;
        }
    }) as Box<dyn FnMut(InputEvent)>);

    if input
        .add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref())
        .is_err()
    {
        return;
    }

    let Some(body) = document.body() else {
        return;
    };
    if body.append_child(&input).is_err() {
        return;
    }

    // Keep the listener alive for the lifetime of the DOM node.
    on_input.forget();
}

pub fn browser_username_value() -> String {
    USERNAME
        .lock()
        .map(|username| username.clone())
        .unwrap_or_default()
}

pub fn unmount_browser_username() {
    remove_dom_input();
}

fn remove_dom_input() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if let Some(element) = document.get_element_by_id(INPUT_ID) {
        element.remove();
    }
}
