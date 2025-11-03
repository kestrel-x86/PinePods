use super::gen_components::{on_shownotes_click, ContextButton, EpisodeModal, FallbackImage};
use super::gen_funcs::{format_datetime, match_date_format, parse_date};
use crate::components::audio::on_play_pause;
use crate::components::context::{AppState, UIState};
use crate::components::gen_funcs::{format_time, strip_images_from_html};
use crate::components::gen_funcs::{sanitize_html_with_blank_target, truncate_description};
use crate::components::safehtml::SafeHtml;
use crate::components::episode_list_item::EpisodeListItem;
use crate::requests::pod_req::Episode;
use gloo::events::EventListener;
use i18nrs::yew::use_translation;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{window, Element, HtmlElement, MouseEvent};
use yew::prelude::*;
use yew::Properties;
use yew::{function_component, html, use_effect_with, use_node_ref, Callback, Html};
use yew_router::history::BrowserHistory;
use yewdux::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PodcastEpisodeVirtualListProps {
    pub episodes: Vec<Episode>,
    pub item_height: f64,
    pub podcast_added: bool,
    pub search_state: Rc<AppState>,
    pub search_ui_state: Rc<UIState>,
    pub dispatch: Dispatch<UIState>,
    pub search_dispatch: Dispatch<AppState>,
    pub history: BrowserHistory,
    pub server_name: Option<String>,
    pub user_id: Option<i32>,
    pub api_key: Option<Option<String>>,
    pub podcast_link: String,
    pub podcast_title: String,
    // Bulk selection props
    pub selected_episodes: Option<Rc<std::collections::HashSet<i32>>>,
    pub is_selecting: Option<bool>,
    pub on_episode_select: Option<Callback<(i32, bool)>>,
    pub on_select_older: Option<Callback<i32>>,
    pub on_select_newer: Option<Callback<i32>>,
}

