use dioxus::prelude::*;
use gloo_net::http::Request;
use shared::{CreateLinkRequest, CreateLinkResponse, ErrorResponse};

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, PartialEq)]
enum Outcome {
    Created(String),
    Failed(String),
}

async fn create_link(url: String) -> Result<String, String> {
    const NETWORK_ERROR: &str = "서버에 연결할 수 없습니다.";
    let res = Request::post("/api/links")
        .json(&CreateLinkRequest { url })
        .map_err(|_| NETWORK_ERROR.to_string())?
        .send()
        .await
        .map_err(|_| NETWORK_ERROR.to_string())?;
    if res.ok() {
        let body: CreateLinkResponse = res.json().await.map_err(|_| NETWORK_ERROR.to_string())?;
        Ok(body.short_url)
    } else {
        let message = match res.json::<ErrorResponse>().await {
            Ok(e) => e.message,
            Err(_) => "알 수 없는 오류가 발생했습니다.".to_string(),
        };
        Err(message)
    }
}

async fn copy_to_clipboard(text: &str) -> bool {
    let Some(window) = web_sys::window() else { return false };
    wasm_bindgen_futures::JsFuture::from(window.navigator().clipboard().write_text(text))
        .await
        .is_ok()
}

#[component]
fn App() -> Element {
    let mut input = use_signal(String::new);
    let mut outcome = use_signal(|| None::<Outcome>);
    let mut copied = use_signal(|| false);

    let submit = move |evt: FormEvent| {
        evt.prevent_default();
        let url = input();
        spawn(async move {
            copied.set(false);
            outcome.set(Some(match create_link(url).await {
                Ok(short_url) => Outcome::Created(short_url),
                Err(message) => Outcome::Failed(message),
            }));
        });
    };

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        h1 { "단축 주소" }
        form { onsubmit: submit,
            input {
                r#type: "text",
                placeholder: "원본 URL을 입력하세요",
                value: "{input}",
                oninput: move |e| input.set(e.value()),
            }
            button { r#type: "submit", "생성" }
        }
        match outcome() {
            Some(Outcome::Created(short_url)) => rsx! {
                div { class: "result",
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
                        "복사"
                    }
                    if copied() {
                        span { class: "copied", "복사됨" }
                    }
                }
            },
            Some(Outcome::Failed(message)) => rsx! { div { class: "error", "{message}" } },
            None => rsx! {},
        }
    }
}
