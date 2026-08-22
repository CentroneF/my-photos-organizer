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
struct ImportSourceRequest<'a> {
    folder_path: &'a str,
}

#[derive(Serialize)]
struct ImportSourceInvokeArgs<'a> {
    request: ImportSourceRequest<'a>,
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
}

#[derive(Deserialize)]
struct CommandError {
    message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RememberedLibrary {
    state: String,
    folder_path: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportSource {
    state: String,
    folder_path: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryQuestion {
    question: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UnlockRequest<'a> {
    password: &'a str,
}

#[derive(Serialize)]
struct UnlockInvokeArgs<'a> {
    request: UnlockRequest<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenExistingRequest<'a> {
    folder_path: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
struct OpenExistingInvokeArgs<'a> {
    request: OpenExistingRequest<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResetRequest<'a> {
    folder_path: &'a str,
    recovery_answer: &'a str,
    new_password: &'a str,
    new_password_confirmation: &'a str,
}

#[derive(Serialize)]
struct ResetInvokeArgs<'a> {
    request: ResetRequest<'a>,
}

fn command_error(value: JsValue, fallback: &str) -> String {
    serde_wasm_bindgen::from_value::<CommandError>(value)
        .map(|error| error.message)
        .unwrap_or_else(|_| fallback.to_owned())
}

pub fn App() -> Element {
    let mut step = use_signal(|| "loading".to_owned());
    let mut folder = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirmation = use_signal(String::new);
    let mut question = use_signal(String::new);
    let mut answer = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut import_source = use_signal(|| ImportSource {
        state: "missing".into(),
        folder_path: None,
    });
    let mut busy = use_signal(|| false);
    let mut recovery_question = use_signal(String::new);
    let mut show_recovery = use_signal(|| false);
    let mut new_password = use_signal(String::new);
    let mut new_confirmation = use_signal(String::new);

    use_effect(move || {
        spawn(async move {
            match invoke("remembered_library", JsValue::NULL).await {
                Ok(value) => match serde_wasm_bindgen::from_value::<RememberedLibrary>(value) {
                    Ok(remembered) => {
                        if let Some(path) = remembered.folder_path {
                            folder.set(path);
                        }
                        step.set(
                            match remembered.state.as_str() {
                                "ready" => "unlock",
                                "stale" => "stale",
                                _ => "folder",
                            }
                            .to_owned(),
                        );
                    }
                    Err(_) => {
                        error.set("Could not read the remembered library.".into());
                        step.set("folder".into());
                    }
                },
                Err(_) => {
                    step.set("folder".into());
                }
            }
        });
    });

    use_effect(move || {
        if step() != "home" {
            return;
        }
        spawn(async move {
            match invoke("remembered_import_source", JsValue::NULL).await {
                Ok(value) => match serde_wasm_bindgen::from_value::<ImportSource>(value) {
                    Ok(source) => import_source.set(source),
                    Err(_) => error.set("Could not read the remembered import folder.".into()),
                },
                Err(value) => error.set(command_error(
                    value,
                    "Could not read the remembered import folder.",
                )),
            }
        });
    });

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
        password.set(String::new());
        confirmation.set(String::new());
        question.set(String::new());
        answer.set(String::new());
        new_password.set(String::new());
        new_confirmation.set(String::new());
        error.set(String::new());
    };

