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
    recursive: bool,
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

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewState {
    state: String,
    source_path: Option<String>,
    candidate_count: u64,
    message: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewItem {
    state: String,
    candidate_id: Option<i64>,
    relative_path: Option<String>,
    filename: Option<String>,
    media_type: Option<String>,
    effective_import_date: Option<String>,
    date_origin: Option<String>,
    tags: Vec<String>,
    preview_url: Option<String>,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecideRequest {
    candidate_id: i64,
    tags: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    candidate_id: i64,
    tags: Vec<String>,
    effective_import_date: String,
}

#[derive(Serialize)]
struct ReviewInvokeArgs<T> {
    request: T,
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
    let mut review_state = use_signal(|| ReviewState {
        state: "none".into(),
        source_path: None,
        candidate_count: 0,
        message: String::new(),
    });
    let mut review_item = use_signal(|| ReviewItem {
        state: "loading".into(),
        candidate_id: None,
        relative_path: None,
        filename: None,
        media_type: None,
        effective_import_date: None,
        date_origin: None,
        tags: vec![],
        preview_url: None,
        message: String::new(),
    });
    let mut review_tags = use_signal(String::new);
    let mut import_date = use_signal(String::new);
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
        spawn(async move {
            if let Ok(value) = invoke("current_review_state", JsValue::NULL).await {
                if let Ok(state) = serde_wasm_bindgen::from_value::<ReviewState>(value) {
                    review_state.set(state);
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
                recursive: false,
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
                recursive: true,
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

    let start_review = move |_| async move {
        error.set(String::new());
        busy.set(true);
        let selected = import_source().folder_path.unwrap_or_default();
        let result = serde_wasm_bindgen::to_value(&ImportSourceInvokeArgs {
            request: ImportSourceRequest {
                folder_path: &selected,
            },
        })
        .map_err(|_| JsValue::NULL)
        .and_then(|args| Ok(args));
        let result = match result {
            Ok(args) => invoke("start_review", args).await,
            Err(value) => Err(value),
        };
        busy.set(false);
        match result {
            Ok(value) => match serde_wasm_bindgen::from_value::<ReviewState>(value) {
                Ok(state) => {
                    review_state.set(state);
                    match invoke("next_review_item", JsValue::NULL).await {
                        Ok(value) => match serde_wasm_bindgen::from_value::<ReviewItem>(value) {
                            Ok(item) => {
                                review_tags.set(item.tags.join(", "));
                                import_date
                                    .set(item.effective_import_date.clone().unwrap_or_default());
                                review_item.set(item);
                                step.set("review".into());
                            }
                            Err(_) => {
                                error.set("The review item returned an unexpected response.".into())
                            }
                        },
                        Err(value) => {
                            error.set(command_error(value, "Could not load the next review item."))
                        }
                    }
                }
                Err(_) => error.set("The review session returned an unexpected response.".into()),
            },
            Err(value) => error.set(command_error(
                value,
                "Could not start the safe review session.",
            )),
        }
    };

    let skip_item = move |_| async move {
        let Some(candidate_id) = review_item().candidate_id else {
            return;
        };
        error.set(String::new());
        busy.set(true);
        let tags = review_tags().split(',').map(str::to_owned).collect();
        let result = serde_wasm_bindgen::to_value(&ReviewInvokeArgs {
            request: DecideRequest { candidate_id, tags },
        })
        .map_err(|_| JsValue::NULL)
        .and_then(Ok);
        let result = match result {
            Ok(args) => invoke("skip_review_item", args).await,
            Err(value) => Err(value),
        };
        busy.set(false);
        match result {
            Ok(_) => match invoke("next_review_item", JsValue::NULL).await {
                Ok(value) => match serde_wasm_bindgen::from_value::<ReviewItem>(value) {
                    Ok(item) => {
                        review_tags.set(item.tags.join(", "));
                        import_date.set(item.effective_import_date.clone().unwrap_or_default());
                        review_item.set(item);
                    }
                    Err(_) => {
                        error.set("The next review item returned an unexpected response.".into())
                    }
                },
                Err(value) => error.set(command_error(
                    value,
                    "The item was skipped, but the next item could not be loaded.",
                )),
            },
            Err(value) => error.set(command_error(value, "Could not skip this item.")),
        }
    };

    let import_item = move |_| async move {
        let Some(candidate_id) = review_item().candidate_id else {
            return;
        };
        error.set(String::new());
        busy.set(true);
        let tags = review_tags().split(',').map(str::to_owned).collect();
        let date = import_date();
        let result = serde_wasm_bindgen::to_value(&ReviewInvokeArgs {
            request: ImportRequest {
                candidate_id,
                tags,
                effective_import_date: date,
            },
        })
        .map_err(|_| JsValue::NULL)
        .and_then(Ok);
        let result = match result {
            Ok(args) => invoke("import_review_item", args).await,
            Err(value) => Err(value),
        };
        busy.set(false);
        match result {
            Ok(_) => match invoke("next_review_item", JsValue::NULL).await {
                Ok(value) => match serde_wasm_bindgen::from_value::<ReviewItem>(value) {
                    Ok(item) => {
                        review_tags.set(item.tags.join(", "));
                        import_date.set(item.effective_import_date.clone().unwrap_or_default());
                        review_item.set(item);
                    }
                    Err(_) => {
                        error.set("The next review item returned an unexpected response.".into())
                    }
                },
                Err(value) => error.set(command_error(
                    value,
                    "The item was imported, but the next item could not be loaded.",
                )),
            },
            Err(value) => error.set(command_error(value, "Could not import this item safely.")),
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
    let review_date_origin = review_item()
        .date_origin
        .clone()
        .unwrap_or_else(|| "unavailable".into());

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
                        p { class: "lede", "Your protected library is ready. Starting review reads supported files but never modifies your originals." }
                        div { class: "folder-summary", span { "Protected library" } strong { "{folder}" } }
                        if import_source().state == "ready" {
                            div { class: "folder-summary",
                                span { "Import source" }
                                strong { "{import_source().folder_path.clone().unwrap_or_default()}" }
                                button { r#type: "button", onclick: choose_import_source, disabled: busy(), "Change" }
                            }
                            if review_state().state == "resumable" {
                                button { class: "primary-button", r#type: "button", onclick: start_review, disabled: busy(), "Resume review" }
                            } else {
                                button { class: "primary-button", r#type: "button", onclick: start_review, disabled: busy(), if busy() { "Starting safe review…" } else { "Start review" } }
                            }
                            if !review_state().message.is_empty() { p { class: "privacy-note", "{review_state().message}" } }
                            if review_state().candidate_count > 0 { p { class: "privacy-note", "{review_state().candidate_count} supported item(s) are ready in {review_state().source_path.clone().unwrap_or_default()}." } }
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
                    } else if step() == "review" {
                        p { class: "step-label success-label", "SAFE MEDIA REVIEW" }
                        if review_item().state == "item" || review_item().state == "unavailable" {
                            h2 { "Decide on this item." }
                            p { class: "lede", "Every choice is explicit. Import creates a copy; Skip leaves the original exactly where it is." }
                            div { class: "review-card",
                                div { class: "review-context", strong { "{review_item().filename.clone().unwrap_or_default()}" } small { "{review_item().relative_path.clone().unwrap_or_default()}" } }
                                if review_item().state == "item" {
                                    if review_item().media_type.as_deref() == Some("video") {
                                        video { class: "media-preview", controls: true, src: "{review_item().preview_url.clone().unwrap_or_default()}" }
                                    } else {
                                        img { class: "media-preview", src: "{review_item().preview_url.clone().unwrap_or_default()}", alt: "Preview of {review_item().filename.clone().unwrap_or_default()}" }
                                    }
                                } else { p { class: "error-message", role: "alert", "{review_item().message}" } }
                                label { class: "review-field", "Tags (comma-separated)" input { value: "{review_tags}", oninput: move |event| review_tags.set(event.value()), placeholder: "Family, summer" } }
                                label { class: "review-field", "Import date" input { r#type: "date", value: "{import_date}", oninput: move |event| import_date.set(event.value()) } }
                                p { class: "privacy-note", "Date source: {review_date_origin}. {review_item().message}" }
                                div { class: "decision-actions", button { class: "secondary-button", r#type: "button", onclick: skip_item, disabled: busy(), "Skip" } button { class: "primary-button", r#type: "button", onclick: import_item, disabled: busy() || review_item().state != "item", if busy() { "Saving decision…" } else { "Import copy" } } }
                            }
                        } else {
                            h2 { "Review queue is clear." }
                            p { class: "lede", "{review_item().message}" }
                            button { class: "primary-button", r#type: "button", onclick: move |_| step.set("home".into()), "Back to library home" }
                        }
                    }

                    if !error().is_empty() { p { class: "error-message", role: "alert", "{error}" } }
                }
            }
        }
    }
}
