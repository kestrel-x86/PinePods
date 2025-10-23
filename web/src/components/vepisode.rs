use crate::components::context::{AppState, UIState};
#[cfg(not(feature = "server_build"))]
use crate::components::downloads_tauri::{
    download_file, remove_episode_from_local_db, update_local_database, update_podcast_database,
};
use crate::components::gen_components::{ContextButton, EpisodeModal, FallbackImage};
use crate::components::gen_funcs::format_error_message;
use crate::components::gen_funcs::{format_time, strip_images_from_html};
use crate::components::notification_center::{NotificationCenter, ToastNotification};
use crate::components::safehtml::SafeHtml;
use crate::requests::people_req::PersonEpisode;
use crate::requests::pod_req::Episode as SearchNewEpisode;
use crate::requests::pod_req::{
    call_download_episode, call_mark_episode_completed, call_mark_episode_uncompleted,
    call_queue_episode, call_remove_downloaded_episode, call_remove_queued_episode,
    call_remove_saved_episode, call_save_episode, DownloadEpisodeRequest, Episode, EpisodeDownload,
    HistoryEpisode, HomeEpisode, MarkEpisodeCompletedRequest, QueuePodcastRequest, QueuedEpisode,
    SavePodcastRequest, SavedEpisode,
};
#[cfg(not(feature = "server_build"))]
use crate::requests::pod_req::{
    call_get_episode_metadata, call_get_podcast_details, EpisodeRequest,
};
use crate::requests::search_pods::SearchEpisode;
use crate::requests::search_pods::{
    call_get_podcast_info, call_youtube_search, test_connection, PeopleEpisode,
    YouTubeSearchResults,
};
use gloo_events::EventListener;
use gloo_timers::callback::Timeout;
use i18nrs::yew::use_translation;
use std::any::Any;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;
use web_sys::{window, Element, HtmlInputElement, MouseEvent};
use yew::prelude::*;
use yew::Callback;
use yew_router::history::{BrowserHistory, History};
use yewdux::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct VEpisodeProps {
    pub episode: Episode,
    pub description: String,
    // pub _is_expanded: bool,
    pub format_release: String, // todo: format in html!{} macro?
    pub on_play_pause: Callback<MouseEvent>,
    pub on_shownotes_click: Callback<MouseEvent>,
    // pub _toggle_expanded: Callback<MouseEvent>,
    pub page_type: String,
    pub on_checkbox_change: Callback<i32>,
    pub is_delete_mode: bool,
    pub completed: bool,
    pub show_modal: bool,
    pub on_modal_open: Callback<i32>,
    pub on_modal_close: Callback<MouseEvent>,
    pub container_height: String,
    pub is_current_episode: bool,
    pub is_playing: bool,
    pub on_touch_start: Callback<TouchEvent>,
    pub on_touch_end: Callback<TouchEvent>,
    pub on_touch_move: Callback<TouchEvent>,
    pub show_context_menu: bool,
    pub context_menu_position: (i32, i32),
    pub close_context_menu: Callback<()>,
    pub context_button_ref: NodeRef,
    pub is_pressing: bool,
}

