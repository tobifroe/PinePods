use serde::Deserialize;
use yew::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, Request, RequestInit, RequestMode, Response, HtmlInputElement, console};
use crate::requests::setting_reqs::{NextcloudAuthRequest, call_add_nextcloud_server, call_check_nextcloud_server};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use std::time::Duration;
use std::collections::HashMap;
use url::form_urlencoded;
use yewdux::use_store;
use crate::components::context::AppState;
use serde_wasm_bindgen;
use serde::Serialize;
// use wasm_timer;


// Assume this struct is for handling the response of the initial login request
#[derive(Serialize, Deserialize)]
pub struct NextcloudLoginResponse {
    pub poll: Poll,
    pub login: String,
}

#[derive(Serialize, Deserialize)]
pub struct Poll {
    pub token: String,
    pub endpoint: String,
}

async fn initiate_nextcloud_login(server_url: &str) -> Result<NextcloudLoginResponse, anyhow::Error> {
    console::log_1(&"Initiating Nextcloud login...".into());
    let login_endpoint = format!("{}/index.php/login/v2", server_url);
    let window = web_sys::window().expect("no global `window` exists");
    let request = Request::new_with_str_and_init(&login_endpoint, RequestInit::new().method("POST").mode(RequestMode::Cors))
        .expect("Failed to build request.");

    match JsFuture::from(window.fetch_with_request(&request)).await {
        Ok(js_value) => {
            console::log_1(&"Received response from server...".into());
            let response: Response = js_value.dyn_into().unwrap();
            if response.status() == 200 {
                console::log_1(&"Response status is 200...".into());
                match JsFuture::from(response.json().unwrap()).await {
                    Ok(json_result) => {
                        console::log_1(&format!("JSON response: {:?}", json_result).into());
                        console::log_1(&"Successfully parsed JSON response...".into());
                        console::log_1(&"Before login response".into());
                        match serde_wasm_bindgen::from_value::<NextcloudLoginResponse>(json_result) {
                            Ok(login_data) => {
                                console::log_1(&format!("Login URL: {}", &login_data.login.clone()).into());
                                window.open_with_url(&login_data.login).expect("Failed to open login URL");
                                Ok(login_data)
                            },
                            Err(_) => {
                                console::log_1(&"Failed to deserialize JSON response...".into());
                                Err(anyhow::Error::msg("Failed to deserialize JSON response"))
                            },
                        }
                    },
                    Err(_) => {
                        console::log_1(&"Failed to parse JSON response...".into());
                        Err(anyhow::Error::msg("Failed to parse JSON response"))
                    },
                }
            } else {
                console::log_1(&format!("Failed to initiate Nextcloud login, status: {}", response.status()).into());
                Err(anyhow::Error::msg(format!("Failed to initiate Nextcloud login, status: {}", response.status())))
            }
        },
        Err(_) => {
            console::log_1(&"Failed to send authentication request...".into());
            Err(anyhow::Error::msg("Failed to send authentication request."))
        },
    }
}

#[function_component(NextcloudOptions)]
pub fn nextcloud_options() -> Html {
    let (state, dispatch) = use_store::<AppState>();
    let effect_dispatch = dispatch.clone();
    let api_key = state.auth_details.as_ref().map(|ud| ud.api_key.clone());
    let user_id = state.user_details.as_ref().map(|ud| ud.UserID.clone());
    let server_name = state.auth_details.as_ref().map(|ud| ud.server_name.clone());
    let server_url = use_state(|| String::new());
    let auth_status = use_state(|| String::new());

    // Handler for server URL input change
    let on_server_url_change = {
        let server_url = server_url.clone();
        Callback::from(move |e: InputEvent| {
            // Cast the event target to HtmlInputElement to access the value
            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                server_url.set(input.value());
            }
        })
    };

    // Handler for initiating authentication
    let on_authenticate_click = {
        let server_url = server_url.clone();
        let server_url_initiate = server_url.clone();
        let server_name = server_name.clone();
        let api_key = api_key.clone();
        let user_id = user_id.clone();
        let auth_status = auth_status.clone();
        Callback::from(move |_| {
            console::log_1(&"Authenticate button clicked.".into());
            let auth_status = auth_status.clone();
            let server = (*server_url_initiate).clone();
            let server_name = server_name.clone();
            let api_key = api_key.clone();
            let user_id = user_id.clone();


            if !server.is_empty() {
                wasm_bindgen_futures::spawn_local(async move {
                    match initiate_nextcloud_login(&server).await {
                        Ok(login_data) => {
                            // Use login_data.poll_endpoint and login_data.token for the next steps
                            let auth_request = NextcloudAuthRequest {
                                user_id: user_id.clone().unwrap(),
                                token: login_data.poll.token,
                                poll_endpoint: login_data.poll.endpoint,
                                nextcloud_url: server.clone(),
                            };
                            match call_add_nextcloud_server(&server_name.clone().unwrap(), &api_key.clone().unwrap().unwrap(), auth_request).await {
                                Ok(_) => {
                                    log::info!("pinepods server now polling nextcloud");
                                    // Start polling the check_gpodder_settings endpoint
                                    loop {
                                        console::log_1(&"Checking gPodder settings...".into());
                                        match call_check_nextcloud_server(&server_name.clone().unwrap(), &api_key.clone().unwrap().unwrap(), user_id.clone().unwrap()).await {
                                            Ok(response) => {
                                                if response.data {
                                                    log::info!("gPodder settings have been set up");
                                                    break;
                                                } else {
                                                    log::info!("gPodder settings are not yet set up, continuing to poll...");
                                                }
                                            },
                                            Err(e) => log::error!("Error calling check_gpodder_settings: {:?}", e),
                                        }

                                        // // Wait for a short period before polling again
                                        let delay = std::time::Duration::from_secs(5);
                                        async_std::task::sleep(delay).await;
                                        // let _ = wasm_timer::Delay::new(delay).await;
                                    }
                            },
                                Err(e) => log::error!("Error calling add_nextcloud_server: {:?}", e),
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to initiate Nextcloud login: {:?}", e);
                            auth_status.set("Failed to initiate Nextcloud login. Please check the server URL.".to_string());
                        }
                    }
                });
            } else {
                auth_status.set("Please enter a Nextcloud server URL.".to_string());
            }
        })
    };


    html! {
        <div class="p-4"> // You can adjust the padding as needed
            <p class="text-lg font-bold mb-4">{"Nextcloud Podcast Sync:"}</p> // Styled paragraph
            <p class="text-md mb-4">{"With this option you can authenticate with a Nextcloud server to use as a podcast sync client. This option works great with AntennaPod on Android so you can have the same exact feed there while on mobile. In addition, if you're already using AntennaPod with Nextcloud Podcast sync you can connect your existing sync feed to quickly import everything right into Pinepods! Clicking the Authenticate Button will prompt you to externally import your Nextcloud Server."}</p> // Styled paragraph
            
            <input type="text" class="input" placeholder="Enter Nextcloud server URL" value={(*server_url).clone()} oninput={on_server_url_change} />
            <button onclick={on_authenticate_click} class="mt-4 bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline" type="button">
                {"Authenticate Nextcloud Server"}
            </button>
        </div>
    }
}
