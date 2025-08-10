// notifications.rs
use crate::components::context::AppState;
use crate::components::gen_funcs::format_error_message;
use crate::requests::setting_reqs::{
    call_get_notification_settings, call_test_notification, call_update_notification_settings,
    NotificationSettings, NotificationSettingsResponse,
};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yewdux::prelude::*;

#[function_component(NotificationOptions)]
pub fn notification_settings() -> Html {
    let (state, _dispatch) = use_store::<AppState>();
    // Form states
    let platform = use_state(|| "ntfy".to_string());
    let enabled = use_state(|| false);
    let ntfy_topic = use_state(|| "".to_string());
    let ntfy_server = use_state(|| "https://ntfy.sh".to_string());
    let ntfy_username = use_state(|| "".to_string());
    let ntfy_password = use_state(|| "".to_string());
    let ntfy_access_token = use_state(|| "".to_string());
    let gotify_url = use_state(|| "".to_string());
    let gotify_token = use_state(|| "".to_string());

    // Add state for notification info
    let notification_info: UseStateHandle<Option<NotificationSettingsResponse>> =
        use_state(|| None);
    let update_trigger = use_state(|| false);

    // Success/error states
    let show_success = use_state(|| false);
    let success_message = use_state(|| "".to_string());

    let server_name = state
        .auth_details
        .as_ref()
        .map(|ud| ud.server_name.clone())
        .unwrap_or_default();
    let api_key = state
        .auth_details
        .as_ref()
        .and_then(|ud| ud.api_key.clone())
        .unwrap_or_default();
    let user_id = state
        .user_details
        .as_ref()
        .map(|ud| ud.UserID)
        .unwrap_or_default();

    // Fetch current settings on load
    // Fetch current settings on load
    {
        let server_name = state.auth_details.as_ref().map(|ud| ud.server_name.clone());
        let api_key = state.auth_details.as_ref().map(|ud| ud.api_key.clone());
        let user_id = state.user_details.as_ref().map(|ud| ud.UserID);
        let update_trigger = update_trigger.clone(); // Add this

        let platform = platform.clone();
        let enabled = enabled.clone();
        let ntfy_topic = ntfy_topic.clone();
        let ntfy_server = ntfy_server.clone();
        let ntfy_username = ntfy_username.clone();
        let ntfy_password = ntfy_password.clone();
        let ntfy_access_token = ntfy_access_token.clone();
        let gotify_url = gotify_url.clone();
        let gotify_token = gotify_token.clone();
        let _dispatch = _dispatch.clone();
        let notification_info = notification_info.clone();

        use_effect_with(
            (api_key.clone(), server_name.clone(), update_trigger.clone()), // Add update_trigger here
            move |(api_key, server_name, _)| {
                if let (Some(api_key), Some(server_name), Some(user_id)) =
                    (api_key.clone(), server_name.clone(), user_id)
                {
                    wasm_bindgen_futures::spawn_local(async move {
                        match call_get_notification_settings(
                            server_name.clone(),
                            api_key.unwrap().clone(),
                            user_id,
                        )
                        .await
                        {
                            // In the effect where settings are fetched, modify this part:
                            Ok(settings_response) => {
                                // Set the notification_info state
                                notification_info.set(Some(settings_response.clone()));

                                // Always default to ntfy platform, but populate all settings
                                let ntfy_setting = settings_response.settings.iter().find(|s| s.platform == "ntfy");
                                let gotify_setting = settings_response.settings.iter().find(|s| s.platform == "gotify");
                                
                                // Always set platform to ntfy by default - override any existing setting
                                platform.set("ntfy".to_string());
                                
                                // Set enabled state based on ntfy setting if it exists
                                if let Some(ntfy) = ntfy_setting {
                                    enabled.set(ntfy.enabled);
                                } else {
                                    enabled.set(false);
                                }

                                // Populate ntfy fields if ntfy setting exists
                                if let Some(ntfy) = ntfy_setting {
                                    if let Some(topic) = &ntfy.ntfy_topic {
                                        ntfy_topic.set(topic.clone());
                                    }
                                    if let Some(server) = &ntfy.ntfy_server_url {
                                        ntfy_server.set(server.clone());
                                    }
                                    if let Some(username) = &ntfy.ntfy_username {
                                        ntfy_username.set(username.clone());
                                    }
                                    if let Some(password) = &ntfy.ntfy_password {
                                        ntfy_password.set(password.clone());
                                    }
                                    if let Some(token) = &ntfy.ntfy_access_token {
                                        ntfy_access_token.set(token.clone());
                                    }
                                }
                                
                                // Populate gotify fields if gotify setting exists (for when user switches)
                                if let Some(gotify) = gotify_setting {
                                    if let Some(url) = &gotify.gotify_url {
                                        gotify_url.set(url.clone());
                                    }
                                    if let Some(token) = &gotify.gotify_token {
                                        gotify_token.set(token.clone());
                                    }
                                }
                            }
                            Err(e) => {
                                let formatted_error = format_error_message(&e.to_string());
                                _dispatch.reduce_mut(|state| {
                                    state.error_message = Some(format!(
                                        "Failed to fetch notification settings: {}",
                                        formatted_error
                                    ));
                                });
                            }
                        }
                    });
                }
                || ()
            },
        );
    }

    // Submit handler
    let submit_server = server_name.clone();
    let submit_api = api_key.clone();
    let submit_user = user_id.clone();
    let on_submit = {
        let platform = platform.clone();
        let enabled = enabled.clone();
        let ntfy_topic = ntfy_topic.clone();
        let ntfy_server = ntfy_server.clone();
        let ntfy_username = ntfy_username.clone();
        let ntfy_password = ntfy_password.clone();
        let ntfy_access_token = ntfy_access_token.clone();
        let gotify_url = gotify_url.clone();
        let gotify_token = gotify_token.clone();
        let show_success = show_success.clone();
        let success_message = success_message.clone();
        let _dispatch = _dispatch.clone();

        Callback::from(move |e: SubmitEvent| {
            let update_trig = update_trigger.clone();
            let success_call = show_success.clone();
            let success_call_message = success_message.clone();
            let dispatch_call = _dispatch.clone();
            let server_submit = submit_server.clone();
            let key_submit = submit_api.clone();
            let id_submit = submit_user.clone();

            e.prevent_default();

            let settings = NotificationSettings {
                platform: (*platform).clone(),
                enabled: *enabled,
                ntfy_topic: Some((*ntfy_topic).clone()),
                ntfy_server_url: Some((*ntfy_server).clone()),
                ntfy_username: Some((*ntfy_username).clone()),
                ntfy_password: Some((*ntfy_password).clone()),
                ntfy_access_token: Some((*ntfy_access_token).clone()),
                gotify_url: Some((*gotify_url).clone()),
                gotify_token: Some((*gotify_token).clone()),
            };

            wasm_bindgen_futures::spawn_local(async move {
                match call_update_notification_settings(
                    server_submit,
                    key_submit,
                    id_submit,
                    settings,
                )
                .await
                {
                    Ok(_) => {
                        success_call.set(true);
                        success_call_message
                            .set("Successfully updated notification settings".to_string());
                        update_trig.set(!*update_trig);
                    }
                    Err(e) => {
                        let formatted_error = format_error_message(&e.to_string());
                        dispatch_call.reduce_mut(|state| {
                            state.error_message = Some(format!(
                                "Failed to update notification settings: {}",
                                formatted_error
                            ));
                        });
                    }
                }
            });
        })
    };

    let on_test_notification = {
        let platform = platform.clone();
        let _dispatch = _dispatch.clone();

        Callback::from(move |_| {
            let dispatch = _dispatch.clone();
            let platform_value = (*platform).clone();
            let test_server = server_name.clone();
            let test_api = api_key.clone();
            let test_user = user_id.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match call_test_notification(test_server, test_api, test_user, platform_value).await
                {
                    Ok(_) => {
                        dispatch.reduce_mut(|state| {
                            state.info_message = Some("Test notification sent!".to_string())
                        });
                    }
                    Err(e) => {
                        // Format the error message to be more user-friendly
                        let formatted_error = format_error_message(&e.to_string());
                        dispatch.reduce_mut(|state| {
                            state.error_message = Some(format!(
                                "Failed to send test notification: {}",
                                formatted_error
                            ))
                        });
                    }
                }
            });
        })
    };

    html! {
        <div class="user-settings-container">
            <div class="settings-header">
                <div class="flex items-center gap-4">
                    <i class="ph ph-bell text-2xl"></i>
                    <h2 class="text-xl font-semibold">{"Notification Settings"}</h2>
                </div>
            </div>

            if let Some(info) = &*notification_info {
                <div class="user-info-container mt-4 p-4 border border-solid border-opacity-10 rounded-lg">
                    <div class="grid grid-cols-2 gap-6">
                        // ntfy Settings
                        <div>
                            <span class="text-sm opacity-80">{"ntfy Configuration:"}</span>
                            {
                                if let Some(ntfy) = info.settings.iter().find(|s| s.platform == "ntfy") {
                                    html! {
                                        <div class="mt-2 space-y-2">
                                            <p>
                                                <span class="text-sm opacity-80">{"Status: "}</span>
                                                <span class="font-medium">{if ntfy.enabled { "Active" } else { "Inactive" }}</span>
                                            </p>
                                            <p>
                                                <span class="text-sm opacity-80">{"Topic: "}</span>
                                                <span class="font-medium">{&ntfy.ntfy_topic.clone().unwrap_or_else(|| "Not Set".to_string())}</span>
                                            </p>
                                            <p>
                                                <span class="text-sm opacity-80">{"Server: "}</span>
                                                <span class="font-medium">{&ntfy.ntfy_server_url.clone().unwrap_or_else(|| "Not Set".to_string())}</span>
                                            </p>
                                        </div>
                                    }
                                } else {
                                    html! {
                                        <p class="mt-2 text-sm opacity-80">{"Not Configured"}</p>
                                    }
                                }
                            }
                        </div>
                        // Gotify Settings
                        <div>
                            <span class="text-sm opacity-80">{"Gotify Configuration:"}</span>
                            {
                                if let Some(gotify) = info.settings.iter().find(|s| s.platform == "gotify") {
                                    html! {
                                        <div class="mt-2 space-y-2">
                                            <p>
                                                <span class="text-sm opacity-80">{"Status: "}</span>
                                                <span class="font-medium">{if gotify.enabled { "Active" } else { "Inactive" }}</span>
                                            </p>
                                            <p>
                                                <span class="text-sm opacity-80">{"Server: "}</span>
                                                <span class="font-medium">{&gotify.gotify_url.clone().unwrap_or_else(|| "Not Set".to_string())}</span>
                                            </p>
                                        </div>
                                    }
                                } else {
                                    html! {
                                        <p class="mt-2 text-sm opacity-80">{"Not Configured"}</p>
                                    }
                                }
                            }
                        </div>
                    </div>
                </div>
            }

            <form onsubmit={on_submit} class="space-y-4">
                <div class="form-group">
                    <label class="form-label">{"Notification Platform"}</label>
                    <select
                        class="form-input"
                        value="ntfy"
                        onchange={let platform = platform.clone(); Callback::from(move |e: Event| {
                            let target: HtmlInputElement = e.target_unchecked_into();
                            platform.set(target.value());
                        })}
                    >
                        <option value="ntfy" selected=true>{"ntfy"}</option>
                        <option value="gotify">{"Gotify"}</option>
                    </select>
                </div>

                <div class="form-group">
                    <label class="form-label">{"Enable Notifications"}</label>
                    <input
                        type="checkbox"
                        checked={*enabled}
                        onchange={let enabled = enabled.clone(); Callback::from(move |e: Event| {
                            let target: HtmlInputElement = e.target_unchecked_into();
                            enabled.set(target.checked());
                        })}
                    />
                </div>

                {
                    if *platform == "ntfy" {
                        html! {
                            <>
                                <div class="form-group">
                                    <label for="ntfy_topic" class="form-label">{"ntfy Topic"}</label>
                                    <input
                                        type="text"
                                        id="ntfy_topic"
                                        value={(*ntfy_topic).clone()}
                                        oninput={let ntfy_topic = ntfy_topic.clone(); Callback::from(move |e: InputEvent| {
                                            let target: HtmlInputElement = e.target_unchecked_into();
                                            ntfy_topic.set(target.value());
                                        })}
                                        class="form-input"
                                        placeholder="Enter your ntfy topic"
                                    />
                                </div>
                                <div class="form-group">
                                    <label for="ntfy_server" class="form-label">{"ntfy Server URL"}</label>
                                    <input
                                        type="text"
                                        id="ntfy_server"
                                        value={(*ntfy_server).clone()}
                                        oninput={let ntfy_server = ntfy_server.clone(); Callback::from(move |e: InputEvent| {
                                            let target: HtmlInputElement = e.target_unchecked_into();
                                            ntfy_server.set(target.value());
                                        })}
                                        class="form-input"
                                        placeholder="Enter ntfy server URL (default: https://ntfy.sh)"
                                    />
                                </div>
                                
                                <div class="form-group">
                                    <label class="form-label">{"Authentication (Optional)"}</label>
                                    <p class="text-sm opacity-80 mb-2">{"Choose either username/password OR access token, not both"}</p>
                                </div>
                                
                                <div class="form-group">
                                    <label for="ntfy_username" class="form-label">{"Username"}</label>
                                    <input
                                        type="text"
                                        id="ntfy_username"
                                        value={(*ntfy_username).clone()}
                                        disabled={!(*ntfy_access_token).is_empty()}
                                        oninput={let ntfy_username = ntfy_username.clone(); let ntfy_access_token = ntfy_access_token.clone(); Callback::from(move |e: InputEvent| {
                                            let target: HtmlInputElement = e.target_unchecked_into();
                                            ntfy_username.set(target.value());
                                            // Clear access token if username/password is being used
                                            if !target.value().is_empty() {
                                                ntfy_access_token.set("".to_string());
                                            }
                                        })}
                                        class="form-input"
                                        placeholder="ntfy username (optional)"
                                    />
                                </div>
                                
                                <div class="form-group">
                                    <label for="ntfy_password" class="form-label">{"Password"}</label>
                                    <input
                                        type="password"
                                        id="ntfy_password"
                                        value={(*ntfy_password).clone()}
                                        disabled={!(*ntfy_access_token).is_empty()}
                                        oninput={let ntfy_password = ntfy_password.clone(); let ntfy_access_token = ntfy_access_token.clone(); Callback::from(move |e: InputEvent| {
                                            let target: HtmlInputElement = e.target_unchecked_into();
                                            ntfy_password.set(target.value());
                                            // Clear access token if username/password is being used
                                            if !target.value().is_empty() {
                                                ntfy_access_token.set("".to_string());
                                            }
                                        })}
                                        class="form-input"
                                        placeholder="ntfy password (optional)"
                                    />
                                </div>
                                
                                <div class="form-group">
                                    <label for="ntfy_access_token" class="form-label">{"Access Token"}</label>
                                    <input
                                        type="password"
                                        id="ntfy_access_token"
                                        value={(*ntfy_access_token).clone()}
                                        disabled={!(*ntfy_username).is_empty() || !(*ntfy_password).is_empty()}
                                        oninput={let ntfy_access_token = ntfy_access_token.clone(); let ntfy_username = ntfy_username.clone(); let ntfy_password = ntfy_password.clone(); Callback::from(move |e: InputEvent| {
                                            let target: HtmlInputElement = e.target_unchecked_into();
                                            ntfy_access_token.set(target.value());
                                            // Clear username/password if access token is being used
                                            if !target.value().is_empty() {
                                                ntfy_username.set("".to_string());
                                                ntfy_password.set("".to_string());
                                            }
                                        })}
                                        class="form-input"
                                        placeholder="ntfy access token (alternative to username/password)"
                                    />
                                </div>
                            </>
                        }
                    } else {
                        html! {
                            <>
                                <div class="form-group">
                                    <label for="gotify_url" class="form-label">{"Gotify Server URL"}</label>
                                    <input
                                        type="text"
                                        id="gotify_url"
                                        value={(*gotify_url).clone()}
                                        oninput={let gotify_url = gotify_url.clone(); Callback::from(move |e: InputEvent| {
                                            let target: HtmlInputElement = e.target_unchecked_into();
                                            gotify_url.set(target.value());
                                        })}
                                        class="form-input"
                                        placeholder="Enter your Gotify server URL"
                                    />
                                </div>
                                <div class="form-group">
                                    <label for="gotify_token" class="form-label">{"Gotify App Token"}</label>
                                    <input
                                        type="text"
                                        id="gotify_token"
                                        value={(*gotify_token).clone()}
                                        oninput={let gotify_token = gotify_token.clone(); Callback::from(move |e: InputEvent| {
                                            let target: HtmlInputElement = e.target_unchecked_into();
                                            gotify_token.set(target.value());
                                        })}
                                        class="form-input"
                                        placeholder="Enter your Gotify application token"
                                    />
                                </div>
                            </>
                        }
                    }
                }

                if *show_success {
                    <div class="success-message">
                        {(*success_message).clone()}
                    </div>
                }

                <button type="submit" class="submit-button">
                    <i class="ph ph-floppy-disk"></i>
                    {"Save Settings"}
                </button>
            </form>

            // Add this right after the form fields but before the submit button
            if *enabled {
                <button
                    type="button"
                    onclick={on_test_notification.clone()}
                    class="submit-button mt-4"
                >
                    <i class="ph ph-bell-ringing"></i>
                    {"Send Test Notification"}
                </button>
            }
        </div>
    }
}
