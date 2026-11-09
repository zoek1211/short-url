//! Korean/English text for API errors and the 404 page.
//!
//! The language comes from the `lang` cookie the web UI sets when the user
//! clicks the language button; without it, the first `ko`/`en` tag in
//! `Accept-Language` decides; otherwise Korean.

use std::convert::Infallible;

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header, request::Parts},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Ko,
    En,
}

impl Lang {
    fn from_tag(tag: &str) -> Option<Self> {
        let primary = tag.trim().split(['-', '_', ';']).next()?.trim();
        if primary.eq_ignore_ascii_case("ko") {
            Some(Self::Ko)
        } else if primary.eq_ignore_ascii_case("en") {
            Some(Self::En)
        } else {
            None
        }
    }

    pub fn from_headers(headers: &HeaderMap) -> Self {
        let from_cookie = headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|pair| pair.trim().strip_prefix("lang="))
            .find_map(Self::from_tag);
        let from_accept = || {
            headers
                .get_all(header::ACCEPT_LANGUAGE)
                .iter()
                .filter_map(|v| v.to_str().ok())
                .flat_map(|v| v.split(','))
                .find_map(Self::from_tag)
        };
        from_cookie.or_else(from_accept).unwrap_or(Self::Ko)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Lang {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Self::from_headers(&parts.headers))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    TooLong,
    Whitespace,
    Malformed,
    Scheme,
    NoHost,
    Userinfo,
    SelfLink,
    BadBody,
    LinkNotFound,
    ServerError,
}

impl Msg {
    pub fn text(self, lang: Lang) -> &'static str {
        use Lang::*;
        use Msg::*;
        match (self, lang) {
            (TooLong, Ko) => "원본 URL은 2048바이트를 넘을 수 없습니다.",
            (TooLong, En) => "The original URL cannot exceed 2048 bytes.",
            (Whitespace, Ko) => "원본 URL에는 공백이나 제어 문자를 넣을 수 없습니다.",
            (Whitespace, En) => "The original URL cannot contain spaces or control characters.",
            (Malformed, Ko) => "원본 URL 형식이 올바르지 않습니다.",
            (Malformed, En) => "The original URL is not a valid URL.",
            (Scheme, Ko) => "원본 URL은 http 또는 https로 시작해야 합니다.",
            (Scheme, En) => "The original URL must start with http or https.",
            (NoHost, Ko) => "원본 URL에 호스트가 없습니다.",
            (NoHost, En) => "The original URL has no host.",
            (Userinfo, Ko) => "원본 URL에는 사용자 정보(user:pass@)를 넣을 수 없습니다.",
            (Userinfo, En) => "The original URL cannot contain user info (user:pass@).",
            (SelfLink, Ko) => "이 서비스의 주소는 원본 URL로 사용할 수 없습니다.",
            (SelfLink, En) => "This service's own address cannot be used as the original URL.",
            (BadBody, Ko) => {
                "요청 본문은 url 필드 하나만 가진 JSON이어야 합니다. 예: {\"url\": \"https://...\"}"
            }
            (BadBody, En) => {
                "The request body must be JSON with only a url field. Example: {\"url\": \"https://...\"}"
            }
            (LinkNotFound, Ko) => "존재하지 않는 링크입니다.",
            (LinkNotFound, En) => "This link does not exist.",
            (ServerError, Ko) => "서버 오류가 발생했습니다.",
            (ServerError, En) => "A server error occurred.",
        }
    }
}

pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// The 404 page for an unknown or malformed 번호, with a link home and a
/// button that switches language (sets the same cookie as the web UI).
pub fn not_found_page(public_base_url: &str, lang: Lang) -> String {
    let home = escape_html(&format!("{}/", public_base_url.trim_end_matches('/')));
    let (html_lang, title, body, link, other, other_code) = match lang {
        Lang::Ko => (
            "ko",
            "단축 주소를 찾을 수 없습니다",
            "존재하지 않거나 잘못된 단축 주소입니다. 주소를 다시 확인해 주세요.",
            "새 단축 주소 만들기",
            "English",
            "en",
        ),
        Lang::En => (
            "en",
            "Short URL not found",
            "This short URL does not exist or is invalid. Please check the address.",
            "Create a new short URL",
            "한국어",
            "ko",
        ),
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="{html_lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
body{{font-family:-apple-system,BlinkMacSystemFont,"Apple SD Gothic Neo","Noto Sans KR",sans-serif;margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#f7f7f8;color:#222}}
main{{max-width:28rem;padding:2rem;text-align:center}}
h1{{font-size:1.4rem;margin:0 0 .75rem}}
p{{margin:0 0 1.5rem;line-height:1.6;color:#555}}
a{{color:#2457d6}}
.lang{{position:fixed;top:1rem;right:1rem;padding:.35rem .75rem;font-size:.9rem;cursor:pointer}}
</style>
</head>
<body>
<button class="lang" type="button" onclick="document.cookie='lang={other_code}; path=/; max-age=31536000; samesite=lax';location.reload()">{other}</button>
<main>
<h1>{title}</h1>
<p>{body}</p>
<a href="{home}">{link}</a>
</main>
</body>
</html>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn lang(cookie: Option<&str>, accept: Option<&str>) -> Lang {
        let mut h = HeaderMap::new();
        if let Some(c) = cookie {
            h.insert(header::COOKIE, HeaderValue::from_str(c).unwrap());
        }
        if let Some(a) = accept {
            h.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_str(a).unwrap());
        }
        Lang::from_headers(&h)
    }

    #[test]
    fn picks_language() {
        assert_eq!(lang(None, None), Lang::Ko);
        assert_eq!(lang(None, Some("en-US,en;q=0.9")), Lang::En);
        assert_eq!(lang(None, Some("ja,en;q=0.8,ko;q=0.5")), Lang::En);
        assert_eq!(lang(None, Some("ko-KR,en;q=0.5")), Lang::Ko);
        assert_eq!(lang(None, Some("ja")), Lang::Ko);
        assert_eq!(lang(Some("a=1; lang=en"), Some("ko-KR")), Lang::En);
        assert_eq!(lang(Some("lang=ko"), Some("en-US")), Lang::Ko);
        assert_eq!(lang(Some("lang=fr"), Some("en-US")), Lang::En);
    }
}
