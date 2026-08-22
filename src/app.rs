#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

static CSS: Asset = asset!("/assets/styles.css");

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[derive(Serialize)]
struct PickerRequest {
    directory: bool,
    multiple: bool,
    title: &'static str,
}

#[derive(Serialize)]
struct PickerInvokeArgs {
    options: PickerRequest,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FolderRequest<'a> {
    folder_path: &'a str,
}

#[derive(Serialize)]
struct FolderInvokeArgs<'a> {
    request: FolderRequest<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupRequest<'a> {
    folder_path: &'a str,
    password: &'a str,
    password_confirmation: &'a str,
    recovery_question: &'a str,
    recovery_answer: &'a str,
}

#[derive(Serialize)]
struct SetupInvokeArgs<'a> {
    request: SetupRequest<'a>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FolderInspection {
    folder_path: String,
    state: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupResult {
    folder_path: String,
    message: String,
}

#[derive(Deserialize)]
struct CommandError {
    message: String,
}

fn command_error(value: JsValue, fallback: &str) -> String {
    serde_wasm_bindgen::from_value::<CommandError>(value)
        .map(|error| error.message)
        .unwrap_or_else(|_| fallback.to_owned())
}

pub fn App() -> Element {
    let mut step = use_signal(|| "folder".to_owned());
    let mut folder = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirmation = use_signal(String::new);
    let mut question = use_signal(String::new);
    let mut answer = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut success = use_signal(|| None::<SetupResult>);
    let mut busy = use_signal(|| false);

    let choose_folder = move |_| async move {
        error.set(String::new());
        let picker = PickerInvokeArgs {
            options: PickerRequest {
                directory: true,
                multiple: false,
                title: "Choose a Photo Handler library folder",
            },
        };
        let Ok(args) = serde_wasm_bindgen::to_value(&picker) else {
            error.set("Could not open the folder picker.".into());
            return;
        };
        let selected = match invoke("plugin:dialog|open", args).await {
            Ok(value) if !value.is_null() => value.as_string(),
            Ok(_) => None,
            Err(value) => {
                error.set(command_error(value, "Could not open the folder picker."));
                return;
            }
        };
        let Some(selected) = selected else { return };

        busy.set(true);
        let request = FolderInvokeArgs {
            request: FolderRequest {
                folder_path: &selected,
            },
        };
        let result = match serde_wasm_bindgen::to_value(&request) {
            Ok(args) => invoke("inspect_library_folder", args).await,
            Err(_) => {
                busy.set(false);
                error.set("Could not inspect the selected folder.".into());
                return;
            }
        };
        busy.set(false);
        match result {
            Ok(value) => match serde_wasm_bindgen::from_value::<FolderInspection>(value) {
                Ok(inspection) => {
                    folder.set(inspection.folder_path);
                    step.set(inspection.state);
                }
                Err(_) => {
                    error.set("The folder inspection returned an unexpected response.".into())
                }
            },
            Err(value) => error.set(command_error(value, "That folder cannot be used safely.")),
        }
    };

    let choose_another = move |_| {
        step.set("folder".into());
        folder.set(String::new());
        error.set(String::new());
    };

    let create = move |event: FormEvent| async move {
        event.prevent_default();
        error.set(String::new());
        busy.set(true);
        let current_folder = folder();
        let current_password = password();
        let current_confirmation = confirmation();
        let current_question = question();
        let current_answer = answer();
        let request = SetupInvokeArgs {
            request: SetupRequest {
                folder_path: &current_folder,
                password: &current_password,
                password_confirmation: &current_confirmation,
                recovery_question: &current_question,
                recovery_answer: &current_answer,
            },
        };
        let result = match serde_wasm_bindgen::to_value(&request) {
            Ok(args) => invoke("setup_library", args).await,
            Err(_) => {
                busy.set(false);
                error.set("Could not prepare protected-library setup.".into());
                return;
            }
        };
        busy.set(false);
        match result {
            Ok(value) => match serde_wasm_bindgen::from_value::<SetupResult>(value) {
                Ok(created) => {
                    password.set(String::new());
                    confirmation.set(String::new());
                    answer.set(String::new());
                    success.set(Some(created));
                    step.set("complete".into());
                }
                Err(_) => error.set("Setup returned an unexpected response.".into()),
            },
            Err(value) => error.set(command_error(value, "Setup could not be completed safely.")),
        }
    };

    let step_one_class = if step() == "folder" {
        "progress-dot active"
    } else {
        "progress-dot done"
    };
    let step_two_class = if step() == "new" {
        "progress-dot active"
    } else if step() == "complete" {
        "progress-dot done"
    } else {
        "progress-dot"
    };

    rsx! {
        link { rel: "stylesheet", href: CSS }
        main { class: "app-shell",
            aside { class: "brand-panel",
                div { class: "brand-mark", "PH" }
                div {
                    p { class: "eyebrow", "PHOTO HANDLER" }
                    h1 { "Your memories, indexed privately." }
                    p { class: "brand-copy", "A local catalogue that keeps every original exactly where you put it." }
                }
                ul { class: "trust-list",
                    li { span { "01" } "Your files stay on this device" }
                    li { span { "02" } "Original media is never moved" }
                    li { span { "03" } "Your catalogue is encrypted" }
                }
            }
            section { class: "flow-panel",
                div { class: "flow-wrap",
                    div { class: "progress-row",
                        span { class: step_one_class, "1" }
                        div { class: "progress-line" }
                        span { class: step_two_class, "2" }
                    }

                    if step() == "folder" {
                        p { class: "step-label", "STEP 1 OF 2" }
                        h2 { "Where should your library live?" }
                        p { class: "lede", "Start by choosing a folder. We’ll inspect it without changing anything, then guide you to the right next step." }
                        button { class: "folder-picker", r#type: "button", onclick: choose_folder, disabled: busy(),
                            span { class: "folder-icon", "⌑" }
                            span { strong { if busy() { "Inspecting folder…" } else { "Choose a folder" } } small { "Empty folder or an existing Photo Handler library" } }
                            span { class: "arrow", "→" }
                        }
                    } else if step() == "new" {
                        p { class: "step-label", "STEP 2 OF 2" }
                        h2 { "Protect your new library" }
                        p { class: "lede", "This folder has no Photo Handler configuration yet. Add protection details to create one." }
                        div { class: "folder-summary", span { "Selected folder" } strong { "{folder}" } button { r#type: "button", onclick: choose_another, "Change" } }
                        form { class: "setup-form", onsubmit: create,
                            div { class: "field-pair",
                                label { "Password" input { r#type: "password", autocomplete: "new-password", value: "{password}", oninput: move |event| password.set(event.value()) } }
                                label { "Confirm password" input { r#type: "password", autocomplete: "new-password", value: "{confirmation}", oninput: move |event| confirmation.set(event.value()) } }
                            }
                            label { "Local recovery question" input { value: "{question}", placeholder: "For example, your first pet’s name?", oninput: move |event| question.set(event.value()) } }
                            label { "Recovery answer" input { r#type: "password", autocomplete: "off", value: "{answer}", oninput: move |event| answer.set(event.value()) } }
                            p { class: "privacy-note", "Recovery stays on this device. There is no email or cloud reset." }
                            if !error().is_empty() { p { class: "error-message", role: "alert", "{error}" } }
                            button { class: "primary-button", r#type: "submit", disabled: busy(), if busy() { "Creating protected library…" } else { "Create protected library" } }
                        }
                    } else if step() == "existing" {
                        p { class: "step-label", "LIBRARY FOUND" }
                        h2 { "This folder is already configured." }
                        p { class: "lede", "We found a protected Photo Handler catalogue and left it unchanged. Unlocking existing libraries arrives in the next setup phase." }
                        div { class: "folder-summary", span { "Existing library" } strong { "{folder}" } }
                        button { class: "secondary-button", r#type: "button", onclick: choose_another, "Choose another folder" }
                    } else if let Some(created) = &*success.read() {
                        p { class: "step-label success-label", "READY" }
                        div { class: "success-icon", "✓" }
                        h2 { "Your protected library is ready." }
                        p { class: "lede", "{created.message}" }
                        div { class: "folder-summary", span { "Library folder" } strong { "{created.folder_path}" } }
                    }

                    if !error().is_empty() && step() == "folder" { p { class: "error-message", role: "alert", "{error}" } }
                }
            }
        }
    }
}
