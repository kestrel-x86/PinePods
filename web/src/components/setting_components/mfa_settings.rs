use crate::components::context::AppState;
use crate::components::gen_funcs::format_error_message;
use crate::components::safehtml::SafeHtml;
use crate::requests::setting_reqs::{
    call_disable_mfa, call_generate_mfa_secret, call_mfa_settings, call_verify_temp_mfa,
};
use std::borrow::Borrow;
use wasm_bindgen::JsCast;
use yew::platform::spawn_local;
use yew::prelude::*;
use yewdux::prelude::*;
use i18nrs::yew::use_translation;

#[function_component(MFAOptions)]
pub fn mfa_options() -> Html {
    let (i18n, _) = use_translation();
    let (state, _dispatch) = use_store::<AppState>();
    let api_key = state.auth_details.as_ref().map(|ud| ud.api_key.clone());
    let user_id = state.user_details.as_ref().map(|ud| ud.UserID.clone());
    let server_name = state.auth_details.as_ref().map(|ud| ud.server_name.clone());
    let _error_message = state.error_message.clone();
    let mfa_status = use_state(|| false);
    let code = use_state(|| "".to_string());

    // Capture i18n strings before they get moved
    let i18n_error_getting_mfa_status = i18n.t("mfa_settings.error_getting_mfa_status").to_string();
    let i18n_mfa_code_verification_failed = i18n.t("mfa_settings.mfa_code_verification_failed").to_string();
    let i18n_failed_to_verify_mfa_code = i18n.t("mfa_settings.failed_to_verify_mfa_code").to_string();
    let i18n_error_disabling_mfa = i18n.t("mfa_settings.error_disabling_mfa").to_string();
    let i18n_failed_to_generate_totp_secret = i18n.t("mfa_settings.failed_to_generate_totp_secret").to_string();
    let i18n_close_modal = i18n.t("common.close_modal").to_string();
    let i18n_setup_mfa = i18n.t("mfa_settings.setup_mfa").to_string();
    let i18n_scan_qr_or_enter_code = i18n.t("mfa_settings.scan_qr_or_enter_code").to_string();
    let i18n_verify_code = i18n.t("mfa_settings.verify_code").to_string();
    let i18n_verify = i18n.t("mfa_settings.verify").to_string();
    let i18n_close = i18n.t("common.cancel").to_string();
    let i18n_mfa_options = i18n.t("mfa_settings.mfa_options").to_string();
    let i18n_mfa_description = i18n.t("mfa_settings.mfa_description").to_string();
    let i18n_enable_mfa = i18n.t("mfa_settings.enable_mfa").to_string();

    let effect_user_id = user_id.clone();
    let effect_api_key = api_key.clone();
    let effect_server_name = server_name.clone();
    let dispatch_effect = _dispatch.clone();
    {
        let mfa_status = mfa_status.clone();
        let i18n_error_getting_mfa_status_clone1 = i18n_error_getting_mfa_status.clone();
        use_effect_with(
            (effect_api_key.clone(), effect_server_name.clone()),
            move |(_api_key, _server_name)| {
                let mfa_status = mfa_status.clone();
                let api_key = effect_api_key.clone();
                let server_name = effect_server_name.clone();
                let user_id = effect_user_id.clone();
                let i18n_error_getting_mfa_status = i18n_error_getting_mfa_status_clone1.clone();
                let future = async move {
                    if let (Some(api_key), Some(server_name)) = (api_key, server_name) {
                        let response =
                            call_mfa_settings(server_name, api_key.unwrap(), user_id.unwrap())
                                .await;
                        match response {
                            Ok(mfa_settings_response) => {
                                mfa_status.set(mfa_settings_response);
                            }
                            Err(e) => {
                                let formatted_error = format_error_message(&e.to_string());
                                dispatch_effect.reduce_mut(|audio_state| {
                                    audio_state.error_message = Option::from(format!(
                                        "{}{}",
                                        i18n_error_getting_mfa_status.clone(),
                                        formatted_error
                                    ))
                                });
                            }
                        }
                    }
                };
                spawn_local(future);
                // Return cleanup function
                || {}
            },
        );
    }
    let dispatch_refresh = _dispatch.clone();
    // Re-fetch MFA status after setup is complete
    {
        let mfa_status = mfa_status.clone();
        let api_key = api_key.clone();
        let server_name = server_name.clone();
        let user_id = user_id.clone();
        let i18n_error_getting_mfa_status_clone2 = i18n_error_getting_mfa_status.clone();

        use_effect_with(mfa_status.clone(), move |_| {
            let mfa_status = mfa_status.clone();
            let api_key = api_key.clone();
            let server_name = server_name.clone();
            let user_id = user_id.clone();

            let i18n_error_getting_mfa_status = i18n_error_getting_mfa_status_clone2.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let (Some(api_key), Some(server_name)) = (api_key, server_name) {
                    match call_mfa_settings(server_name, api_key.unwrap(), user_id.unwrap()).await {
                        Ok(mfa_settings_response) => {
                            mfa_status.set(mfa_settings_response);
                        }
                        Err(e) => {
                            let formatted_error = format_error_message(&e.to_string());
                            dispatch_refresh.reduce_mut(|audio_state| {
                                audio_state.error_message = Option::from(format!(
                                    "{}{}",
                                    i18n_error_getting_mfa_status.clone(),
                                    formatted_error
                                ))
                            });
                        }
                    }
                }
            });

            || ()
        });
    }
    // let html_self_service = self_service_status.clone();
    let loading = use_state(|| false);

    // Define the state of the application
    #[derive(Clone, PartialEq)]
    enum PageState {
        Hidden,
        Setup,
    }

    // Define the initial state
    let page_state = use_state(|| PageState::Hidden);
    let mfa_code = use_state(|| String::new());
    let mfa_secret = use_state(|| String::new());

    // Define the function to close the modal
    let close_modal = {
        let page_state = page_state.clone();
        Callback::from(move |_| {
            page_state.set(PageState::Hidden);
        })
    };

    let on_background_click = {
        let on_close_modal = close_modal.clone();
        Callback::from(move |e: MouseEvent| {
            let target = e.target().unwrap();
            let element = target.dyn_into::<web_sys::Element>().unwrap();
            if element.tag_name() == "DIV" {
                on_close_modal.emit(e);
            }
        })
    };

    let stop_propagation = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    let open_setup_modal = {
        let mfa_code = mfa_code.clone();
        let page_state = page_state.clone();
        let mfa_secret = mfa_secret.clone();
        let server_name = server_name.clone(); // Replace with actual server name
        let api_key = api_key.clone(); // Replace with actual API key
        let user_id = user_id.clone(); // Replace with actual user ID
        let mfa_status = mfa_status.clone();
        let i18n_error_disabling_mfa = i18n_error_disabling_mfa.clone();
        let i18n_failed_to_generate_totp_secret = i18n_failed_to_generate_totp_secret.clone();

        Callback::from(move |_| {
            let i18n_error_disabling_mfa = i18n_error_disabling_mfa.clone();
            let i18n_failed_to_generate_totp_secret = i18n_failed_to_generate_totp_secret.clone();
            let mfa_code = mfa_code.clone();
            let page_state = page_state.clone();
            let mfa_secret = mfa_secret.clone();
            let server_name = server_name.clone();
            let api_key = api_key.clone();
            let user_id = user_id;
            let mfa_status = mfa_status.clone();

            // Now call the API to generate the TOTP secret
            wasm_bindgen_futures::spawn_local(async move {
                match call_generate_mfa_secret(
                    server_name.clone().unwrap(),
                    api_key.clone().unwrap().unwrap(),
                    user_id.clone().unwrap(),
                )
                .await
                {
                    Ok(response) => {
                        if *mfa_status {
                            let result = call_disable_mfa(
                                &server_name.unwrap(),
                                &api_key.unwrap().unwrap(),
                                user_id.unwrap(),
                            )
                            .await;

                            match result {
                                Ok(_) => {
                                    // Handle success
                                    page_state.set(PageState::Hidden); // Hide the modal
                                    mfa_status.set(false);
                                }
                                Err(e) => {
                                    // Handle error
                                    log::error!("{}{:?}", i18n_error_disabling_mfa, e);
                                }
                            }
                        } else {
                            mfa_secret.set(response.secret);
                            mfa_code.set(response.qr_code_svg); // Directly use the SVG QR code
                            page_state.set(PageState::Setup); // Move to the setup page state
                        }
                    }
                    Err(e) => {
                        log::error!("{}{}", i18n_failed_to_generate_totp_secret, e);
                        // Handle error appropriately
                    }
                }
            });
        })
    };

    // Define the function to close the modal
    let verify_code = {
        let page_state = page_state.clone();
        let api_key = api_key.clone();
        let user_id = state.user_details.as_ref().map(|ud| ud.UserID.clone());
        let server_name = server_name.clone();
        let code = code.clone();
        let mfa_status_clone = mfa_status.clone();
        let i18n_mfa_code_verification_failed = i18n_mfa_code_verification_failed.clone();
        let i18n_failed_to_verify_mfa_code = i18n_failed_to_verify_mfa_code.clone();

        Callback::from(move |_| {
            let i18n_mfa_code_verification_failed = i18n_mfa_code_verification_failed.clone();
            let i18n_failed_to_verify_mfa_code = i18n_failed_to_verify_mfa_code.clone();
            let api_key = api_key.clone();
            let user_id = user_id.clone();
            let server_name = server_name.clone();
            let page_state = page_state.clone();
            let code = code.clone();
            let _dispatch = _dispatch.clone();
            let mfa_status_update = mfa_status_clone.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match call_verify_temp_mfa(
                    &server_name.unwrap(),
                    &api_key.unwrap().unwrap(),
                    user_id.unwrap(),
                    (*code).clone(),
                )
                .await
                {
                    Ok(response) => {
                        if response.verified {
                            // Handle successful verification, e.g., updating UI state or navigating
                            page_state.set(PageState::Hidden); // Example: hiding MFA prompt
                            mfa_status_update.set(true); // Update MFA status
                                                         // refresh_mfa_status.emit(());
                        } else {
                            _dispatch.reduce_mut(|audio_state| {
                                audio_state.error_message =
                                    Option::from(i18n_mfa_code_verification_failed.clone())
                            });
                            // Handle failed verification, e.g., showing an error message
                        }
                    }
                    Err(e) => {
                        let formatted_error = format_error_message(&e.to_string());
                        _dispatch.reduce_mut(|audio_state| {
                            audio_state.error_message = Option::from(format!(
                                "{}{}",
                                i18n_failed_to_verify_mfa_code,
                                formatted_error
                            ))
                        });
                        // Handle error appropriately, e.g., showing an error message
                    }
                }
            });
        })
    };

    let on_code_change = {
        let code = code.clone();
        Callback::from(move |e: InputEvent| {
            code.set(
                e.target_unchecked_into::<web_sys::HtmlInputElement>()
                    .value(),
            );
        })
    };
    // let svg_data_url = format!("data:image/svg+xml;utf8,{}", url_encode(&(*mfa_code).clone()));
    let qr_code_svg = (*mfa_code).clone();
    let setup_mfa_modal = html! {
        <div id="setup-mfa-modal" tabindex="-1" aria-hidden="true" class="fixed top-0 right-0 left-0 z-50 flex justify-center items-center w-full h-[calc(100%-1rem)] max-h-full bg-black bg-opacity-25" onclick={on_background_click.clone()}>
            <div class="modal-container relative p-4 w-full max-w-md max-h-full rounded-lg shadow" onclick={stop_propagation.clone()}>
                <div class="modal-container relative rounded-lg shadow">
                    <div class="flex flex-col items-start justify-between p-4 md:p-5 border-b rounded-t dark:border-gray-600">
                        <button onclick={close_modal.clone()} class="self-end text-gray-400 bg-transparent hover:bg-gray-200 hover:text-gray-900 rounded-lg text-sm w-8 h-8 ms-auto inline-flex justify-center items-center dark:hover:bg-gray-600 dark:hover:text-white">
                            <svg class="w-3 h-3" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 14 14">
                                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m1 1 6 6m0 0 6 6M7 7l6-6M7 7l-6 6"/>
                            </svg>
                            <span class="sr-only">{&i18n_close_modal}</span>
                        </button>
                        <h3 class="text-xl font-semibold">
                            {&i18n_setup_mfa}
                        </h3>
                        <p class="item_container-text text-m font-semibold">
                            {&i18n_scan_qr_or_enter_code}
                        </p>
                        <div class="mt-4 self-center bg-white rounded-lg overflow-hidden p-4 shadow-lg">
                            <SafeHtml html={qr_code_svg} />
                        </div>
                        // More HTML as needed

                        <div class="mfa-code-box mt-4 p-4 rounded-md overflow-x-auto whitespace-nowrap max-w-full">
                            {(*mfa_secret).clone()}
                        </div>
                        <div>
                            <label for="fullname" class="block mb-2 mt-2 text-sm font-semibold font-medium">{&i18n_verify_code}</label>
                            <input oninput={on_code_change} type="text" id="fullname" name="fullname" class="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-600 dark:border-gray-500 dark:placeholder-gray-400 dark:text-white" required=true />
                        </div>
                        <div class="flex justify-between space-x-4">
                            <button onclick={verify_code.clone()} class="mt-4 download-button font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline">
                                {&i18n_verify}
                            </button>
                            <button onclick={close_modal.clone()} class="mt-4 download-button font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline">
                                {&i18n_close}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    };

    html! {
        <>
        {
            match *page_state {
            PageState::Setup => setup_mfa_modal,
            _ => html! {},
            }
        }
        <div class="p-4"> // You can adjust the padding as needed
            <p class="item_container-text text-lg font-bold mb-4">{&i18n_mfa_options}</p>
            <p class="item_container-text text-md mb-4">{&i18n_mfa_description}</p> // Styled paragraph

            <label class="relative inline-flex items-center cursor-pointer">
            <input type="checkbox" disabled={**loading.borrow()} checked={**mfa_status.borrow()} class="sr-only peer" onclick={open_setup_modal.clone()} />
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                <span class="ms-3 text-sm font-medium item_container-text">{&i18n_enable_mfa}</span>
            </label>
        </div>
        </>
    }
}
