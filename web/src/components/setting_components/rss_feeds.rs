use crate::components::context::AppState;
use crate::components::gen_funcs::format_error_message;
use crate::requests::setting_reqs::{call_rss_feed_status, call_toggle_rss_feeds, call_get_rss_key};
use std::borrow::Borrow;
use web_sys::console;
use yew::platform::spawn_local;
use yew::prelude::*;
use yewdux::prelude::*;

#[function_component(RSSFeedSettings)]
pub fn rss_feed_settings() -> Html {
    let (state, _dispatch) = use_store::<AppState>();
    let api_key = state.auth_details.as_ref().map(|ud| ud.api_key.clone());
    let user_id = state.user_details.as_ref().map(|ud| ud.UserID.clone());
    let server_name = state.auth_details.as_ref().map(|ud| ud.server_name.clone());
    let rss_feed_status = use_state(|| false);
    let loading = use_state(|| false);
    let rss_feed_url = use_state(|| String::new());

    // Effect to get initial RSS feed status
    {
        let rss_feed_status = rss_feed_status.clone();
        use_effect_with(
            (api_key.clone(), server_name.clone()),
            move |(api_key, server_name)| {
                let rss_feed_status = rss_feed_status.clone();
                let api_key = api_key.clone();
                let server_name = server_name.clone();
                let future = async move {
                    if let (Some(api_key), Some(server_name)) = (api_key, server_name) {
                        let response = call_rss_feed_status(server_name, api_key.unwrap()).await;
                        match response {
                            Ok(status) => {
                                rss_feed_status.set(status);
                            }
                            Err(e) => console::log_1(
                                &format!("Error getting RSS feed status: {}", e).into(),
                            ),
                        }
                    }
                };
                spawn_local(future);
                || {}
            },
        );
    }

    let html_rss_status = rss_feed_status.clone();

    // Effect to fetch RSS key and generate URL when RSS feeds are enabled
    {
        let rss_feed_url = rss_feed_url.clone();
        use_effect_with(
            (api_key.clone(), server_name.clone(), user_id.clone(), *rss_feed_status),
            move |(api_key, server_name, user_id, rss_enabled)| {
                let rss_feed_url = rss_feed_url.clone();
                let api_key = api_key.clone();
                let server_name = server_name.clone();
                let user_id = user_id.clone();
                let rss_enabled = *rss_enabled;
                spawn_local(async move {
                    if rss_enabled {
                        if let (Some(api_key), Some(server_name), Some(user_id)) = 
                            (api_key, server_name, user_id) {
                            match call_get_rss_key(server_name.clone(), api_key.unwrap(), user_id).await {
                                Ok(rss_key) => {
                                    let url = format!("{}/rss/{}?api_key={}", server_name, user_id, rss_key);
                                    rss_feed_url.set(url);
                                }
                                Err(e) => {
                                    console::log_1(&format!("Error getting RSS key: {}", e).into());
                                    rss_feed_url.set(String::new());
                                }
                            }
                        }
                    } else {
                        rss_feed_url.set(String::new());
                    }
                });
                || {}
            },
        );
    }

    html! {
        <div class="p-4">
            <p class="item_container-text text-lg font-bold mb-4">{"RSS Feed Settings:"}</p>
            <p class="item_container-text text-md mb-4">{"Enable RSS feeds to access your podcasts from any podcast app. When enabled, you can use the URL below to subscribe to your podcasts in your favorite podcast app. The URL includes your API key, so keep it private."}</p>

            <label class="relative inline-flex items-center cursor-pointer mb-4">
                <input
                    type="checkbox"
                    disabled={**loading.borrow()}
                    checked={**rss_feed_status.borrow()}
                    class="sr-only peer"
                    onclick={Callback::from(move |_| {
                        let api_key = api_key.clone();
                        let server_name = server_name.clone();
                        let rss_feed_status = html_rss_status.clone();
                        let _dispatch = _dispatch.clone();
                        let loading = loading.clone();
                        let future = async move {
                            loading.set(true);
                            if let (Some(api_key), Some(server_name)) = (api_key, server_name) {
                                let response = call_toggle_rss_feeds(server_name, api_key.unwrap()).await;
                                match response {
                                    Ok(toggle_response) => {
                                        rss_feed_status.set(toggle_response.enabled);
                                    },
                                    Err(e) => {
                                        let formatted_error = format_error_message(&e.to_string());
                                        _dispatch.reduce_mut(|audio_state|
                                            audio_state.error_message = Some(format!("Error toggling RSS feeds: {}", formatted_error))
                                        );
                                    },
                                }
                            }
                            loading.set(false);
                        };
                        spawn_local(future);
                    })}
                />
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                <span class="ms-3 text-sm font-medium item_container-text">{"Enable RSS Feeds"}</span>
            </label>

            if *rss_feed_status {
                <div class="mt-4">
                    <p class="item_container-text font-semibold mb-2">{"Your RSS Feed URL:"}</p>
                    <div class="relative">
                        <input
                            type="text"
                            value={(*rss_feed_url).clone()}
                            readonly=true
                            class="w-full p-2 pr-20 border rounded bg-gray-100 dark:bg-gray-700 text-sm item_container-text"
                        />
                        <button
                            onclick={{
                                let rss_feed_url = rss_feed_url.clone();
                                Callback::from(move |_| {
                                    if let Some(window) = web_sys::window() {
                                        let clipboard = window.navigator().clipboard();
                                        let _ = clipboard.write_text(&(*rss_feed_url));
                                    }
                                })
                            }}
                            class="absolute right-2 top-1/2 transform -translate-y-1/2 px-4 py-1 text-sm text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                        >
                            {"Copy"}
                        </button>
                    </div>
                </div>
            }
        </div>
    }
}
