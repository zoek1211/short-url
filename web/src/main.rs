use dioxus::prelude::*;
use gloo_net::http::Request;
use shared::{CreateLinkRequest, CreateLinkResponse, ErrorResponse};

mod i18n;
use i18n::Lang;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, PartialEq)]
enum Failure {
    Network,
    Unknown,
    /// Message from the server, already in the language chosen at request time.
    Server(String),
}

#[derive(Clone, PartialEq)]
enum Outcome {
    Created { short_url: String, existing: bool },
    Failed(Failure),
}

async fn create_link(url: String) -> Result<CreateLinkResponse, Failure> {
    let res = Request::post("/api/links")
        .json(&CreateLinkRequest { url })
        .map_err(|_| Failure::Network)?
        .send()
        .await
        .map_err(|_| Failure::Network)?;
    if res.ok() {
        res.json().await.map_err(|_| Failure::Network)
    } else {
        Err(match res.json::<ErrorResponse>().await {
            Ok(e) => Failure::Server(e.message),
            Err(_) => Failure::Unknown,
        })
    }
}

async fn copy_to_clipboard(text: &str) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    wasm_bindgen_futures::JsFuture::from(window.navigator().clipboard().write_text(text))
        .await
        .is_ok()
}

#[component]
fn App() -> Element {
    let mut input = use_signal(String::new);
    let mut outcome = use_signal(|| None::<Outcome>);
    let mut copied = use_signal(|| false);
    let mut lang = use_signal(|| {
        let detected = Lang::detect();
        detected.apply(false);
        detected
    });
    let t = lang().text();

    let submit = move |evt: FormEvent| {
        evt.prevent_default();
        let url = input();
        spawn(async move {
            copied.set(false);
            outcome.set(Some(match create_link(url).await {
                Ok(body) => Outcome::Created {
                    short_url: body.short_url,
                    existing: body.existing,
                },
                Err(failure) => Outcome::Failed(failure),
            }));
        });
    };

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        document::Title { "{t.title}" }
        button {
            class: "lang",
            r#type: "button",
            onclick: move |_| {
                let next = lang().other();
                next.apply(true);
                lang.set(next);
            },
            "{t.switch_to}"
        }
        h1 { "{t.title}" }
        form { onsubmit: submit,
            input {
                r#type: "text",
                placeholder: t.placeholder,
                value: "{input}",
                oninput: move |e| input.set(e.value()),
            }
            button { r#type: "submit", "{t.create}" }
        }
        match outcome() {
            Some(Outcome::Created { short_url, existing }) => rsx! {
                div { class: "result",
                    if existing {
                        span { class: "notice", "{t.existing}" }
                    }
                    a { href: "{short_url}", "{short_url}" }
                    button {
                        onclick: move |_| {
                            let short_url = short_url.clone();
                            spawn(async move {
                                if copy_to_clipboard(&short_url).await {
                                    copied.set(true);
                                    gloo_timers::future::TimeoutFuture::new(2000).await;
                                    copied.set(false);
                                }
                            });
                        },
                        "{t.copy}"
                    }
                    if copied() {
                        span { class: "copied", "{t.copied}" }
                    }
                }
            },
            Some(Outcome::Failed(failure)) => {
                let message = match failure {
                    Failure::Network => t.network_error.to_string(),
                    Failure::Unknown => t.unknown_error.to_string(),
                    Failure::Server(message) => message,
                };
                rsx! { div { class: "error", "{message}" } }
            }
            None => rsx! {},
        }
    }
}