/*
Has no ctx menu. Tap opens modal.
THIS IS THE ONE
*/
#[function_component(VEpisode)]
pub fn virtual_episode_item(props: &VEpisodeProps) -> Html {
    let (state, _) = use_store::<AppState>();

    let span_episode = props.episode.episodeduration.clone();
    let formatted_duration = format_time(span_episode as f64);
    let duration_clone = formatted_duration.clone();
    let duration_again = formatted_duration.clone();

    // Calculate the percentage of the episode that has been listened to
    let listen_duration_percentage = if props.episode.episodeduration > 0 {
        ((props.episode.listenduration.clone().unwrap_or_default() as f64
            / props.episode.episodeduration as f64)
            * 100.0)
            .min(100.0)
    } else {
        0.0
    };

    // Check if viewport is narrow (< 500px)
    let is_narrow_viewport = {
        let window = web_sys::window().expect("no global window exists");
        window.inner_width().unwrap().as_f64().unwrap() < 500.0
    };

    let checkbox_ep = props.episode.get_episode_id(Some(0));
    let should_show_buttons = !props.episode.episodeurl.is_empty();
    let preview_description = strip_images_from_html(&props.description);

    // Handle context menu position
    // let context_menu_style = if props.show_context_menu {
    //     format!(
    //         "position: fixed; top: {}px; left: {}px; z-index: 1000;",
    //         props.context_menu_position.1, props.context_menu_position.0
    //     )
    // } else {
    //     String::new()
    // };

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window)]
        fn toggleDescription(guid: &str, expanded: bool);
    }

    html! {
        <div>
            <div
                class={classes!(
                    "item-container", "border-solid", "border", "flex", "items-start", "mb-4",
                    "shadow-md", "rounded-lg", "touch-manipulation", "transition-all", "duration-150",
                    if props.is_pressing {
                        "bg-accent-color bg-opacity-20 transform scale-[0.98]"
                    } else {
                        ""
                    }
                )}
                style={format!("height: {}; overflow: hidden; user-select: {};",
                    props.container_height,
                    if props.is_pressing { "none" } else { "auto" }
                )}
                ontouchstart={props.on_touch_start.clone() }
                ontouchend={ props.on_touch_end.clone() }
                ontouchmove={ props.on_touch_move .clone()}
            >

                {if props.is_delete_mode {
                    html! {
                        <div class="flex items-center pl-4">
                            <input
                                type="checkbox"
                                checked={state.selected_episodes_for_deletion.contains(&props.episode.get_episode_id(Some(0)))}
                                class="podcast-dropdown-checkbox h-5 w-5 rounded border-2 text-primary focus:ring-primary focus:ring-offset-0 cursor-pointer appearance-none checked:bg-primary checked:border-primary"
                                onchange={props.on_checkbox_change.reform(move |_| checkbox_ep)}
                            />
                        </div>
                    }
                } else {
                    html! {}
                }}

                <div class="flex flex-col w-auto object-cover pl-4">
                    <FallbackImage
                        src={props.episode.get_episode_artwork()}
                        alt={format!("Cover for {}", props.episode.get_episode_title())}
                        class="episode-image"
                    />
                </div>
                <div class="flex flex-col p-4 space-y-2 flex-grow md:w-7/12 self-start">
                    <div class="flex items-center space-x-2 cursor-pointer" onclick={props.on_shownotes_click.clone()}>
                    <p class="item_container-text episode-title font-semibold line-clamp-2">
                        {props.episode.get_episode_title()}
                    </p>
                    {
                        if props.completed.clone() {
                            html! {
                                <i class="ph ph-check-circle text-2xl text-green-500"></i>
                            }
                        } else {
                            html! {}
                        }
                    }
                    </div>
                    <hr class="my-2 border-t hidden md:block"/>
                    {
                        html! {
                            <div class="item-description-text cursor-pointer hidden md:block"
                                onclick={let episode_id = props.episode.get_episode_id(None);
                                        let omo = props.on_modal_open.clone();
                                        Callback::from(move |e: MouseEvent| {
                                            e.prevent_default();
                                            omo.emit(episode_id);
                                        })}>
                                <div class="item_container-text line-clamp-2">
                                    <SafeHtml html={preview_description} />
                                </div>
                            </div>
                        }
                    }

                    <div class="episode-time-badge-container" style="max-width: 100%; overflow: hidden;">
                        <span
                            class="episode-time-badge inline-flex items-center px-2.5 py-0.5 rounded me-2"
                            style="flex-grow: 0; flex-shrink: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;"
                        >
                            <svg class="time-icon w-2.5 h-2.5 me-1.5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 20 20">
                                <path d="M10 0a10 10 0 1 0 10 10A10.011 10.011 0 0 0 10 0Zm3.982 13.982a1 1 0 0 1-1.414 0l-3.274-3.274A1.012 1.012 0 0 1 9 10V6a1 1 0 0 1 2 0v3.586l2.982 2.982a1 1 0 0 1 0 1.414Z"/>
                            </svg>
                            { props.format_release.clone() }
                        </span>
                    </div>
                    {
                        if props.completed {
                            // For completed episodes
                            if is_narrow_viewport {
                                // In narrow viewports, just show "Completed"
                                html! {
                                    <div class="flex items-center space-x-2">
                                        <span class="item_container-text">{"Completed"}</span>
                                    </div>
                                }
                            } else {
                                // In wider viewports, show duration and "Completed"
                                html! {
                                    <div class="flex items-center space-x-2">
                                        <span class="item_container-text">{ duration_clone }</span>
                                        <span class="item_container-text">{ "-  Completed" }</span>
                                    </div>
                                }
                            }
                        } else {
                            // For in-progress episodes
                            if props.episode.listenduration.clone().unwrap_or_default() > 0 {
                                html! {
                                    <div class="flex items-center space-x-2">
                                        // Only show current position in wider viewports
                                        {
                                            if !is_narrow_viewport {
                                                html! {
                                                    <span class="item_container-text">{ format_time(props.episode.listenduration.clone().unwrap_or_default() as f64) }</span>
                                                }
                                            } else {
                                                html! {}
                                            }
                                        }
                                        <div class="progress-bar-container">
                                            <div class="progress-bar" style={ format!("width: {}%;", listen_duration_percentage) }></div>
                                        </div>
                                        <span class="item_container-text">{ duration_again }</span>
                                    </div>
                                }
                            } else {
                                // For episodes with no listen progress
                                html! {
                                    <span class="item_container-text">{ format!("{}", formatted_duration) }</span>
                                }
                            }
                        }
                    }
                </div>
                {
                    html! {
                        <div class="flex flex-col items-center h-full w-2/12 px-2 space-y-4 md:space-y-8 button-container" style="align-self: center;">
                            if should_show_buttons {
                                <button
                                    class="item-container-button selector-button font-bold py-2 px-4 rounded-full flex items-center justify-center md:w-16 md:h-16 w-10 h-10"
                                    onclick={props.on_play_pause.clone()}
                                >
                                    {
                                        if props.is_current_episode && props.is_playing {
                                            html! { <i class="ph ph-pause-circle md:text-6xl text-4xl"></i> }
                                        } else {
                                            html! { <i class="ph ph-play-circle md:text-6xl text-4xl"></i> }
                                        }
                                    }
                                </button>
                                <div class="hidden sm:block"> // Standard desktop context button
                                    <div ref={props.context_button_ref.clone()}>
                                        <ContextButton episode={props.episode.clone()} page_type={props.page_type.to_string()} />
                                    </div>
                                </div>
                            }
                        </div>
                    }
                }
            </div>

            // This shows the context menu via long press
            {
                if props.show_context_menu {
                    html! {
                        <ContextButton
                            episode={props.episode.clone()}
                            page_type={props.page_type.to_string()}
                            show_menu_only={true}
                            position={Some(props.context_menu_position)}
                            on_close={props.close_context_menu.clone()}
                        />
                    }
                } else {
                    html! {}
                }
            }

            if props.show_modal {
                <EpisodeModal
                    episode_id={props.episode.get_episode_id(None)}
                    episode_url={props.episode.episodeurl.clone()}
                    episode_artwork={props.episode.get_episode_artwork()}
                    episode_title={props.episode.get_episode_title()}
                    description={props.description.clone()}
                    format_release={props.format_release.to_string()}
                    duration={props.episode.episodeduration}
                    on_close={props.on_modal_close.clone()}
                    on_show_notes={props.on_shownotes_click.clone()}
                    listen_duration_percentage={listen_duration_percentage}
                    is_youtube={props.episode.get_is_youtube()}
                />
            }
        </div>
    }
}
