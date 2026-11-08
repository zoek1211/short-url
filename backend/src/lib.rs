use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use shared::{CreateLinkRequest, CreateLinkResponse, ErrorResponse};
use sqlx::PgPool;
use url::{ParseError, Url};

#[derive(Debug, Clone)]
pub struct Config {
    pub public_base_url: String,
    pub backend_base_url: Option<String>,
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    config: Config,
}

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

pub fn router(pool: PgPool, config: Config) -> Router {
    Router::new()
        .route("/api/links", post(create_link))
        .route("/api/links/{number}", get(show_link))
        .route("/{number}", get(redirect))
        .with_state(AppState { pool, config })
}

enum ApiError {
    BadRequest(&'static str),
    NotFound(String),
    NotFoundJson(&'static str),
    Unserviceable,
    Internal(sqlx::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: message.into(),
                }),
            )
                .into_response(),
            Self::NotFoundJson(message) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: message.into(),
                }),
            )
                .into_response(),
            Self::NotFound(page) => (StatusCode::NOT_FOUND, Html(page)).into_response(),
            Self::Unserviceable => {
                tracing::error!("stored original_url failed to parse");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "서버 오류가 발생했습니다.".into(),
                    }),
                )
                    .into_response()
            }
            Self::Internal(e) => {
                tracing::error!("database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "서버 오류가 발생했습니다.".into(),
                    }),
                )
                    .into_response()
            }
        }
    }
}

const MAX_URL_BYTES: usize = 2048;

fn host_of(base_url: &str) -> Option<String> {
    Url::parse(base_url)
        .ok()?
        .host_str()
        .map(str::to_ascii_lowercase)
}

/// Validates the trimmed 원본 URL and returns nothing; the caller stores the string as-is.
fn validate_original_url(url: &str, config: &Config) -> Result<(), ApiError> {
    if url.len() > MAX_URL_BYTES {
        return Err(ApiError::BadRequest(
            "원본 URL은 2048바이트를 넘을 수 없습니다.",
        ));
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(ApiError::BadRequest(
            "원본 URL에는 공백이나 제어 문자를 넣을 수 없습니다.",
        ));
    }
    let parsed = match Url::parse(url) {
        Ok(parsed) => parsed,
        Err(ParseError::RelativeUrlWithoutBase) => return Err(ApiError::BadRequest(MSG_SCHEME)),
        Err(ParseError::EmptyHost) => return Err(ApiError::BadRequest(MSG_HOST)),
        Err(_) => return Err(ApiError::BadRequest("원본 URL 형식이 올바르지 않습니다.")),
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ApiError::BadRequest(MSG_SCHEME));
    }
    let Some(host) = parsed.host_str().filter(|h| !h.is_empty()) else {
        return Err(ApiError::BadRequest(MSG_HOST));
    };
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ApiError::BadRequest(
            "원본 URL에는 사용자 정보(user:pass@)를 넣을 수 없습니다.",
        ));
    }
    let own_hosts = [
        Some(config.public_base_url.as_str()),
        config.backend_base_url.as_deref(),
    ];
    if own_hosts
        .into_iter()
        .flatten()
        .filter_map(host_of)
        .any(|h| h == host)
    {
        return Err(ApiError::BadRequest(
            "이 서비스의 주소는 원본 URL로 사용할 수 없습니다.",
        ));
    }
    Ok(())
}

const MSG_SCHEME: &str = "원본 URL은 http 또는 https로 시작해야 합니다.";
const MSG_HOST: &str = "원본 URL에 호스트가 없습니다.";

