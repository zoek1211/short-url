//! Korean/English UI text. The choice is kept in a `lang` cookie so the
//! backend answers API errors and the 404 page in the same language.

use wasm_bindgen::JsCast;
use web_sys::HtmlDocument;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Ko,
    En,
}

pub struct Text {
    pub title: &'static str,
    pub placeholder: &'static str,
    pub create: &'static str,
    pub existing: &'static str,
    pub copy: &'static str,
    pub copied: &'static str,
    pub network_error: &'static str,
    pub unknown_error: &'static str,
    /// Label of the button that switches to the other language.
    pub switch_to: &'static str,
}

const KO: Text = Text {
    title: "단축 주소",
    placeholder: "원본 URL을 입력하세요",
    create: "생성",
    existing: "이미 만들어진 주소예요",
    copy: "복사",
    copied: "복사됨",
    network_error: "서버에 연결할 수 없습니다.",
    unknown_error: "알 수 없는 오류가 발생했습니다.",
    switch_to: "English",
};

const EN: Text = Text {
    title: "Short URL",
    placeholder: "Enter the original URL",
    create: "Create",
    existing: "This URL was already shortened",
    copy: "Copy",
    copied: "Copied",
    network_error: "Cannot reach the server.",
    unknown_error: "An unknown error occurred.",
    switch_to: "한국어",
};

impl Lang {
    pub fn text(self) -> &'static Text {
        match self {
            Lang::Ko => &KO,
            Lang::En => &EN,
        }
    }

    pub fn other(self) -> Self {
        match self {
            Lang::Ko => Lang::En,
            Lang::En => Lang::Ko,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Lang::Ko => "ko",
            Lang::En => "en",
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        match tag
            .trim()
            .split(['-', '_'])
            .next()?
            .to_ascii_lowercase()
            .as_str()
        {
            "ko" => Some(Lang::Ko),
            "en" => Some(Lang::En),
            _ => None,
        }
    }

    /// Same rule as the backend: `lang` cookie, else the first ko/en entry in
    /// the browser's language list, else Korean.
    pub fn detect() -> Self {
        let Some(window) = web_sys::window() else {
            return Lang::Ko;
        };
        let from_cookie = html_document().and_then(|d| d.cookie().ok()).and_then(|c| {
            c.split(';')
                .filter_map(|p| p.trim().strip_prefix("lang="))
                .find_map(Self::from_tag)
        });
        from_cookie
            .or_else(|| {
                window
                    .navigator()
                    .languages()
                    .iter()
                    .filter_map(|v| v.as_string())
                    .find_map(|t| Self::from_tag(&t))
            })
            .unwrap_or(Lang::Ko)
    }

    /// Remembers the choice and updates `<html lang>`.
    pub fn apply(self, remember: bool) {
        let Some(doc) = html_document() else {
            return;
        };
        if remember {
            let _ = doc.set_cookie(&format!(
                "lang={}; path=/; max-age=31536000; samesite=lax",
                self.code()
            ));
        }
        if let Some(root) = doc.document_element() {
            let _ = root.set_attribute("lang", self.code());
        }
    }
}

fn html_document() -> Option<HtmlDocument> {
    web_sys::window()?
        .document()?
        .dyn_into::<HtmlDocument>()
        .ok()
}