    let choose_import_source = move |_| async move {
        error.set(String::new());
        let picker = PickerInvokeArgs {
            options: PickerRequest {
                directory: true,
                multiple: false,
                title: "Choose a folder to import from",
            },
        };
        let Ok(args) = serde_wasm_bindgen::to_value(&picker) else {
            error.set("Could not open the import-folder picker.".into());
            return;
        };
        let selected = match invoke("plugin:dialog|open", args).await {
            Ok(value) if !value.is_null() => value.as_string(),
            Ok(_) => None,
            Err(value) => {
                error.set(command_error(
                    value,
                    "Could not open the import-folder picker.",
                ));
                return;
            }
        };
        let Some(selected) = selected else { return };

        busy.set(true);
        let result = match serde_wasm_bindgen::to_value(&ImportSourceInvokeArgs {
            request: ImportSourceRequest {
                folder_path: &selected,
            },
        }) {
            Ok(args) => invoke("save_import_source", args).await,
            Err(_) => {
                busy.set(false);
                error.set("Could not save the selected import folder.".into());
                return;
            }
        };
        busy.set(false);
        match result {
            Ok(value) => match serde_wasm_bindgen::from_value::<ImportSource>(value) {
                Ok(source) => import_source.set(source),
                Err(_) => {
                    error.set("The import-folder selection returned an unexpected response.".into())
                }
            },
            Err(value) => error.set(command_error(
                value,
                "That folder cannot be used as the import source.",
            )),
        }
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
                    question.set(String::new());
                    answer.set(String::new());
                    folder.set(created.folder_path);
                    step.set("home".into());
                }
                Err(_) => error.set("Setup returned an unexpected response.".into()),
            },
            Err(value) => error.set(command_error(value, "Setup could not be completed safely.")),
        }
    };

    let unlock = move |event: FormEvent| async move {
        event.prevent_default();
        error.set(String::new());
        busy.set(true);
        let selected_folder = folder();
        let entered_password = password();
        let result = if selected_folder.is_empty() {
            serde_wasm_bindgen::to_value(&UnlockInvokeArgs {
                request: UnlockRequest {
                    password: &entered_password,
                },
            })
            .map_err(|_| JsValue::NULL)
            .and_then(|args| Ok(args))
        } else {
            serde_wasm_bindgen::to_value(&OpenExistingInvokeArgs {
                request: OpenExistingRequest {
                    folder_path: &selected_folder,
                    password: &entered_password,
                },
            })
            .map_err(|_| JsValue::NULL)
            .and_then(|args| Ok(args))
        };
        let result = match result {
            Ok(args) => {
                if selected_folder.is_empty() {
                    invoke("unlock_library", args).await
                } else {
                    invoke("open_existing_library", args).await
                }
            }
            Err(_) => Err(JsValue::NULL),
        };
        busy.set(false);
        match result {
            Ok(value) => match serde_wasm_bindgen::from_value::<SetupResult>(value) {
                Ok(result) => {
                    password.set(String::new());
                    folder.set(result.folder_path);
                    step.set("home".into());
                }
                Err(_) => error.set("Unlock returned an unexpected response.".into()),
            },
            Err(value) => error.set(command_error(value, "The library could not be unlocked.")),
        }
    };

    let begin_recovery = move |_| async move {
        error.set(String::new());
        busy.set(true);
        let selected_folder = folder();
        let result = match serde_wasm_bindgen::to_value(&FolderInvokeArgs {
            request: FolderRequest {
                folder_path: &selected_folder,
            },
        }) {
            Ok(args) => invoke("recovery_question", args).await,
            Err(_) => Err(JsValue::NULL),
        };
        busy.set(false);
        match result {
            Ok(value) => match serde_wasm_bindgen::from_value::<RecoveryQuestion>(value) {
                Ok(result) => {
                    recovery_question.set(result.question);
                    show_recovery.set(true);
                    password.set(String::new());
                }
                Err(_) => error.set("Could not read the recovery question.".into()),
            },
            Err(value) => error.set(command_error(
                value,
                "Recovery is unavailable for this library.",
            )),
        }
    };

    let reset_password = move |event: FormEvent| async move {
        event.prevent_default();
        error.set(String::new());
        busy.set(true);
        let answer_value = answer();
        let next_password = new_password();
        let next_confirmation = new_confirmation();
        let selected_folder = folder();
        let result = match serde_wasm_bindgen::to_value(&ResetInvokeArgs {
            request: ResetRequest {
                folder_path: &selected_folder,
                recovery_answer: &answer_value,
                new_password: &next_password,
                new_password_confirmation: &next_confirmation,
            },
        }) {
            Ok(args) => invoke("reset_library_password", args).await,
            Err(_) => Err(JsValue::NULL),
        };
        busy.set(false);
        match result {
            Ok(value) => match serde_wasm_bindgen::from_value::<SetupResult>(value) {
                Ok(result) => {
                    answer.set(String::new());
                    new_password.set(String::new());
                    new_confirmation.set(String::new());
                    show_recovery.set(false);
                    folder.set(result.folder_path);
                    step.set("home".into());
                }
                Err(_) => error.set("Password reset returned an unexpected response.".into()),
            },
            Err(value) => error.set(command_error(value, "The recovery answer is incorrect.")),
        }
    };

    let step_one_class = if step() == "folder" {
        "progress-dot active"
    } else {
        "progress-dot done"
    };
    let step_two_class = if step() == "new" {
        "progress-dot active"
    } else if step() == "home" {
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
                    } else if step() == "loading" {
                        p { class: "step-label", "STARTING" }
                        h2 { "Finding your protected library…" }
                        p { class: "lede", "We are checking only the remembered local library location." }
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
                    } else if step() == "unlock" || step() == "existing" {
                        p { class: "step-label", "LIBRARY FOUND" }
                        h2 { "Unlock your protected library" }
                        p { class: "lede", "Your encrypted catalogue is ready. Original media stays exactly where it is." }
                        div { class: "folder-summary",
                            div { class: "folder-summary-heading",
                                span { "Existing library" }
                                button { r#type: "button", onclick: choose_another, "Open another" }
                            }
                            strong { "{folder}" }
                        }
                        if show_recovery() {
                            form { class: "setup-form", onsubmit: reset_password,
                                label { "Recovery question" input { value: "{recovery_question}", readonly: true } }
                                label { "Recovery answer" input { r#type: "password", autocomplete: "off", value: "{answer}", oninput: move |event| answer.set(event.value()) } }
                                div { class: "field-pair",
                                    label { "New password" input { r#type: "password", autocomplete: "new-password", value: "{new_password}", oninput: move |event| new_password.set(event.value()) } }
                                    label { "Confirm new password" input { r#type: "password", autocomplete: "new-password", value: "{new_confirmation}", oninput: move |event| new_confirmation.set(event.value()) } }
                                }
                                p { class: "privacy-note", "Recovery is local only. There is no email or cloud reset." }
                                if !error().is_empty() { p { class: "error-message", role: "alert", "{error}" } }
                                button { class: "primary-button", r#type: "submit", disabled: busy(), if busy() { "Resetting password…" } else { "Reset password" } }
                                button { class: "secondary-button", r#type: "button", onclick: move |_| { show_recovery.set(false); answer.set(String::new()); new_password.set(String::new()); new_confirmation.set(String::new()); }, "Cancel recovery" }
                            }
                        } else {
                            form { class: "setup-form", onsubmit: unlock,
                                label { "Password" input { r#type: "password", autocomplete: "current-password", value: "{password}", oninput: move |event| password.set(event.value()) } }
                                if !error().is_empty() { p { class: "error-message", role: "alert", "{error}" } }
                                button { class: "primary-button", r#type: "submit", disabled: busy(), if busy() { "Unlocking…" } else { "Unlock library" } }
                                button { class: "secondary-button", r#type: "button", onclick: begin_recovery, disabled: busy(), "Use local recovery" }
                            }
                        }
                    } else if step() == "stale" {
                        p { class: "step-label", "LIBRARY UNAVAILABLE" }
                        h2 { "Your remembered library is unavailable." }
                        p { class: "lede", "It may have moved or be offline. Choose it again; Photo Handler will only open a recognized library after its password is valid." }
                        if !folder().is_empty() { div { class: "folder-summary", span { "Remembered location" } strong { "{folder}" } } }
                        button { class: "primary-button", r#type: "button", onclick: choose_another, "Open existing library" }
                    } else if step() == "home" {
                        p { class: "step-label success-label", "LIBRARY HOME" }
                        h2 { "Choose where to import from." }
                        p { class: "lede", "Your protected library is ready. Choosing an import folder only remembers its location; Photo Handler will not scan or modify its files yet." }
                        div { class: "folder-summary", span { "Protected library" } strong { "{folder}" } }
                        if import_source().state == "ready" {
                            div { class: "folder-summary",
                                span { "Import source" }
                                strong { "{import_source().folder_path.clone().unwrap_or_default()}" }
                                button { r#type: "button", onclick: choose_import_source, disabled: busy(), "Change" }
                            }
                        } else if import_source().state == "stale" {
                            p { class: "step-label", "IMPORT FOLDER UNAVAILABLE" }
                            p { class: "lede", "The remembered import folder may have moved or be offline. Choose another folder when it is available." }
                            div { class: "folder-summary",
                                span { "Remembered location" }
                                strong { "{import_source().folder_path.clone().unwrap_or_default()}" }
                                button { r#type: "button", onclick: choose_import_source, disabled: busy(), "Change" }
                            }
                        } else {
                            button { class: "folder-picker", r#type: "button", onclick: choose_import_source, disabled: busy(),
                                span { class: "folder-icon", "⌑" }
                                span { strong { if busy() { "Saving import folder…" } else { "Choose import folder" } } small { "Any folder is allowed except your protected library" } }
                                span { class: "arrow", "→" }
                            }
                        }
                    }

                    if !error().is_empty() && (step() == "folder" || step() == "stale" || step() == "home") { p { class: "error-message", role: "alert", "{error}" } }
                }
            }
        }
    }
}
