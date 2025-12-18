use crate::components::context_menu_button::PageType;
use crate::components::episode_list_item::EpisodeListItem;
use crate::requests::episode::Episode;
use gloo::events::EventListener;
use i18nrs::yew::use_translation;
use wasm_bindgen::JsCast;
use web_sys::{window, Element, HtmlElement};
use yew::prelude::*;
use yew::Properties;
use yew::{function_component, html, use_effect_with, use_node_ref, Callback, Html};
use yewdux::prelude::*;

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

/// Any required callbacks for drag interactions. Field can be None if no callback is required.
/// If no fields are set, dragging for this VirtualList will be disabled
#[derive(Properties, PartialEq, Clone, Default)]
pub struct DragCallbacks {
    pub ondragstart: Option<Callback<DragEvent>>,
    pub ondragenter: Option<Callback<DragEvent>>,
    pub ondragover: Option<Callback<DragEvent>>,
    pub ondrop: Option<Callback<DragEvent>>,
}

impl DragCallbacks {
    /// Item is draggable if any callback field is set
    pub fn draggable(&self) -> bool {
        return self.ondragstart.is_some()
            || self.ondragenter.is_some()
            || self.ondragover.is_some()
            || self.ondrop.is_some();
    }
}

#[derive(Properties, PartialEq)]
pub struct VirtualListProps {
    pub episodes: Vec<Episode>,
    #[prop_or(PageType::Default)]
    pub page_type: PageType,
    #[prop_or_default]
    pub drag_callbacks: DragCallbacks,
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
                    drag_callbacks={ props.drag_callbacks.clone() }
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
