use crate::components::app_drawer::App_drawer;
use i18nrs::yew::use_translation;
use yew::{function_component, html, Html};

#[function_component(SearchNew)]
pub fn search_new() -> Html {
    let (i18n, _) = use_translation();

    html! {
        <div>
            <h1>{ &i18n.t("search_new.search_new") }</h1>
            <App_drawer />
        </div>
    }
}
