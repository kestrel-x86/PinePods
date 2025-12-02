use yew::prelude::*;

#[function_component(Loading)]
pub fn loading() -> Html {
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