#[function_component(PodcastEpisodeVirtualList)]
pub fn podcast_episode_virtual_list(props: &PodcastEpisodeVirtualListProps) -> Html {
    let (i18n, _) = use_translation();
    let scroll_pos = use_state(|| 0.0);
    let container_ref = use_node_ref();
    let container_height = use_state(|| 0.0);
    let item_height = use_state(|| 234.0); // Default item height
    let container_item_height = use_state(|| 221.0); // Actual container height, separate from spacing
    let force_update = use_state(|| 0);
    let selected_episode_index = use_state(|| None::<usize>);

    // Pre-capture translation strings
    let select_newer_title = i18n.t("virtual_list.select_newer_episodes");
    let select_older_title = i18n.t("virtual_list.select_older_episodes");
    let completed_text = i18n.t("virtual_list.completed");
    let cover_for_text = i18n.t("virtual_list.cover_for");

    // Effect to set initial container height, item height, and listen for window resize
    {
        let container_height = container_height.clone();
        let item_height = item_height.clone();
        let container_item_height = container_item_height.clone();
        let force_update = force_update.clone();

        use_effect_with((), move |_| {
            let window = window().expect("no global `window` exists");
            let window_clone = window.clone();

            let update_sizes = Callback::from(move |_| {
                let height = window_clone.inner_height().unwrap().as_f64().unwrap();
                container_height.set(height - 100.0);

                let width = window_clone.inner_width().unwrap().as_f64().unwrap();
                // Set both the total item height (with margin) and container height
                let (new_item_height, new_container_height) = if width <= 530.0 {
                    (122.0 + 16.0, 122.0)
                } else if width <= 768.0 {
                    (150.0 + 16.0, 150.0)
                } else {
                    (221.0 + 16.0, 221.0)
                };

                item_height.set(new_item_height);
                container_item_height.set(new_container_height);
                force_update.set(*force_update + 1);
            });

            update_sizes.emit(());

            let listener = EventListener::new(&window, "resize", move |_| {
                update_sizes.emit(());
            });

            move || drop(listener)
        });
    }

    // Effect for scroll handling
    {
        let scroll_pos = scroll_pos.clone();
        let container_ref = container_ref.clone();
        use_effect_with(container_ref.clone(), move |container_ref| {
            let container = container_ref.cast::<HtmlElement>().unwrap();
            let listener = EventListener::new(&container, "scroll", move |event| {
                let target = event.target().unwrap().unchecked_into::<Element>();
                scroll_pos.set(target.scroll_top() as f64);
            });
            move || drop(listener)
        });
    }

    let start_index = (*scroll_pos / *item_height).floor() as usize;
    let visible_count = ((*container_height / *item_height).ceil() as usize) + 1;
    let end_index = (start_index + visible_count).min(props.episodes.len());

    let on_modal_close = {
        let selected_episode_index = selected_episode_index.clone();
        Callback::from(move |_: MouseEvent| selected_episode_index.set(None))
    };

    let visible_episodes = (start_index..end_index)
        .map(|index| {
            // Replace the modal open/close callbacks with:
            let on_modal_open = {
                let selected_episode_index = selected_episode_index.clone();
                let index = index; // This is your loop index
                Callback::from(move |_: MouseEvent| selected_episode_index.set(Some(index)))
            };

            let episode = &props.episodes[index];
            let dispatch = props.dispatch.clone();
            let search_state_clone = props.search_state.clone();
            let search_ui_state_clone = props.search_ui_state.clone();

            let episode_url_clone = episode.episodeurl.clone();
            let episode_title_clone = episode.episodetitle.clone();
            let episode_description_clone = episode.episodedescription.clone();
            let episode_artwork_clone = episode.artworkurl.clone();
            let episode_duration_clone = episode.episodeduration.clone();
            web_sys::console::log_1(&format!("Virtual List - episode.is_youtube: {:?}", episode.is_youtube).into());
            let episode_duration_in_seconds = episode_duration_clone.clone();
            let episode_id_clone = episode.episodeid;

            let server_name_play = props.server_name.clone();
            let user_id_play = props.user_id;
            let api_key_play = props.api_key.clone();

            let is_expanded = search_state_clone.expanded_descriptions.contains(&episode.guid.clone());

            let sanitized_description = sanitize_html_with_blank_target(&episode.episodedescription.clone());
            let (description, _is_truncated) = if is_expanded {
                (sanitized_description, false)
            } else {
                truncate_description(sanitized_description, 300)
            };

            let date_format = match_date_format(search_state_clone.date_format.as_deref());
            let datetime = parse_date(&episode.episodepubdate.clone(), &search_state_clone.user_tz);
            let format_release = format!("{}", format_datetime(&datetime, &search_state_clone.hour_preference, date_format));

            let on_play_pause = on_play_pause(
                episode_url_clone.clone(),
                episode_title_clone.clone(),
                episode_description_clone.clone(),
                format_release.clone(),
                episode_artwork_clone.clone(),
                episode_duration_in_seconds,
                episode_id_clone.clone(),
                episode.listenduration,
                api_key_play.unwrap().unwrap(),
                user_id_play.unwrap(),
                server_name_play.unwrap(),
                dispatch.clone(),
                search_ui_state_clone.clone(),
                None, // is_local
                episode.is_youtube, // is_youtube_vid
            );

            let formatted_duration = format_time(episode_duration_in_seconds.into());
            let is_current_episode = props.search_ui_state
                .currently_playing
                .as_ref()
                .map_or(false, |current| {
                    let title_match = current.title == episode.episodetitle.clone();
                    let url_match = current.src == episode.episodeurl.clone();

                    // Add episode_id comparison
                    let id_match = current.episode_id == episode.episodeid;

                    // If it's YouTube content, prioritize ID and title match over URL
                    if episode.is_youtube {
                        id_match || title_match
                    } else {
                        // For regular podcasts, use the original logic
                        title_match && url_match
                    }
                });

            let is_playing = props.search_ui_state.audio_playing.unwrap_or(false);

            let episode_url_for_ep_item = episode_url_clone.clone();
            let should_show_buttons = !episode_url_for_ep_item.is_empty();
            let preview_description = strip_images_from_html(&description);

            // Check if viewport is narrow (< 500px)
            let is_narrow_viewport = {
                let window = web_sys::window().expect("no global window exists");
                window.inner_width().unwrap().as_f64().unwrap() < 500.0
            };

            let make_shownotes_callback = {
                let history = props.history.clone();
                let search_dispatch = props.search_dispatch.clone();
                let podcast_link = props.podcast_link.clone();
                let podcast_title = props.podcast_title.clone();
                let episode_id = episode.episodeid;
                let episode_url = episode.episodeurl.clone();
                let is_youtube = episode.is_youtube.clone();

                Callback::from(move |_: MouseEvent| {
                    on_shownotes_click(
                        history.clone(),
                        search_dispatch.clone(),
                        Some(episode_id),
                        Some(podcast_link.clone()),
                        Some(episode_url.clone()),
                        Some(podcast_title.clone()),
                        true,
                        None,
                        is_youtube.clone()
                    ).emit(MouseEvent::new("click").unwrap());
                })
            };

            html! {
                <>
                <div
                    key={format!("{}-{}", episode.episodeid, *force_update)}
                    class="item-container border-solid border flex items-start mb-4 shadow-md rounded-lg"
                    style={format!("height: {}px; overflow: hidden;", *container_item_height)}
                >
                    {
                        // Show checkbox when in selection mode
                        if props.is_selecting.unwrap_or(false) {
                            let episode_id = episode.episodeid;
                            let is_selected = props.selected_episodes.as_ref().map_or(false, |selected| selected.contains(&episode_id));
                            let on_select = props.on_episode_select.clone();
                            let checkbox_callback = Callback::from(move |_| {
                                if let Some(callback) = &on_select {
                                    callback.emit((episode_id, !is_selected));
                                }
                            });

                            html! {
                                <div class="flex flex-col items-center justify-center pl-4" style={format!("height: {}px;", *container_item_height)}>
                                    {
                                        if let Some(on_select_newer) = &props.on_select_newer {
                                            let episode_id = episode.episodeid;
                                            let callback = on_select_newer.clone();
                                            let newer_callback = Callback::from(move |_| {
                                                callback.emit(episode_id);
                                            });
                                            html! {
                                                <button
                                                    onclick={newer_callback}
                                                    class="episode-select-button mb-1"
                                                    title={select_newer_title.clone()}
                                                >
                                                    {"↑"}
                                                </button>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                    <input
                                        type="checkbox"
                                        checked={is_selected}
                                        onchange={checkbox_callback}
                                        class="w-5 h-5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 my-1"
                                    />
                                    {
                                        if let Some(on_select_older) = &props.on_select_older {
                                            let episode_id = episode.episodeid;
                                            let callback = on_select_older.clone();
                                            let older_callback = Callback::from(move |_| {
                                                callback.emit(episode_id);
                                            });
                                            html! {
                                                <button
                                                    onclick={older_callback}
                                                    class="episode-select-button mt-1"
                                                    title={select_older_title.clone()}
                                                >
                                                    {"↓"}
                                                </button>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }
                    <div class="flex flex-col w-auto object-cover pl-4">
                        <FallbackImage
                            src={episode.artworkurl.clone()}
                            alt={format!("{} {}", cover_for_text, &episode.episodetitle.clone())}
                            class="episode-image"
                        />
                    </div>
                    <div class="flex flex-col p-4 space-y-2 flex-grow md:w-7/12">
                        <div class="flex items-center space-x-2 cursor-pointer" onclick={make_shownotes_callback.clone()}>
                            <p class="item_container-text episode-title font-semibold line-clamp-2">
                                { &episode.episodetitle.clone() }
                            </p>
                            {
                                if episode.completed{
                                    html! {
                                        <i class="ph ph-check-circle text-2xl text-green-500"></i>
                                    }
                                } else {
                                    html! {}
                                }
                            }
                        </div>
                        {
                            html! {
                                <div class="item-description-text cursor-pointer hidden md:block"
                                     onclick={on_modal_open.clone()}>
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
                                { format_release.clone() }
                            </span>
                        </div>

                        {
                            if episode.completed {
                                // For completed episodes
                                if is_narrow_viewport {
                                    // In narrow viewports, just show "Completed"
                                    html! {
                                        <div class="flex items-center space-x-2">
                                            <span class="item_container-text">{completed_text.clone()}</span>
                                        </div>
                                    }
                                } else {
                                    // In wider viewports, show duration and "Completed"
                                    html! {
                                        <div class="flex items-center space-x-2">
                                            <span class="item_container-text">{ formatted_duration }</span>
                                            <span class="item_container-text">{ format!("-  {}", completed_text.clone()) }</span>
                                        </div>
                                    }
                                }
                            } else {
                                if  episode.listenduration > 0 {
                                    let listen_duration_percentage = if episode_duration_in_seconds > 0 {
                                        ((episode.listenduration as f64 / episode_duration_in_seconds as f64) * 100.0).min(100.0)
                                    } else {
                                        0.0
                                    };
                                    html! {
                                        <div class="flex items-center space-x-2">
                                            // Only show current position in wider viewports
                                            {
                                                if !is_narrow_viewport {
                                                    html! {
                                                        <span class="item_container-text">{ format_time(episode.listenduration as f64) }</span>
                                                    }
                                                } else {
                                                    html! {}
                                                }
                                            }
                                            <div class="progress-bar-container">
                                                <div class="progress-bar" style={ format!("width: {}%;", listen_duration_percentage) }></div>
                                            </div>
                                            <span class="item_container-text">{ formatted_duration }</span>
                                        </div>
                                    }
                                } else {
                                    html! {
                                        <span class="item_container-text">{ formatted_duration }</span>
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
                                        onclick={on_play_pause}
                                    >
                                        {
                                            if is_current_episode && is_playing {
                                                html! { <i class="ph ph-pause-circle md:text-6xl text-4xl"></i> }
                                            } else {
                                                html! { <i class="ph ph-play-circle md:text-6xl text-4xl"></i> }
                                            }
                                        }
                                    </button>
                                    {
                                        if props.podcast_added {
                                            let page_type = "episode_layout".to_string();
                                            html! {
                                                <div class="hidden sm:block">
                                                    <ContextButton episode={episode.clone()} page_type={page_type.clone()} />
                                                </div>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                }
                            </div>
                        }
                    }
                </div>
                </>
            }
        })
        .collect::<Html>();

    let total_height = props.episodes.len() as f64 * *item_height;
    let offset_y = start_index as f64 * *item_height;

    html! {
        <>
        <div
            ref={container_ref}
            class="virtual-list-container flex-grow overflow-y-auto"
            style="height: calc(100vh - 100px);"
        >
            <div style={format!("height: {}px; position: relative;", total_height)}>
                <div style={format!("position: absolute; top: {}px; left: 0; right: 0;", offset_y)}>
                    { visible_episodes }
                </div>
            </div>
        </div>
        {
            if let Some(index) = *selected_episode_index {
                let episode = &props.episodes[index];
                let sanitized_description = sanitize_html_with_blank_target(&episode.episodedescription.clone());
                let description = sanitized_description;
                let date_format = match_date_format(props.search_state.date_format.as_deref());
                let datetime = parse_date(&episode.episodepubdate.clone(), &props.search_state.user_tz);
                let format_release = format_datetime(&datetime, &props.search_state.hour_preference, date_format);

                // Create the callback here where we have access to index
                let modal_shownotes_callback = {
                    let history = props.history.clone();
                    let search_dispatch = props.search_dispatch.clone();
                    let podcast_link = props.podcast_link.clone();
                    let podcast_title = props.podcast_title.clone();
                    let episode_id = episode.episodeid;
                    let episode_url = episode.episodeurl.clone();
                    let is_youtube = episode.is_youtube;

                    Callback::from(move |_: MouseEvent| {
                        on_shownotes_click(
                            history.clone(),
                            search_dispatch.clone(),
                            Some(episode_id),
                            Some(podcast_link.clone()),
                            Some(episode_url.clone()),
                            Some(podcast_title.clone()),
                            true,
                            None,
                            is_youtube,
                        ).emit(MouseEvent::new("click").unwrap());
                    })
                };

                html! {
                    <EpisodeModal
                        episode_id={episode.episodeid}
                        episode_url={episode.episodeurl.clone()}
                        episode_artwork={episode.artworkurl.clone()}
                        episode_title={episode.episodetitle.clone()}
                        description={description}
                        format_release={format_release}
                        duration={episode.episodeduration.clone()}
                        on_close={on_modal_close.clone()}
                        on_show_notes={modal_shownotes_callback}
                        listen_duration_percentage={0.0}
                        is_youtube={episode.is_youtube}
                    />
                }
            } else {
                html! {}
            }
        }
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct PersonEpisodeVirtualListProps {
    pub episodes: Vec<Episode>,
    pub item_height: f64,
    pub search_state: Rc<AppState>,
    pub search_ui_state: Rc<UIState>,
    pub dispatch: Dispatch<UIState>,
    pub search_dispatch: Dispatch<AppState>,
    pub history: BrowserHistory,
    pub server_name: Option<String>,
    pub user_id: Option<i32>,
    pub api_key: Option<Option<String>>,
}

#[function_component(PersonEpisodeVirtualList)]
pub fn person_episode_virtual_list(props: &PersonEpisodeVirtualListProps) -> Html {
    let scroll_pos = use_state(|| 0.0);
    let container_ref = use_node_ref();
    let container_height = use_state(|| 600.0); // Fixed height for person episodes
    let item_height = use_state(|| 234.0); // Match existing episode item height

    // Effect for scroll handling
    {
        let scroll_pos = scroll_pos.clone();
        let container_ref = container_ref.clone();
        use_effect_with(container_ref.clone(), move |container_ref| {
            if let Some(container_element) = container_ref.cast::<HtmlElement>() {
                let container_element_clone = container_element.clone();
                let listener = EventListener::new(&container_element, "scroll", {
                    let scroll_pos = scroll_pos.clone();
                    move |_| {
                        scroll_pos.set(container_element_clone.scroll_top() as f64);
                    }
                });

                Box::new(move || drop(listener)) as Box<dyn FnOnce()>
            } else {
                Box::new(|| ()) as Box<dyn FnOnce()>
            }
        });
    }

    // Calculate visible range
    let visible_start = ((*scroll_pos as f64) / (*item_height as f64)).floor() as usize;
    let visible_count = ((*container_height as f64) / (*item_height as f64)).ceil() as usize + 1;
    let visible_end = (visible_start + visible_count).min(props.episodes.len());

    let total_height = props.episodes.len() as f64 * *item_height;
    let offset_y = visible_start as f64 * *item_height;

    html! {
        <div
            ref={container_ref}
            class="virtual-list-container"
            style={format!("height: {}px; overflow-y: auto;", *container_height)}
        >
            <div style={format!("height: {}px; position: relative;", total_height)}>
                <div style={format!("transform: translateY({}px);", offset_y)}>
                    { (visible_start..visible_end).map(|index| {
                        let episode = &props.episodes[index];
                        html! {
                            <EpisodeListItem
                                episode={ episode.clone() }
                                page_type={ "people" }
                                on_checkbox_change={ Callback::noop() }
                                is_delete_mode={ false }
                            />
                        }
                    }).collect::<Html>() }
                </div>
            </div>
        </div>
    }
}

// Helper function to calculate responsive item height - MUST be synchronous and accurate
#[allow(dead_code)]
fn calculate_item_height(window_width: f64) -> f64 {
    // CRITICAL: Must match the exact height that episodes render at, including margin
    // Episodes render at container_height + mb-4 margin (16px)
    let height = if window_width <= 530.0 {
        122.0 + 16.0 // Mobile: episode container 122px + mb-4 margin
    } else if window_width <= 768.0 {
        150.0 + 16.0 // Tablet: episode container 150px + mb-4 margin
    } else {
        221.0 + 16.0 // Desktop: episode container 221px + mb-4 margin
    };

    web_sys::console::log_1(
        &format!(
            "FEED HEIGHT CALC: width={}, calculated_height={}",
            window_width, height
        )
        .into(),
    );

    height
}

#[derive(Properties, PartialEq)]
pub struct VirtualListProps {
    pub episodes: Vec<Episode>,
    pub page_type: String,
}

#[function_component(VirtualList)]
pub fn virtual_list(props: &VirtualListProps) -> Html {
    let scroll_pos = use_state(|| 0.0);
    let container_ref = use_node_ref();
    let container_height = use_state(|| 0.0);
    let item_height = use_state(|| 234.0); // Default item height
    let force_update = use_state(|| 0);

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

                            // Always update scroll position for smoothest scrolling
                            if new_scroll_top != old_scroll_top {
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

    // Add buffer episodes above and below for smooth scrolling
    let buffer_size = 2; // Render 2 extra episodes above and below
    let buffered_start = start_index.saturating_sub(buffer_size);
    let buffered_end = (start_index + visible_count + buffer_size).min(props.episodes.len());

    let visible_episodes = (buffered_start..buffered_end)
        .map(|index| {
            let episode = props.episodes[index].clone();
            html! {
                <EpisodeListItem
                    key={format!("{}-{}", episode.episodeid, *force_update)}
                    episode={episode.clone()}
                    page_type={props.page_type.clone()}
                    on_checkbox_change={ Callback::noop() }
                    is_delete_mode={ false }
                />
            }
        })
        .collect::<Html>();

    let total_height = props.episodes.len() as f64 * *item_height;
    let offset_y = buffered_start as f64 * *item_height;

    html! {
        <div ref={container_ref}
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
            <div style={format!("height: {}px; flex-shrink: 0;", total_height - offset_y - (buffered_end - buffered_start) as f64 * *item_height)}></div>
        </div>
    }
}
