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

mod i18n;
pub use i18n::Lang;
use i18n::{Msg, not_found_page};

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
        .route("/healthz", get(healthz))
        .route("/api/links", post(create_link))
        .route("/api/links/{number}", get(show_link))
        .route("/{number}", get(redirect))
        .with_state(AppState { pool, config })
}

async fn healthz() -> &'static str {
    "ok"
}

enum ApiError {
    BadRequest(Msg),
    NotFoundPage,
    NotFoundJson(Msg),
    Unserviceable,
    Internal(sqlx::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e)
    }
}

impl ApiError {
    fn respond(self, lang: Lang, config: &Config) -> Response {
        let json = |status: StatusCode, msg: Msg| {
            (
                status,
                Json(ErrorResponse {
                    message: msg.text(lang).into(),
                }),
            )
                .into_response()
        };
        match self {
            Self::BadRequest(msg) => json(StatusCode::BAD_REQUEST, msg),
            Self::NotFoundJson(msg) => json(StatusCode::NOT_FOUND, msg),
            Self::NotFoundPage => (
                StatusCode::NOT_FOUND,
                Html(not_found_page(&config.public_base_url, lang)),
            )
                .into_response(),
            Self::Unserviceable => {
                tracing::error!("stored original_url failed to parse");
                json(StatusCode::INTERNAL_SERVER_ERROR, Msg::ServerError)
            }
            Self::Internal(e) => {
                tracing::error!("database error: {e}");
                json(StatusCode::INTERNAL_SERVER_ERROR, Msg::ServerError)
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
        return Err(ApiError::BadRequest(Msg::TooLong));
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(ApiError::BadRequest(Msg::Whitespace));
    }
    let parsed = match Url::parse(url) {
        Ok(parsed) => parsed,
        Err(ParseError::RelativeUrlWithoutBase) => return Err(ApiError::BadRequest(Msg::Scheme)),
        Err(ParseError::EmptyHost) => return Err(ApiError::BadRequest(Msg::NoHost)),
        Err(_) => return Err(ApiError::BadRequest(Msg::Malformed)),
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ApiError::BadRequest(Msg::Scheme));
    }
    let Some(host) = parsed.host_str().filter(|h| !h.is_empty()) else {
        return Err(ApiError::BadRequest(Msg::NoHost));
    };
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ApiError::BadRequest(Msg::Userinfo));
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
        return Err(ApiError::BadRequest(Msg::SelfLink));
    }
    Ok(())
}

async fn create_link(
    State(state): State<AppState>,
    lang: Lang,
    req: Result<Json<CreateLinkRequest>, JsonRejection>,
) -> Response {
    create_link_inner(&state, req)
        .await
        .unwrap_or_else(|e| e.respond(lang, &state.config))
}

async fn create_link_inner(
    state: &AppState,
    req: Result<Json<CreateLinkRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(req) = req.map_err(|_| ApiError::BadRequest(Msg::BadBody))?;
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
    )
        .into_response())
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
    lang: Lang,
    Path(segment): Path<String>,
) -> Response {
    show_link_inner(&state, &segment)
        .await
        .unwrap_or_else(|e| e.respond(lang, &state.config))
}

async fn show_link_inner(state: &AppState, segment: &str) -> Result<Response, ApiError> {
    let not_found = || ApiError::NotFoundJson(Msg::LinkNotFound);
    let number = parse_number(segment).ok_or_else(not_found)?;
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
    }))
    .into_response())
}

async fn redirect(
    State(state): State<AppState>,
    lang: Lang,
    Path(segment): Path<String>,
) -> Response {
    redirect_inner(&state, &segment)
        .await
        .unwrap_or_else(|e| e.respond(lang, &state.config))
}

async fn redirect_inner(state: &AppState, segment: &str) -> Result<Response, ApiError> {
    let not_found = || ApiError::NotFoundPage;
    let number = parse_number(segment).ok_or_else(not_found)?;
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
