#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

static CSS: Asset = asset!("/assets/styles.css");

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen(inline_js = r#"
export function warm_video_preview(video) {
  return new Promise((resolve, reject) => {
    let fallback_pause;
    const pause_after_first_frame = () => {
      clearTimeout(fallback_pause);
      video.pause();
      resolve();
    };
    const schedule_pause_after_frame = () => {
      if (typeof video.requestVideoFrameCallback === "function") {
        video.requestVideoFrameCallback(pause_after_first_frame);
      } else {
        requestAnimationFrame(() => requestAnimationFrame(pause_after_first_frame));
      }
    };
    video.addEventListener("playing", schedule_pause_after_frame, { once: true });
    fallback_pause = setTimeout(pause_after_first_frame, 250);
    video.play().catch((error) => {
      clearTimeout(fallback_pause);
      video.removeEventListener("playing", schedule_pause_after_frame);
      reject(error);
    });
  });
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    fn warm_video_preview(video: &web_sys::HtmlVideoElement) -> Result<js_sys::Promise, JsValue>;
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
struct SimilarityThreshold {
    threshold: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetSimilarityThresholdRequest {
    threshold: u32,
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
    metadata: ReviewMetadata,
    tags: Vec<String>,
    preview_url: Option<String>,
    exact_matches: Vec<ExactMatch>,
    similar_matches: Vec<SimilarMatch>,
    visual_comparison_message: Option<String>,
    imported_count: u64,
    skipped_count: u64,
    message: String,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewMetadata {
    file_size_bytes: Option<u64>,
    created_at: Option<String>,
    modified_at: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    captured_at: Option<String>,
    camera: Option<String>,
    orientation: Option<String>,
    #[serde(default)]
    gps: Option<GpsCoordinates>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GpsCoordinates {
    latitude: f64,
    longitude: f64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExactMatch {
    decision: String,
    filename: String,
    relative_path: String,
    decided_at: String,
    tags: Vec<String>,
    preview_url: Option<String>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SimilarMatch {
    candidate_id: i64,
    filename: String,
    decided_at: String,
    tags: Vec<String>,
    preview_url: Option<String>,
    similarity_label: String,
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
#[serde(rename_all = "camelCase")]
struct SubstituteRequest {
    candidate_id: i64,
    replaced_candidate_id: i64,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanLibraryRequest<'a> {
    password: &'a str,
}

#[derive(Serialize)]
struct CleanLibraryInvokeArgs<'a> {
    request: CleanLibraryRequest<'a>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchLibraryResult {
    items: Vec<SearchLibraryItem>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchLibraryItem {
    filename: String,
    media_type: String,
    effective_import_date: Option<String>,
    original_media_date: Option<String>,
    tags: Vec<String>,
    preview_url: Option<String>,
    preview_state: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchLibraryRequest<'a> {
    date_field: &'a str,
    start_date: Option<&'a str>,
    end_date: Option<&'a str>,
    media_type: Option<&'a str>,
    tags: &'a [String],
}
#[derive(Serialize)]
struct SearchLibraryInvokeArgs<'a> {
    request: SearchLibraryRequest<'a>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagSuggestionResult {
    tags: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentTagsResult {
    tags: Vec<String>,
}

#[derive(Serialize)]
struct TagSuggestionRequest<'a> {
    prefix: &'a str,
}

#[derive(Serialize)]
struct TagSuggestionInvokeArgs<'a> {
    request: TagSuggestionRequest<'a>,
}

fn command_error(value: JsValue, fallback: &str) -> String {
    serde_wasm_bindgen::from_value::<CommandError>(value)
        .map(|error| error.message)
        .unwrap_or_else(|_| fallback.to_owned())
}

fn metadata_value(value: Option<String>) -> String {
    value.unwrap_or_else(|| "Not available".into())
}

fn metadata_size(value: Option<u64>) -> String {
    value
        .map(|bytes| format!("{bytes} bytes"))
        .unwrap_or_else(|| "Not available".into())
}

fn metadata_dimensions(width: Option<u32>, height: Option<u32>) -> String {
    match (width, height) {
        (Some(width), Some(height)) => format!("{width} × {height}"),
        _ => "Not available".into(),
    }
}

fn metadata_gps(value: Option<GpsCoordinates>) -> String {
    value
        .map(|coordinates| {
            format!(
                "{:.6}°, {:.6}°",
                coordinates.latitude, coordinates.longitude
            )
        })
        .unwrap_or_else(|| "Not available".into())
}

fn video_target(event: &MediaEvent) -> Option<web_sys::HtmlVideoElement> {
    event
        .data()
        .downcast::<web_sys::Event>()
        .and_then(|event| event.target())
        .and_then(|target| target.dyn_into::<web_sys::HtmlVideoElement>().ok())
}

#[component]
fn VideoCardPreview(preview_url: String) -> Element {
    let mut presentation = use_signal(|| "loading".to_owned());
    let video_class = if matches!(presentation().as_str(), "ready" | "error") {
        "media-card-video media-card-video-ready"
    } else {
        "media-card-video media-card-video-pending"
    };

    rsx! {
        video {
            class: "{video_class}",
            muted: true,
            preload: "auto",
            controls: true,
            src: "{preview_url}",
            onloadeddata: move |event| {
                if presentation() != "loading" {
                    return;
                }
                let Some(video) = video_target(&event) else {
                    presentation.set("error".into());
                    return;
                };
                match warm_video_preview(&video) {
                    Ok(warmed_preview) => {
                        spawn(async move {
                            if wasm_bindgen_futures::JsFuture::from(warmed_preview)
                                .await
                                .is_ok()
                            {
                                presentation.set("ready".into());
                            } else {
                                presentation.set("error".into());
                            }
                        });
                    }
                    Err(_) => {
                        presentation.set("error".into());
                    }
                }
            },
            onerror: move |_| presentation.set("error".into()),
        }
        if presentation() == "loading" {
            p { class: "preview-fallback video-preview-fallback", "Preparing video preview…" }
        } else if presentation() == "error" {
            p { class: "preview-fallback video-preview-fallback", "Video preview unavailable — press Play to view" }
        }
    }
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
        metadata: ReviewMetadata::default(),
        tags: vec![],
        preview_url: None,
        exact_matches: vec![],
        similar_matches: vec![],
        visual_comparison_message: None,
        imported_count: 0,
        skipped_count: 0,
        message: String::new(),
    });
    let mut review_selected_tags = use_signal(Vec::<String>::new);
    let mut similarity_threshold = use_signal(|| 10_u32);
    let mut review_tag_draft = use_signal(String::new);
    let mut recent_review_tags = use_signal(Vec::<String>::new);
    let mut selected_similar_match = use_signal(|| None::<SimilarMatch>);
    let mut import_date = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut recovery_question = use_signal(String::new);
    let mut show_recovery = use_signal(|| false);
    let mut new_password = use_signal(String::new);
    let mut new_confirmation = use_signal(String::new);
    let mut clean_password = use_signal(String::new);
    let mut search_start_date = use_signal(String::new);
    let mut search_end_date = use_signal(String::new);
    let mut search_date_field = use_signal(|| "selected".to_owned());
    let mut search_media_type = use_signal(String::new);
    let mut search_tag_input = use_signal(String::new);
    let mut search_selected_tags = use_signal(Vec::<String>::new);
    let mut tag_suggestions = use_signal(Vec::<String>::new);
    let mut search_items = use_signal(Vec::<SearchLibraryItem>::new);
    let mut search_loading = use_signal(|| false);
    let mut search_actions_open = use_signal(|| false);
    let mut search_dates_expanded = use_signal(|| true);
    let mut search_media_expanded = use_signal(|| true);
    let mut search_tags_expanded = use_signal(|| true);

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
        if step() != "settings" {
            return;
        }
        spawn(async move {
            match invoke("similarity_threshold", JsValue::NULL).await {
                Ok(value) => match serde_wasm_bindgen::from_value::<SimilarityThreshold>(value) {
                    Ok(result) => similarity_threshold.set(result.threshold),
                    Err(_) => error
                        .set("The similarity preference returned an unexpected response.".into()),
                },
                Err(value) => error.set(command_error(
                    value,
                    "Could not load the library similarity preference.",
                )),
            }
        });
    });

    use_effect(move || {
        if step() != "import" {
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

    use_effect(move || {
        if step() != "home" {
            return;
        }
        let start = search_start_date();
        let end = search_end_date();
        let date_field = search_date_field();
        let media = search_media_type();
        let tags = search_selected_tags();
        spawn(async move {
            search_loading.set(true);
            let request = SearchLibraryInvokeArgs {
                request: SearchLibraryRequest {
                    date_field: &date_field,
                    start_date: (!start.is_empty()).then_some(start.as_str()),
                    end_date: (!end.is_empty()).then_some(end.as_str()),
                    media_type: (!media.is_empty()).then_some(media.as_str()),
                    tags: &tags,
                },
            };
            match serde_wasm_bindgen::to_value(&request) {
                Ok(args) => match invoke("search_library", args).await {
                    Ok(value) => {
                        match serde_wasm_bindgen::from_value::<SearchLibraryResult>(value) {
                            Ok(result) => search_items.set(result.items),
                            Err(_) => error
                                .set("The library search returned an unexpected response.".into()),
                        }
                    }
                    Err(value) => error.set(command_error(
                        value,
                        "Could not search the protected library.",
                    )),
                },
                Err(_) => error.set("Could not prepare the library search.".into()),
            }
            search_loading.set(false);
        });
    });

    use_effect(move || {
        let prefix = search_tag_input();
        if prefix.chars().count() < 2 {
            tag_suggestions.set(Vec::new());
            return;
        }
        spawn(async move {
            let request = TagSuggestionInvokeArgs {
                request: TagSuggestionRequest { prefix: &prefix },
            };
            match serde_wasm_bindgen::to_value(&request) {
                Ok(args) => match invoke("suggest_library_tags", args).await {
                    Ok(value) => match serde_wasm_bindgen::from_value::<TagSuggestionResult>(value)
                    {
                        Ok(result) => tag_suggestions.set(result.tags),
                        Err(_) => {
                            error.set("Tag suggestions returned an unexpected response.".into())
                        }
                    },
                    Err(value) => {
                        error.set(command_error(value, "Could not load tag suggestions."))
                    }
                },
                Err(_) => error.set("Could not prepare tag suggestions.".into()),
            }
        });
    });

    use_effect(move || {
        if step() != "review" {
            return;
        }
        spawn(async move {
            match invoke("recent_library_tags", JsValue::NULL).await {
                Ok(value) => match serde_wasm_bindgen::from_value::<RecentTagsResult>(value) {
                    Ok(result) => recent_review_tags.set(result.tags),
                    Err(_) => {
                        error.set("Recent review tags returned an unexpected response.".into())
                    }
                },
                Err(value) => error.set(command_error(value, "Could not load recent review tags.")),
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
                                selected_similar_match.set(None);
                                review_selected_tags.set(item.tags.clone());
                                review_tag_draft.set(String::new());
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
        let tags = review_selected_tags();
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
                        selected_similar_match.set(None);
                        review_selected_tags.set(item.tags.clone());
                        review_tag_draft.set(String::new());
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
        let tags = review_selected_tags();
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
                        selected_similar_match.set(None);
                        review_selected_tags.set(item.tags.clone());
                        review_tag_draft.set(String::new());
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

    let substitute_item = move |_| async move {
        let (Some(candidate_id), Some(matched)) =
            (review_item().candidate_id, selected_similar_match())
        else {
            return;
        };
        error.set(String::new());
        busy.set(true);
        let result = serde_wasm_bindgen::to_value(&ReviewInvokeArgs {
            request: SubstituteRequest {
                candidate_id,
                replaced_candidate_id: matched.candidate_id,
                tags: review_selected_tags(),
                effective_import_date: import_date(),
            },
        })
        .map_err(|_| JsValue::NULL)
        .and_then(Ok);
        let result = match result {
            Ok(args) => invoke("substitute_review_item", args).await,
            Err(value) => Err(value),
        };
        busy.set(false);
        match result {
            Ok(_) => match invoke("next_review_item", JsValue::NULL).await {
                Ok(value) => match serde_wasm_bindgen::from_value::<ReviewItem>(value) {
                    Ok(item) => {
                        selected_similar_match.set(None);
                        review_selected_tags.set(item.tags.clone());
                        review_tag_draft.set(String::new());
                        import_date.set(item.effective_import_date.clone().unwrap_or_default());
                        review_item.set(item);
                    }
                    Err(_) => {
                        error.set("The next review item returned an unexpected response.".into())
                    }
                },
                Err(value) => error.set(command_error(
                    value,
                    "The item was substituted, but the next item could not be loaded.",
                )),
            },
            Err(value) => error.set(command_error(
                value,
                "Could not substitute this managed copy safely.",
            )),
        }
    };

    let update_similarity_threshold = move |threshold: u32| async move {
        error.set(String::new());
        busy.set(true);
        let result = serde_wasm_bindgen::to_value(&ReviewInvokeArgs {
            request: SetSimilarityThresholdRequest { threshold },
        })
        .map_err(|_| JsValue::NULL)
        .and_then(Ok);
        let result = match result {
            Ok(args) => invoke("set_similarity_threshold", args).await,
            Err(value) => Err(value),
        };
        match result {
            Ok(value) => match serde_wasm_bindgen::from_value::<SimilarityThreshold>(value) {
                Ok(result) => similarity_threshold.set(result.threshold),
                Err(_) => error
                    .set("The saved similarity preference returned an unexpected response.".into()),
            },
            Err(value) => error.set(command_error(
                value,
                "Could not save the library similarity preference.",
            )),
        }
        busy.set(false);
    };

    let close_library = move |_| async move {
        error.set(String::new());
        busy.set(true);
        match invoke("lock_library", JsValue::NULL).await {
            Ok(_) => {
                password.set(String::new());
                search_items.set(Vec::new());
                step.set("unlock".into());
            }
            Err(value) => error.set(command_error(
                value,
                "Could not lock the protected library.",
            )),
        }
        busy.set(false);
    };

    let return_to_home = move |_| {
        clean_password.set(String::new());
        error.set(String::new());
        step.set("home".into());
    };

    let clean_library = move |event: FormEvent| async move {
        event.prevent_default();
        error.set(String::new());
        busy.set(true);
        let entered_password = clean_password();
        let result = serde_wasm_bindgen::to_value(&CleanLibraryInvokeArgs {
            request: CleanLibraryRequest {
                password: &entered_password,
            },
        })
        .map_err(|_| JsValue::NULL)
        .and_then(Ok);
        let result = match result {
            Ok(args) => invoke("clean_library", args).await,
            Err(value) => Err(value),
        };
        busy.set(false);
        clean_password.set(String::new());
        match result {
            Ok(_) => {
                import_source.set(ImportSource {
                    state: "missing".into(),
                    folder_path: None,
                });
                review_state.set(ReviewState {
                    state: "none".into(),
                    source_path: None,
                    candidate_count: 0,
                    message: String::new(),
                });
                review_item.set(ReviewItem {
                    state: "empty".into(),
                    candidate_id: None,
                    relative_path: None,
                    filename: None,
                    media_type: None,
                    effective_import_date: None,
                    date_origin: None,
                    metadata: ReviewMetadata::default(),
                    tags: vec![],
                    preview_url: None,
                    exact_matches: vec![],
                    similar_matches: vec![], visual_comparison_message: None,
                    imported_count: 0,
                    skipped_count: 0,
                    message: String::new(),
                });
                step.set("home".into());
            }
            Err(value) => error.set(command_error(
                value,
                "Could not clean managed media. Nothing outside eligible managed date folders was changed.",
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

    let is_onboarding = matches!(step().as_str(), "loading" | "folder" | "stale");
    let shell_class = if is_onboarding {
        "app-shell"
    } else {
        "app-shell selected-library-workspace"
    };
    let review_date_origin = review_item()
        .date_origin
        .clone()
        .unwrap_or_else(|| "unavailable".into());
    let flow_panel_class = if step() == "review" {
        "flow-panel review-flow-panel"
    } else if step() == "home" {
        "flow-panel search-flow-panel"
    } else {
        "flow-panel"
    };
    let flow_wrap_class = if step() == "review" {
        "flow-wrap review-flow-wrap"
    } else if step() == "home" {
        "flow-wrap search-flow-wrap"
    } else {
        "flow-wrap"
    };

    rsx! {
        link { rel: "stylesheet", href: CSS }
        main { class: "{shell_class}",
            if is_onboarding {
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
            }
            section { class: "{flow_panel_class}",
                div { class: "{flow_wrap_class}",
                    if step() == "folder" {
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
                        div { class: "library-search", "data-testid": "library-search",
                            div { class: "library-search-header",
                                h2 { "Your managed media" }
                                div { class: "library-action-menu",
                                    button {
                                        class: "library-action-trigger",
                                        r#type: "button",
                                        "aria-label": "Library actions",
                                        "aria-expanded": "{search_actions_open}",
                                        "aria-controls": "library-action-menu",
                                        onclick: move |_| search_actions_open.set(!search_actions_open()),
                                        "⚙"
                                    }
                                    if search_actions_open() {
                                        div { id: "library-action-menu", class: "library-action-popover", role: "menu",
                                            button { r#type: "button", role: "menuitem", onclick: move |_| { error.set(String::new()); search_actions_open.set(false); step.set("import".into()); }, "Import media" }
                                            button { r#type: "button", role: "menuitem", onclick: move |_| { error.set(String::new()); search_actions_open.set(false); step.set("settings".into()); }, "Library settings" }
                                            button { r#type: "button", role: "menuitem", onclick: close_library, disabled: busy(), if busy() { "Closing…" } else { "Close library" } }
                                            button { r#type: "button", role: "menuitem", onclick: move |_| { search_actions_open.set(false); step.set("danger".into()); }, "Danger zone" }
                                        }
                                    }
                                }
                            }
                            div { class: "library-search-workspace",
                                main { class: "library-results",
                                    if search_date_field() != "selected" || !search_start_date().is_empty() || !search_end_date().is_empty() || !search_media_type().is_empty() || !search_selected_tags().is_empty() {
                                        div { class: "applied-filter-bar", "aria-label": "Applied filters",
                                            strong { "Applied filters" }
                                            if search_date_field() == "original" {
                                                button { class: "applied-filter-chip", r#type: "button", onclick: move |_| search_date_field.set("selected".into()), "Original media date ×" }
                                            }
                                            if !search_start_date().is_empty() {
                                                button { class: "applied-filter-chip", r#type: "button", onclick: move |_| search_start_date.set(String::new()), "From: {search_start_date} ×" }
                                            }
                                            if !search_end_date().is_empty() {
                                                button { class: "applied-filter-chip", r#type: "button", onclick: move |_| search_end_date.set(String::new()), "To: {search_end_date} ×" }
                                            }
                                            if !search_media_type().is_empty() {
                                                button { class: "applied-filter-chip", r#type: "button", onclick: move |_| search_media_type.set(String::new()), "Media: {search_media_type} ×" }
                                            }
                                            for tag in search_selected_tags() {
                                                button { class: "applied-filter-chip", r#type: "button", onclick: move |_| search_selected_tags.with_mut(|tags| tags.retain(|selected| selected != &tag)), "{tag} ×" }
                                            }
                                            button { class: "clear-filters-button", r#type: "button", onclick: move |_| { search_date_field.set("selected".into()); search_start_date.set(String::new()); search_end_date.set(String::new()); search_media_type.set(String::new()); search_tag_input.set(String::new()); search_selected_tags.set(Vec::new()); tag_suggestions.set(Vec::new()); }, "Clear all" }
                                        }
                                    }
                                    if search_date_field() == "original" { p { class: "privacy-note", "Original dates are available only for imports made after this feature was added. Earlier imports remain searchable by selected import date." } }
                                    if search_loading() { p { class: "privacy-note", "Loading imported media…" } }
                                    if !search_loading() && search_items().is_empty() { div { class: "library-empty", "data-testid": "library-empty-state", h3 { "No media has been imported yet." } p { "Use Import media to safely review a folder and create managed copies. Originals are never moved or deleted." } button { class: "primary-button", r#type: "button", onclick: move |_| step.set("import".into()), "Import media" } } }
                                    if !search_loading() && !search_items().is_empty() { div { class: "media-grid", "data-testid": "library-search-grid", for item in search_items() { article { class: "media-card", div { class: "media-card-preview", if item.preview_state == "available" && item.preview_url.is_some() { if item.media_type == "video" { VideoCardPreview { key: "{item.preview_url.clone().unwrap_or_default()}", preview_url: item.preview_url.clone().unwrap_or_default() } } else { img { src: "{item.preview_url.clone().unwrap_or_default()}", alt: "Preview of {item.filename}" } } } else { p { class: "preview-fallback", "Preview unavailable" } } } div { class: "media-card-details", strong { "{item.filename}" } small { "{item.media_type}" } small { "Selected: " {item.effective_import_date.clone().unwrap_or_else(|| "unavailable".into())} } small { "Original: " {item.original_media_date.clone().unwrap_or_else(|| "not recorded".into())} } if !item.tags.is_empty() { small { "Tags: " {item.tags.join(", ")} } } } } } } }
                                }
                                aside { class: "filter-sidebar", "aria-label": "Library filters",
                                    section { class: "filter-section",
                                        button { class: "filter-disclosure", r#type: "button", "aria-expanded": "{search_dates_expanded}", onclick: move |_| search_dates_expanded.set(!search_dates_expanded()), span { "Date" } span { if search_dates_expanded() { "−" } else { "+" } } }
                                        if search_dates_expanded() { div { class: "filter-section-content", label { "Date to search" select { value: "{search_date_field}", onchange: move |event| search_date_field.set(event.value()), option { value: "selected", "Selected import date" } option { value: "original", "Original media date" } } } label { "From" input { r#type: "date", value: "{search_start_date}", oninput: move |event| search_start_date.set(event.value()) } } label { "To" input { r#type: "date", value: "{search_end_date}", oninput: move |event| search_end_date.set(event.value()) } } } }
                                    }
                                    section { class: "filter-section",
                                        button { class: "filter-disclosure", r#type: "button", "aria-expanded": "{search_media_expanded}", onclick: move |_| search_media_expanded.set(!search_media_expanded()), span { "Media type" } span { if search_media_expanded() { "−" } else { "+" } } }
                                        if search_media_expanded() { div { class: "filter-section-content", label { "Media type" select { value: "{search_media_type}", onchange: move |event| search_media_type.set(event.value()), option { value: "", "All media" } option { value: "image", "Images" } option { value: "video", "Videos" } } } } }
                                    }
                                    section { class: "filter-section",
                                        button { class: "filter-disclosure", r#type: "button", "aria-expanded": "{search_tags_expanded}", onclick: move |_| search_tags_expanded.set(!search_tags_expanded()), span { "Tags" } span { if search_tags_expanded() { "−" } else { "+" } } }
                                        if search_tags_expanded() { div { class: "filter-section-content tag-filter", label { "Tags" input { value: "{search_tag_input}", oninput: move |event| search_tag_input.set(event.value()), placeholder: "Type at least two characters", "aria-describedby": "tag-suggestion-help" } } small { id: "tag-suggestion-help", "Suggestions include imported media only." }
                                            if !tag_suggestions().is_empty() { div { class: "tag-suggestions", role: "listbox", for suggestion in tag_suggestions() { button { class: "tag-suggestion", r#type: "button", role: "option", onclick: move |_| { let suggestion = suggestion.clone(); search_selected_tags.with_mut(|tags| { if !tags.contains(&suggestion) { tags.push(suggestion); tags.sort(); } }); search_tag_input.set(String::new()); tag_suggestions.set(Vec::new()); }, "{suggestion}" } } } }
                                            if !search_selected_tags().is_empty() { div { class: "selected-tags", "aria-label": "Selected tags", for tag in search_selected_tags() { button { class: "tag-chip", r#type: "button", onclick: move |_| search_selected_tags.with_mut(|tags| tags.retain(|selected| selected != &tag)), "{tag} ×" } } } }
                                        } }
                                    }
                                }
                            }
                        }
                    } else if step() == "settings" {
                        p { class: "step-label success-label", "LIBRARY SETTINGS" }
                        h2 { "Library settings" }
                        p { class: "lede", "Choose how broadly Photo Handler finds visually similar imported pictures." }
                        div { class: "folder-summary",
                            span { "Protected library" }
                            strong { "{folder}" }
                        }
                        div { class: "similarity-settings", role: "radiogroup", "aria-label": "Similarity matching",
                            strong { "Similar pictures" }
                            small { "Broader matching can show more related photos." }
                            for (threshold, label) in [(8_u32, "Strict"), (10, "Balanced"), (14, "Broad"), (20, "Very Broad")] {
                                label { class: "similarity-preset", "data-selected": "{similarity_threshold() == threshold}",
                                    input { r#type: "radio", name: "similarity-threshold", checked: similarity_threshold() == threshold, onchange: move |_| update_similarity_threshold(threshold), disabled: busy() }
                                    span { "{label} ({threshold})" }
                                }
                            }
                        }
                        button { class: "secondary-button", r#type: "button", onclick: move |_| { error.set(String::new()); step.set("home".into()); }, disabled: busy(), "Back" }
                    } else if step() == "import" {
                        h2 { class: "import-title", "Choose where to import from." }
                        if import_source().state == "ready" {
                            div { class: "folder-summary",
                                span { "Import source" }
                                strong { "{import_source().folder_path.clone().unwrap_or_default()}" }
                                button { r#type: "button", onclick: choose_import_source, disabled: busy(), "Change" }
                            }
                            div { class: "decision-actions",
                                button { class: "secondary-button", r#type: "button", onclick: move |_| step.set("home".into()), disabled: busy(), "Back" }
                                if review_state().state == "resumable" || review_state().state == "complete" {
                                    button { class: "primary-button", r#type: "button", onclick: start_review, disabled: busy(), "Resume review" }
                                } else {
                                    button { class: "primary-button", r#type: "button", onclick: start_review, disabled: busy(), if busy() { "Starting safe review…" } else { "Start review" } }
                                }
                            }
                        } else if import_source().state == "stale" {
                            p { class: "step-label", "IMPORT FOLDER UNAVAILABLE" }
                            p { class: "lede", "The remembered import folder may have moved or be offline. Choose another folder when it is available." }
                            div { class: "folder-summary",
                                span { "Remembered location" }
                                strong { "{import_source().folder_path.clone().unwrap_or_default()}" }
                                button { r#type: "button", onclick: choose_import_source, disabled: busy(), "Change" }
                            }
                            button { class: "secondary-button", r#type: "button", onclick: move |_| step.set("home".into()), disabled: busy(), "Back" }
                        } else {
                            button { class: "folder-picker", r#type: "button", onclick: choose_import_source, disabled: busy(),
                                span { class: "folder-icon", "⌑" }
                                span { strong { if busy() { "Saving import folder…" } else { "Choose import folder" } } small { "Any folder is allowed except your protected library" } }
                                span { class: "arrow", "→" }
                            }
                            button { class: "secondary-button", r#type: "button", onclick: move |_| step.set("home".into()), disabled: busy(), "Back" }
                        }
                    } else if step() == "danger" {
                        p { class: "step-label", "DANGER ZONE" }
                        h2 { "Clean managed debug media" }
                        p { class: "lede", "Eligible managed date folders will be moved to your operating system Trash. Only after every move succeeds, this clears review history and the remembered import folder. If a move fails, you can safely retry." }
                        form { class: "setup-form", onsubmit: clean_library,
                            p { class: "privacy-note", "Original source media, .photo-handler, and unrelated content in this library are preserved." }
                            label { "Current library password" input { r#type: "password", autocomplete: "current-password", value: "{clean_password}", oninput: move |event| clean_password.set(event.value()) } }
                            button { class: "primary-button", r#type: "submit", disabled: busy(), if busy() { "Cleaning managed media…" } else { "Move managed copies to Trash" } }
                            button { class: "secondary-button", r#type: "button", onclick: return_to_home, disabled: busy(), "Back" }
                        }
                    } else if step() == "review" {
                        p { class: "step-label success-label", "SAFE MEDIA REVIEW" }
                        if review_item().state == "item" || review_item().state == "unavailable" {
                            div { class: "review-heading",
                                h2 { "Decide on this item." }
                                button { class: "secondary-button review-close-button", r#type: "button", onclick: move |_| { error.set(String::new()); step.set("home".into()); }, disabled: busy(), "Close review" }
                            }
                            p { class: "lede", "Every choice is explicit. Import creates a copy; Skip leaves the original exactly where it is." }
                            div { class: "review-card",
                                div { class: "review-media-panel",
                                    if review_item().state == "item" {
                                        if review_item().media_type.as_deref() == Some("video") {
                                            video { class: "media-preview", controls: true, src: "{review_item().preview_url.clone().unwrap_or_default()}", onerror: move |_| error.set("This video cannot be played by the embedded browser. Its details remain available and it was not decided automatically; try another supported desktop codec or Skip explicitly.".into()) }
                                        } else {
                                            img { class: "media-preview", src: "{review_item().preview_url.clone().unwrap_or_default()}", alt: "Preview of {review_item().filename.clone().unwrap_or_default()}" }
                                        }
                                    } else {
                                        p { class: "error-message", role: "alert", "{review_item().message}" }
                                    }
                                }
                                    div { class: "review-details",
                                    div { class: "review-context", strong { "{review_item().filename.clone().unwrap_or_default()}" } small { "{review_item().relative_path.clone().unwrap_or_default()}" } }
                                    div { class: "review-metadata", "aria-label": "Media metadata",
                                        strong { "Media details" }
                                        dl {
                                            div { dt { "Type" } dd { "{metadata_value(review_item().media_type.clone())}" } }
                                            div { dt { "Size" } dd { "{metadata_size(review_item().metadata.file_size_bytes)}" } }
                                            div { dt { "Dimensions" } dd { "{metadata_dimensions(review_item().metadata.width, review_item().metadata.height)}" } }
                                            div { dt { "Created" } dd { "{metadata_value(review_item().metadata.created_at.clone())}" } }
                                            div { dt { "Modified" } dd { "{metadata_value(review_item().metadata.modified_at.clone())}" } }
                                            div { dt { "Captured" } dd { "{metadata_value(review_item().metadata.captured_at.clone())}" } }
                                            div { dt { "Camera" } dd { "{metadata_value(review_item().metadata.camera.clone())}" } }
                                            div { dt { "Orientation" } dd { "{metadata_value(review_item().metadata.orientation.clone())}" } }
                                            div { dt { "GPS location" } dd { "{metadata_gps(review_item().metadata.gps.clone())}" } }
                                        }
                                    }
                                    div { class: "review-field",
                                        span { "Tags" }
                                        div { class: "review-tag-editor", "aria-label": "Selected tags",
                                            for tag in review_selected_tags() {
                                                div { class: "review-tag-chip",
                                                    span { "{tag}" }
                                                    button {
                                                        r#type: "button",
                                                        "aria-label": "Remove {tag}",
                                                        onclick: move |_| {
                                                            let mut tags = review_selected_tags();
                                                            tags.retain(|selected| selected != &tag);
                                                            review_selected_tags.set(tags);
                                                        },
                                                        "×"
                                                    }
                                                }
                                            }
                                            input {
                                                value: "{review_tag_draft}",
                                                placeholder: "Type a tag and press Space",
                                                oninput: move |event| {
                                                    let value = event.value();
                                                    let commits = value.split_whitespace().collect::<Vec<_>>();
                                                    let ends_with_space = value.chars().last().is_some_and(char::is_whitespace);
                                                    let draft = if ends_with_space { String::new() } else { commits.last().copied().unwrap_or_default().to_owned() };
                                                    let commit_count = commits.len().saturating_sub((!ends_with_space) as usize);
                                                    if commit_count > 0 {
                                                        let mut tags = review_selected_tags();
                                                        for tag in commits.into_iter().take(commit_count) {
                                                            let tag = tag.to_lowercase();
                                                            if !tag.is_empty() && !tags.contains(&tag) {
                                                                tags.push(tag);
                                                            }
                                                        }
                                                        review_selected_tags.set(tags);
                                                    }
                                                    review_tag_draft.set(draft);
                                                }
                                            }
                                        }
                                        if !recent_review_tags().is_empty() {
                                            div { class: "recent-review-tags", "aria-label": "Recent imported tags",
                                                small { "Recent tags" }
                                                for tag in recent_review_tags() {
                                                    button {
                                                        class: "tag-chip",
                                                        r#type: "button",
                                                        "aria-pressed": "{review_selected_tags().contains(&tag)}",
                                                        onclick: move |_| {
                                                            let mut tags = review_selected_tags();
                                                            if tags.contains(&tag) {
                                                                tags.retain(|selected| selected != &tag);
                                                            } else {
                                                                tags.push(tag.clone());
                                                            }
                                                            review_selected_tags.set(tags);
                                                        },
                                                        "{tag}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    label { class: "review-field", "Import date" input { r#type: "date", value: "{import_date}", oninput: move |event| import_date.set(event.value()) } }
                                    p { class: "privacy-note", "Date source: {review_date_origin}. {review_item().message}" }
                                    if let Some(message) = review_item().visual_comparison_message.clone() { p { class: "privacy-note", "{message}" } }
                                    if !review_item().similar_matches.is_empty() {
                                        div { class: "similar-history", "aria-label": "Possible similar pictures",
                                            strong { "Possible similar pictures" }
                                            for matched in review_item().similar_matches {
                                                div { class: "similar-history-item",
                                                    if let Some(url) = matched.preview_url.clone() { img { src: "{url}", alt: "Managed preview of {matched.filename}" } }
                                                    small { "{matched.similarity_label}: {matched.filename} · {matched.decided_at}" }
                                                    if !matched.tags.is_empty() { small { "Tags: " {matched.tags.join(", ")} } }
                                                    button {
                                                        class: "secondary-button compare-button",
                                                        r#type: "button",
                                                        onclick: move |_| selected_similar_match.set(Some(matched.clone())),
                                                        disabled: busy(),
                                                        "Compare"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if !review_item().exact_matches.is_empty() {
                                        div { class: "exact-history", "aria-label": "Exact file history",
                                            for matched in review_item().exact_matches {
                                                article { class: "exact-history-item",
                                                    strong { if matched.decision == "imported" { "Exact same file previously imported" } else { "Exact same file previously skipped" } }
                                                    div { class: "exact-history-copy",
                                                        if let Some(url) = matched.preview_url.clone() { img { class: "exact-history-preview", src: "{url}", alt: "Managed preview of {matched.filename}" } }
                                                        div { small { "{matched.filename}" } small { "{matched.relative_path}" } small { "Handled: {matched.decided_at}" } if !matched.tags.is_empty() { small { "Tags: {matched.tags.join(\", \")}" } } }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "decision-actions", button { class: "secondary-button", r#type: "button", onclick: skip_item, disabled: busy(), "Skip" } button { class: "primary-button", r#type: "button", onclick: import_item, disabled: busy() || review_item().state != "item", if busy() { "Saving decision…" } else { "Import copy" } } }
                                }
                            }
                        } else if review_item().state == "complete" {
                            h2 { "Review complete." }
                            p { class: "lede", "{review_item().message}" }
                            div { class: "completion-counts", role: "status",
                                p { strong { "{review_item().imported_count}" } " imported" }
                                p { strong { "{review_item().skipped_count}" } " skipped" }
                            }
                            p { class: "privacy-note", "Every original remains at its source. Nothing was deleted or moved." }
                            div { class: "decision-actions",
                                button { class: "secondary-button", r#type: "button", onclick: move |_| step.set("home".into()), "Back to library home" }
                                button { class: "primary-button", r#type: "button", onclick: move |_| step.set("home".into()), "Review another folder" }
                            }
                        } else {
                            h2 { "Review queue is clear." }
                            p { class: "lede", "{review_item().message}" }
                            button { class: "primary-button", r#type: "button", onclick: move |_| step.set("home".into()), "Back to library home" }
                        }
                    }

                    if let Some(matched) = selected_similar_match() {
                        div { class: "comparison-overlay",
                            div {
                                class: "comparison-dialog",
                                role: "dialog",
                                "aria-modal": "true",
                                "aria-labelledby": "comparison-dialog-title",
                                tabindex: "0",
                                onkeydown: move |event| {
                                    if event.key() == Key::Escape {
                                        selected_similar_match.set(None);
                                    }
                                },
                                h2 { id: "comparison-dialog-title", "Compare similar pictures" }
                                p { class: "privacy-note", "Choose only after reviewing both files. Keep Both imports a new copy; Skip leaves the source unchanged." }
                                div { class: "comparison-panels",
                                    article { class: "comparison-panel",
                                        h3 { "Current candidate" }
                                        if let Some(url) = review_item().preview_url.clone() {
                                            img { class: "comparison-preview", src: "{url}", alt: "Preview of current candidate {review_item().filename.clone().unwrap_or_default()}" }
                                        } else {
                                            p { class: "preview-fallback", "Current candidate preview unavailable" }
                                        }
                                        strong { "{review_item().filename.clone().unwrap_or_default()}" }
                                    }
                                    article { class: "comparison-panel",
                                        h3 { "Imported match" }
                                        if let Some(url) = matched.preview_url.clone() {
                                            img { class: "comparison-preview", src: "{url}", alt: "Preview of imported match {matched.filename}" }
                                        } else {
                                            p { class: "preview-fallback", "Imported match preview unavailable" }
                                        }
                                        strong { "{matched.filename}" }
                                        small { "Imported: {matched.decided_at}" }
                                        if !matched.tags.is_empty() { small { "Tags: {matched.tags.join(\", \")}" } }
                                    }
                                }
                                div { class: "comparison-actions",
                                    button {
                                        class: "secondary-button",
                                        r#type: "button",
                                        autofocus: true,
                                        onclick: move |_| selected_similar_match.set(None),
                                        disabled: busy(),
                                        "Close"
                                    }
                                    button { class: "secondary-button", r#type: "button", onclick: skip_item, disabled: busy(), "Skip" }
                                    button { class: "secondary-button", r#type: "button", onclick: substitute_item, disabled: busy() || review_item().state != "item", "Substitute" }
                                    button { class: "primary-button", r#type: "button", onclick: import_item, disabled: busy() || review_item().state != "item", if busy() { "Saving decision…" } else { "Keep Both" } }
                                }
                            }
                        }
                    }

                    if !error().is_empty() { p { class: "error-message", role: "alert", "{error}" } }
                }
            }
        }
    }
}

#[cfg(test)]
mod review_layout_tests {
    const STYLES: &str = include_str!("../assets/styles.css");

    #[test]
    fn review_uses_the_full_window_and_keeps_preview_aspect_ratio() {
        for selector in [
            ".review-flow-panel { height: 100dvh; min-height: 0;",
            ".review-flow-wrap { width: 100%; height: 100%; min-height: 0; display: flex; flex-direction: column; }",
            ".review-card { flex: 1 1 auto; min-height: 0; display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(18rem, 1fr);",
            ".review-media-panel { min-width: 0; min-height: 0; display: flex;",
            ".review-details { min-width: 0; display: flex; flex-direction: column; gap: 1rem;",
            ".media-preview { display: block; width: 100%; max-width: 100%; height: 100%; max-height: 100%;",
            "object-fit: contain;",
        ] {
            assert!(STYLES.contains(selector), "missing review layout rule: {selector}");
        }
    }

    #[test]
    fn review_exposes_metadata_and_space_delimited_tag_chips() {
        let source = include_str!("app.rs");
        for hook in [
            "ReviewMetadata",
            "Media details",
            "GPS location",
            "metadata_gps",
            "Not available",
            "review_selected_tags",
            "review_tag_draft",
            "Type a tag and press Space",
            "Remove {tag}",
            "recent_library_tags",
            "aria-pressed",
        ] {
            assert!(
                source.contains(hook),
                "missing review metadata/tag hook: {hook}"
            );
        }
        for selector in [
            ".review-metadata { display: grid;",
            ".review-tag-editor { display: flex;",
            ".recent-review-tags { display: flex;",
        ] {
            assert!(
                STYLES.contains(selector),
                "missing review metadata/tag style: {selector}"
            );
        }
    }

    #[test]
    fn review_shows_bounded_advisory_exact_history_before_decisions() {
        let source = include_str!("app.rs");
        for hook in [
            "Exact same file previously imported",
            "Exact same file previously skipped",
            "exact-history",
            "review_item().exact_matches",
            "class: \"decision-actions\"",
        ] {
            assert!(source.contains(hook), "missing exact-history hook: {hook}");
        }
        assert!(STYLES.contains(
            ".exact-history { display: grid; gap: .55rem; max-height: 12rem; overflow: auto;"
        ));
    }

    #[test]
    fn review_compares_every_similar_import_without_hiding_actions() {
        let source = include_str!("app.rs");
        for hook in [
            "Possible similar pictures",
            "similar_matches",
            "candidate_id: i64",
            "Compare",
            "selected_similar_match",
            "role: \"dialog\"",
            "\"aria-modal\": \"true\"",
            "Key::Escape",
            "Close",
            "Keep Both",
            "Substitute",
            "substitute_review_item",
            "replaced_candidate_id: matched.candidate_id",
            "Could not substitute this managed copy safely.",
            "Imported match preview unavailable",
            "visual_comparison_message",
            "comparison-panels",
            "class: \"decision-actions\"",
        ] {
            assert!(source.contains(hook), "missing similarity hook: {hook}");
        }
        assert!(STYLES.contains(
            ".similar-history { display: grid; gap: .45rem; max-height: 12rem; overflow: auto;"
        ));
        for rule in [
            ".comparison-overlay { position: fixed; inset: 0;",
            ".comparison-dialog { width: min(100% - 2rem, 72rem); max-height: calc(100dvh - 2rem); overflow: auto;",
            ".comparison-panels { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr));",
            ".comparison-dialog :focus-visible { outline: 3px solid #f2c572;",
            ".comparison-panels { grid-template-columns: 1fr; }",
        ] {
            assert!(STYLES.contains(rule), "missing comparison style: {rule}");
        }
    }

    #[test]
    fn library_settings_exposes_persistent_similarity_presets_without_review_controls() {
        let source = include_str!("app.rs");
        for hook in [
            "step.set(\"settings\".into())",
            "Library settings",
            "Protected library",
            "similarity_threshold",
            "set_similarity_threshold",
            "Strict",
            "Balanced",
            "Broad",
            "Very Broad",
            "role: \"radiogroup\"",
            "name: \"similarity-threshold\"",
            "similarity_threshold.set(result.threshold)",
            "Could not save the library similarity preference.",
        ] {
            assert!(
                source.contains(hook),
                "missing similarity-preset hook: {hook}"
            );
        }
        let review_source = source
            .split("} else if step() == \"review\"")
            .nth(1)
            .and_then(|branch| branch.split("#[cfg(test)]").next())
            .expect("review branch must exist");
        assert!(
            !review_source.contains("similarity-settings"),
            "Review must not contain similarity settings controls"
        );
        for rule in [
            ".similarity-settings { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr));",
            ".similarity-preset[data-selected=\"true\"] {",
            ".similarity-settings { grid-template-columns: 1fr; }",
        ] {
            assert!(STYLES.contains(rule), "missing similarity-preset style: {rule}");
        }
    }

    #[test]
    fn close_review_returns_home_without_recording_a_decision() {
        let source = include_str!("app.rs");
        assert!(source.contains("\"Close review\""));
        assert!(source.contains("step.set(\"home\".into());"));
        assert!(source.contains(
            "onclick: move |_| { error.set(String::new()); step.set(\"home\".into()); }"
        ));
    }

    #[test]
    fn library_search_uses_workspace_menu_and_applied_filter_hooks() {
        let source = include_str!("app.rs");
        for hook in [
            "suggest_library_tags",
            "Selected import date",
            "Original media date",
            "tag-suggestions",
            "selected-tags",
            "Original dates are available only for imports made after this feature was added",
            "library-search-workspace",
            "filter-sidebar",
            "filter-disclosure",
            "search_dates_expanded",
            "search_media_expanded",
            "search_tags_expanded",
            "library-action-trigger",
            "Library actions",
            "library-action-popover",
            "Applied filters",
            "applied-filter-chip",
            "Clear all",
        ] {
            assert!(source.contains(hook), "missing library-search hook: {hook}");
        }
        let home_source = source
            .split("} else if step() == \"home\"")
            .nth(1)
            .and_then(|branch| branch.split("} else if step() == \"settings\"").next())
            .expect("home branch must exist");
        for removed_text in [
            "LIBRARY SEARCH",
            "Browse imported copies. Originals remain untouched.",
        ] {
            assert!(
                !home_source.contains(removed_text),
                "removed library-search header text remains: {removed_text}"
            );
        }
        for rule in [
            ".library-search-workspace { display: grid; grid-template-columns: minmax(13rem, 16rem) minmax(0, 1fr);",
            ".filter-sidebar { grid-column: 1; grid-row: 1; display: grid;",
            ".library-results { grid-column: 2;",
            ".library-action-popover { position: absolute;",
            ".applied-filter-bar { min-height:",
            ".filter-disclosure:focus-visible,",
            ".library-search-workspace { grid-template-columns: 1fr; }",
        ] {
            assert!(
                STYLES.contains(rule),
                "missing library-search style: {rule}"
            );
        }
    }

    #[test]
    fn video_card_waits_for_a_first_frame_and_keeps_a_labelled_fallback() {
        let source = include_str!("app.rs");
        for hook in [
            "fn VideoCardPreview(preview_url: String)",
            "preload: \"auto\"",
            "fn video_target(event: &MediaEvent)",
            "onloadeddata: move |event|",
            "warm_video_preview(&video)",
            "video.requestVideoFrameCallback(pause_after_first_frame)",
            "fallback_pause = setTimeout(pause_after_first_frame, 250)",
            "video.pause();",
            "onerror: move |_| presentation.set(\"error\".into())",
            "Preparing video preview…",
            "Video preview unavailable — press Play to view",
            "media-card-video-pending",
            "media-card-video-ready",
        ] {
            assert!(source.contains(hook), "missing video-card hook: {hook}");
        }

        for rule in [
            ".media-card-video-pending { visibility: hidden; }",
            ".media-card-video-ready { visibility: visible; }",
            ".video-preview-fallback { grid-area: 1 / 1; pointer-events: none; }",
        ] {
            assert!(STYLES.contains(rule), "missing video-card style: {rule}");
        }
    }
}
