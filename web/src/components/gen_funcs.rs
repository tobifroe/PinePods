use std::collections::HashMap;
use ammonia::Builder;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DomParser, SupportedType, XmlHttpRequest};
use wasm_bindgen::JsCast;
use web_sys::window;
use yew::prelude::*;
use crate::requests::login_requests::use_check_authentication;
use yew::prelude::*;
use yewdux::prelude::Dispatch;
use crate::components::context::AppState;
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHasher, SaltString
    },
    Argon2
};
use otpauth::TOTP;
use js_sys::Date;


pub fn format_date(date_str: &str) -> String {
    let date = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S")
        .unwrap_or_else(|_| chrono::NaiveDateTime::from_timestamp(0, 0)); // Fallback for parsing error
    date.format("%m-%d-%Y").to_string()
}

pub fn truncate_description(description: String, max_length: usize) -> (String, bool) {
    let is_truncated = description.len() > max_length;

    let truncated_html = if is_truncated {
        description.chars().take(max_length).collect::<String>() + "..."
    } else {
        description.to_string()
    };

    (truncated_html, is_truncated)
}


pub fn sanitize_html_with_blank_target(description: &str) -> String {
    // Create the inner HashMap for attribute "target" with value "_blank"
    let mut attribute_values = HashMap::new();
    attribute_values.insert("target", "_blank");

    // Create the outer HashMap for tag "a"
    let mut tag_attribute_values = HashMap::new();
    tag_attribute_values.insert("a", attribute_values);

    // Configure the builder with the correct attribute values
    let mut builder = Builder::default();
    builder.add_tags(&["a"]); // ensure <a> tags are allowed
    builder.add_tag_attributes("a", &["href", "target"]); // allow href and target attributes on <a> tags
    builder.set_tag_attribute_values(tag_attribute_values); // set target="_blank" on all <a> tags

    // Clean the input HTML with the specified builder
    builder.clean(description).to_string()
}
// pub fn sanitize_html(description: &str) -> String {
//     let sanitized_html = clean(description);
// }

pub fn check_auth(effect_dispatch: Dispatch<AppState>) {
    use_effect_with(
        (),
        move |_| {
            let effect_dispatch_clone = effect_dispatch.clone();

            spawn_local(async move {
                let window = window().expect("no global `window` exists");
                let location = window.location();
                let current_route = location.href().expect("should be able to get href");
                use_check_authentication(effect_dispatch_clone, &current_route);
            });

            || ()
        }
    );
}



pub fn encode_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?.to_string();
    Ok(password_hash)
}


pub fn validate_user_input(username: &str, password: &str, email: &str) -> Result<(), String> {
    if username.len() < 4 {
        return Err("Username must be at least 4 characters long".to_string());
    }

    if password.len() < 6 {
        return Err("Password must be at least 6 characters long".to_string());
    }

    let email_regex = regex::Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
    if !email_regex.is_match(email) {
        return Err("Email is not in a valid format".to_string());
    }

    Ok(())
}

/// Verifies a TOTP code provided by the user against the stored secret.
///
/// # Arguments
///
/// * `secret` - The secret used to generate the TOTP codes.
/// * `code` - The user-entered TOTP code as a string.
///
/// # Returns
///
/// * `bool` - `true` if the code is valid, `false` otherwise.
pub fn verify_totp_code(secret: &str, code: &str) -> bool {
    let totp = TOTP::new(secret);
    let timestamp = Date::now() as u64 / 1000; // Convert milliseconds to seconds

    match code.parse::<u32>() {
        Ok(code_num) => totp.verify(code_num, 30, timestamp),
        Err(_) => false,
    }
}


// fn window() -> Window {
//     web_sys::window().expect("no global `window` exists")
// }
//
// fn document() -> Document {
//     window().document().expect("should have a document on window")
// }

// pub fn parse_opml(opml_content: &str) -> Vec<(String, String)> {
//     let parser = window().dom_parser().expect("should get DOM parser");
//     let doc = parser.parse_from_string(&opml_content, web_sys::SupportedType::Xml)
//         .expect("should parse the document");
//
//     let outlines = doc.query_selector_all("outline").expect("should query for outlines");
//     let mut podcasts = Vec::new();
//
//     for i in 0..outlines.length() {
//         if let Some(outline) = outlines.item(i).and_then(|item| item.dyn_into::<Element>().ok()) {
//             let title = outline.get_attribute("title").unwrap_or_default();
//             let xml_url = outline.get_attribute("xmlUrl").unwrap_or_default();
//             podcasts.push((title, xml_url));
//         }
//     }
//
//     podcasts
// }
pub fn parse_opml(opml_content: &str) -> Vec<(String, String)> {
    let parser = DomParser::new().unwrap();
    let doc = parser.parse_from_string(opml_content, SupportedType::TextXml)
        .unwrap()
        .dyn_into::<web_sys::Document>()
        .unwrap();

    let mut podcasts = Vec::new();
    let outlines = doc.query_selector_all("outline").unwrap();
    for i in 0..outlines.length() {
        if let Some(outline) = outlines.item(i).and_then(|o| o.dyn_into::<web_sys::Element>().ok()) {
            let title = outline.get_attribute("title").unwrap_or_default();
            let xml_url = outline.get_attribute("xmlUrl").unwrap_or_default();
            podcasts.push((title, xml_url));
        }
    }
    podcasts
}