async fn create_link(
    State(state): State<AppState>,
    req: Result<Json<CreateLinkRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreateLinkResponse>), ApiError> {
    let Json(req) = req.map_err(|_| {
        ApiError::BadRequest(
            "요청 본문은 url 필드 하나만 가진 JSON이어야 합니다. 예: {\"url\": \"https://...\"}",
        )
    })?;
    let original_url = req.url.trim();
    validate_original_url(original_url, &state.config)?;

    // Race-safe: a conflicting concurrent insert makes ON CONFLICT wait for that
    // transaction, then do nothing; the follow-up SELECT (READ COMMITTED) sees
    // the committed row. The existing row is never updated. The loop only
    // guards against the row vanishing between the two statements.
    let mut found = None;
    for _ in 0..3 {
        let inserted: Option<i64> = sqlx::query_scalar(
            "INSERT INTO links (original_url) VALUES ($1) \
             ON CONFLICT (original_url) DO NOTHING RETURNING number",
        )
        .bind(original_url)
        .fetch_optional(&state.pool)
        .await?;
        if let Some(number) = inserted {
            found = Some((number, false));
            break;
        }
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT number FROM links WHERE original_url = $1")
                .bind(original_url)
                .fetch_optional(&state.pool)
                .await?;
        if let Some(number) = existing {
            found = Some((number, true));
            break;
        }
    }
    let (number, existing) = found.ok_or_else(|| ApiError::Internal(sqlx::Error::RowNotFound))?;

    let base = state.config.public_base_url.trim_end_matches('/');
    Ok((
        if existing {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        },
        Json(CreateLinkResponse {
            number,
            short_url: format!("{base}/{number}"),
            existing,
        }),
    ))
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn not_found_page(config: &Config) -> String {
    let home = format!("{}/", config.public_base_url.trim_end_matches('/'));
    let home = escape_html(&home);
    format!(
        r#"<!DOCTYPE html>
<html lang="ko">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>단축 주소를 찾을 수 없습니다</title>
<style>
body{{font-family:-apple-system,BlinkMacSystemFont,"Apple SD Gothic Neo","Noto Sans KR",sans-serif;margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#f7f7f8;color:#222}}
main{{max-width:28rem;padding:2rem;text-align:center}}
h1{{font-size:1.4rem;margin:0 0 .75rem}}
p{{margin:0 0 1.5rem;line-height:1.6;color:#555}}
a{{color:#2457d6}}
</style>
</head>
<body>
<main>
<h1>단축 주소를 찾을 수 없습니다</h1>
<p>존재하지 않거나 잘못된 단축 주소입니다. 주소를 다시 확인해 주세요.</p>
<a href="{home}">새 단축 주소 만들기</a>
</main>
</body>
</html>
"#
    )
}

/// A 번호 is ASCII digits only, no leading zero, and a positive i64.
fn parse_number(segment: &str) -> Option<i64> {
    if segment.is_empty()
        || segment.starts_with('0')
        || !segment.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    segment.parse::<i64>().ok().filter(|n| *n >= 1)
}

/// GET /api/links/{number}: the only method a per-link API path accepts.
async fn show_link(
    State(state): State<AppState>,
    Path(segment): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let not_found = || ApiError::NotFoundJson("존재하지 않는 링크입니다.");
    let number = parse_number(&segment).ok_or_else(not_found)?;
    let url: Option<String> =
        sqlx::query_scalar("SELECT original_url FROM links WHERE number = $1")
            .bind(number)
            .fetch_optional(&state.pool)
            .await?;
    let url = url.ok_or_else(not_found)?;
    let base = state.config.public_base_url.trim_end_matches('/');
    Ok(Json(serde_json::json!({
        "number": number,
        "short_url": format!("{base}/{number}"),
        "original_url": url,
    })))
}

async fn redirect(
    State(state): State<AppState>,
    Path(segment): Path<String>,
) -> Result<Response, ApiError> {
    let not_found = || ApiError::NotFound(not_found_page(&state.config));
    let number = parse_number(&segment).ok_or_else(not_found)?;
    let url: Option<String> =
        sqlx::query_scalar("SELECT original_url FROM links WHERE number = $1")
            .bind(number)
            .fetch_optional(&state.pool)
            .await?;
    let url = url.ok_or_else(not_found)?;
    // Stored string is kept verbatim; the header gets the ASCII serialization
    // (punycode host, percent-encoded path/query/fragment).
    let location = Url::parse(&url).map_err(|_| ApiError::Unserviceable)?;
    Ok((
        StatusCode::FOUND,
        [(header::LOCATION, location.as_str().to_owned())],
    )
        .into_response())
}
