use super::app_drawer::App_drawer;
use super::gen_components::{empty_message, on_shownotes_click, Search_nav, UseScrollToTop};
use crate::components::audio::on_play_pause;
use crate::components::audio::AudioPlayer;
use crate::components::context::{AppState, UIState};
use crate::components::episode_list_item::EpisodeListItem;
use crate::components::episodes_layout::AppStateMsg;
use crate::components::gen_funcs::{
    format_datetime, match_date_format, parse_date, sanitize_html_with_blank_target,
};
use crate::requests::pod_req::QueuedEpisodesResponse;
use crate::requests::pod_req::{self, Episode};
use gloo_events::EventListener;
use gloo_utils::document;
use i18nrs::yew::use_translation;
use wasm_bindgen::JsCast;
use web_sys::Element;
use web_sys::{window, DragEvent, HtmlElement, TouchEvent};
use yew::prelude::*;
use yew::{function_component, html, Html, UseStateHandle};
use yew_router::history::BrowserHistory;
use yewdux::prelude::*;

// Add this at the top of your file
#[allow(dead_code)]
const SCROLL_THRESHOLD: f64 = 150.0; // Increased threshold for easier activation
#[allow(dead_code)]
const SCROLL_SPEED: f64 = 15.0; // Increased speed

// Helper function to calculate responsive item height including all spacing
#[allow(dead_code)]
fn calculate_item_height(window_width: f64) -> f64 {
    // Try to measure actual height from DOM first
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        if let Ok(Some(first_item)) = document.query_selector(".item-container") {
            if let Some(element) = first_item.dyn_ref::<web_sys::HtmlElement>() {
                let rect = element.get_bounding_client_rect();
                let actual_height = rect.height();
                let margin_bottom = 16.0; // mb-4 = 1rem = 16px
                let total_height = actual_height + margin_bottom;

                web_sys::console::log_1(
                    &format!(
                        "MEASURED: width={}, container_height={}, margin={}, total={}",
                        window_width, actual_height, margin_bottom, total_height
                    )
                    .into(),
                );

                return total_height;
            }
        }
    }

    // Fallback to estimated heights if measurement fails
    if window_width <= 530.0 {
        122.0 + 16.0 // Mobile: base height + mb-4
    } else if window_width <= 768.0 {
        150.0 + 16.0 // Tablet: base height + mb-4
    } else {
        221.0 + 16.0 // Desktop: base height + mb-4
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct ScrollState {
    interval_id: Option<i32>,
}

#[allow(dead_code)]
fn stop_auto_scroll(interval_id: i32) {
    if let Some(window) = window() {
        window.clear_interval_with_handle(interval_id);
    }
}

#[function_component(Queue)]
pub fn queue() -> Html {
    let (i18n, _) = use_translation();
    let (state, dispatch) = use_store::<AppState>();

    let error = use_state(|| None);
    let (post_state, _post_dispatch) = use_store::<AppState>();
    let (audio_state, _audio_dispatch) = use_store::<UIState>();

    let loading = use_state(|| true);

    // Fetch episodes on component mount
    let loading_ep = loading.clone();
    {
        // let episodes = episodes.clone();
        let error = error.clone();
        let api_key = post_state
            .auth_details
            .as_ref()
            .map(|ud| ud.api_key.clone());
        let user_id = post_state.user_details.as_ref().map(|ud| ud.UserID.clone());
        let server_name = post_state
            .auth_details
            .as_ref()
            .map(|ud| ud.server_name.clone());
        let effect_dispatch = dispatch.clone();

        // fetch_episodes(api_key.flatten(), user_id, server_name, dispatch, error, pod_req::call_get_recent_eps);

        use_effect_with(
            (api_key.clone(), user_id.clone(), server_name.clone()),
            move |_| {
                let error_clone = error.clone();
                if let (Some(api_key), Some(user_id), Some(server_name)) =
                    (api_key.clone(), user_id.clone(), server_name.clone())
                {
                    let dispatch = effect_dispatch.clone();

                    wasm_bindgen_futures::spawn_local(async move {
                        match pod_req::call_get_queued_episodes(&server_name, &api_key, &user_id)
                            .await
                        {
                            Ok(fetched_episodes) => {
                                let completed_episode_ids: Vec<i32> = fetched_episodes
                                    .iter()
                                    .filter(|ep| ep.completed)
                                    .map(|ep| ep.episodeid)
                                    .collect();

                                dispatch.reduce_mut(move |state| {
                                    state.queued_episodes = Some(QueuedEpisodesResponse {
                                        episodes: fetched_episodes,
                                    });
                                    state.completed_episodes = Some(completed_episode_ids);
                                });

                                // Fetch local episode IDs for Tauri mode
                                #[cfg(not(feature = "server_build"))]
                                {
                                    let dispatch_local = dispatch.clone();
                                    wasm_bindgen_futures::spawn_local(async move {
                                        if let Ok(local_episodes) = crate::components::downloads_tauri::fetch_local_episodes().await {
                                            let local_episode_ids: Vec<i32> = local_episodes
                                                .iter()
                                                .map(|ep| ep.episodeid)
                                                .collect();
                                            dispatch_local.reduce_mut(move |state| {
                                                state.locally_downloaded_episodes = Some(local_episode_ids);
                                            });
                                        }
                                    });
                                }

                                loading_ep.set(false);
                                // web_sys::console::log_1(&format!("State after update: {:?}", state).into()); // Log state after update
                            }
                            Err(e) => {
                                error_clone.set(Some(e.to_string()));
                                loading_ep.set(false);
                            }
                        }
                    });
                }
                || ()
            },
        );
    }

    html! {
        <>
        <div class="main-container">
            <Search_nav />
            <UseScrollToTop />
                if *loading { // If loading is true, display the loading animation
                    {
                        html! {
                            <div class="loading-animation">
                                <div class="frame1"></div>
                                <div class="frame2"></div>
                                <div class="frame3"></div>
                                <div class="frame4"></div>
                                <div class="frame5"></div>
                                <div class="frame6"></div>
                            </div>
                        }
                    }
                } else {
                    {
                        html! {
                            // Modern mobile-friendly queue page with tab-style page title
                            <div class="mb-2">
                                // Tab-style page indicator
                                <div class="page-tab-indicator">
                                    <i class="ph ph-queue tab-icon"></i>
                                    {&i18n.t("queue.queue")}
                                </div>
                            </div>
                        }
                    }

                    {
                        if let Some(queued_eps) = state.queued_episodes.clone() {
                            if queued_eps.episodes.is_empty() {
                                // Render "No Queued Episodes Found" if episodes list is empty
                                empty_message(
                                    &i18n.t("queue.no_queued_episodes_found"),
                                    &i18n.t("queue.queue_episodes_instructions")
                                )
                            } else {
                                html! {
                                    <VirtualQueueList
                                        episodes={queued_eps.episodes.clone()}
                                    />
                                }
                            }
                        } else {
                            empty_message(
                                &i18n.t("queue.no_queued_episodes_found_state_none"),
                                &i18n.t("queue.queue_episodes_instructions")
                            )
                        }
                }
            }
        {
            if let Some(audio_props) = &audio_state.currently_playing {
                html! { <AudioPlayer src={audio_props.src.clone()} title={audio_props.title.clone()} description={audio_props.description.clone()} release_date={audio_props.release_date.clone()} artwork_url={audio_props.artwork_url.clone()} duration={audio_props.duration.clone()} episode_id={audio_props.episode_id.clone()} duration_sec={audio_props.duration_sec.clone()} start_pos_sec={audio_props.start_pos_sec.clone()} end_pos_sec={audio_props.end_pos_sec.clone()} offline={audio_props.offline.clone()} is_youtube={audio_props.is_youtube.clone()} /> }
            } else {
                html! {}
            }
        }
        </div>
        <App_drawer />
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct VirtualQueueListProps {
    pub episodes: Vec<Episode>,
}

#[function_component(VirtualQueueList)]
pub fn virtual_queue_list(props: &VirtualQueueListProps) -> Html {
    let scroll_pos = use_state(|| 0.0);
    let container_ref = use_node_ref();
    let container_height = use_state(|| 0.0);
    let item_height = use_state(|| 234.0); // Default item height
    let force_update = use_state(|| 0);

    // Shared drag state for all episodes
    let dragging = use_state(|| None::<i32>);

    // Effect to set initial container height, item height, and listen for window resize
    {
        let container_height = container_height.clone();
        let item_height = item_height.clone();
        let force_update = force_update.clone();

        use_effect_with((), move |_| {
            let window = window().expect("no global `window` exists");
            let window_clone = window.clone();

            let update_sizes = Callback::from(move |_| {
                let height = window_clone.inner_height().unwrap().as_f64().unwrap();
                container_height.set(height - 100.0);

                let width = window_clone.inner_width().unwrap().as_f64().unwrap();
                let new_item_height = calculate_item_height(width);

                web_sys::console::log_1(
                    &format!(
                        "Virtual list: width={}, item_height={}",
                        width, new_item_height
                    )
                    .into(),
                );
                item_height.set(new_item_height);
                force_update.set(*force_update + 1);
            });

            update_sizes.emit(());

            let listener = EventListener::new(&window, "resize", move |_| {
                update_sizes.emit(());
            });

            move || drop(listener)
        });
    }

    // Effect for scroll handling - prevent feedback loop with debouncing
    {
        let scroll_pos = scroll_pos.clone();
        let container_ref = container_ref.clone();
        use_effect_with(container_ref.clone(), move |container_ref| {
            if let Some(container) = container_ref.cast::<HtmlElement>() {
                let scroll_pos_clone = scroll_pos.clone();
                let is_updating = std::rc::Rc::new(std::cell::RefCell::new(false));

                let scroll_listener = EventListener::new(&container, "scroll", move |event| {
                    // Prevent re-entrant calls that cause feedback loops
                    if *is_updating.borrow() {
                        return;
                    }

                    if let Some(target) = event.target() {
                        if let Ok(element) = target.dyn_into::<Element>() {
                            let new_scroll_top = element.scroll_top() as f64;
                            let old_scroll_top = *scroll_pos_clone;

                            // Only update if there's a significant change
                            if (new_scroll_top - old_scroll_top).abs() >= 5.0 {
                                *is_updating.borrow_mut() = true;

                                // Use requestAnimationFrame to batch updates and prevent feedback
                                let scroll_pos_clone2 = scroll_pos_clone.clone();
                                let is_updating_clone = is_updating.clone();
                                let callback =
                                    wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                                        scroll_pos_clone2.set(new_scroll_top);
                                        *is_updating_clone.borrow_mut() = false;
                                    })
                                        as Box<dyn FnMut()>);

                                web_sys::window()
                                    .unwrap()
                                    .request_animation_frame(callback.as_ref().unchecked_ref())
                                    .unwrap();
                                callback.forget();
                            }
                        }
                    }
                });

                Box::new(move || {
                    drop(scroll_listener);
                }) as Box<dyn FnOnce()>
            } else {
                Box::new(|| {}) as Box<dyn FnOnce()>
            }
        });
    }

    let start_index = (*scroll_pos / *item_height).floor() as usize;
    let visible_count = ((*container_height / *item_height).ceil() as usize) + 1;
    let end_index = (start_index + visible_count).min(props.episodes.len());

    // Debug logging to see what's happening
    web_sys::console::log_1(&format!(
        "Virtual list debug: scroll_pos={}, item_height={}, container_height={}, start_index={}, visible_count={}, end_index={}, total_episodes={}",
        *scroll_pos, *item_height, *container_height, start_index, visible_count, end_index, props.episodes.len()
    ).into());

    let visible_episodes = (start_index..end_index)
        .map(|index| {
            let episode = props.episodes[index].clone();
            html! {
                <QueueEpisode
                    key={format!("{}-{}", episode.episodeid, *force_update)}
                    episode={episode.clone()}
                    all_episodes={props.episodes.clone()}
                    dragging={dragging.clone()}
                />
            }
        })
        .collect::<Html>();

    let total_height = props.episodes.len() as f64 * *item_height;
    let offset_y = start_index as f64 * *item_height;

    // Debug the offset calculation specifically
    web_sys::console::log_1(
        &format!(
            "Offset debug: total_height={}, offset_y={}, start_index={}",
            total_height, offset_y, start_index
        )
        .into(),
    );

    html! {
        <div
            ref={container_ref}
            class="virtual-list-container flex-grow overflow-y-auto"
            style="height: calc(100vh - 100px); -webkit-overflow-scrolling: touch; overscroll-behavior-y: contain;"
        >
            // Top spacer to push content down without using transforms
            <div style={format!("height: {}px; flex-shrink: 0;", offset_y)}></div>

            // Visible episodes
            <div>
                { visible_episodes }
            </div>

            // Bottom spacer to maintain total height
            <div style={format!("height: {}px; flex-shrink: 0;", total_height - offset_y - (end_index - start_index) as f64 * *item_height)}></div>
        </div>
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct QueueEpisodeProps {
    pub episode: Episode,
    pub all_episodes: Vec<Episode>,
    pub dragging: UseStateHandle<Option<i32>>,
}

#[function_component(QueueEpisode)]
pub fn queue_episode(props: &QueueEpisodeProps) -> Html {
    html! { // FIX: drag/drop functionality
        <EpisodeListItem
            episode={ props.episode.clone() }
            page_type={ "queue" }
            on_checkbox_change={ Callback::noop() }
            is_delete_mode={ false }
        />
    }
}
