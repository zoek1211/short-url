use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use shared::{CreateLinkRequest, CreateLinkResponse, ErrorResponse};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct Config {
    pub public_base_url: String,
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
        .route("/{number}", get(redirect))
        .with_state(AppState { pool, config })
}

enum ApiError {
    BadRequest(&'static str),
    NotFound,
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
                Json(ErrorResponse { message: message.into() }),
            )
                .into_response(),
            Self::NotFound => StatusCode::NOT_FOUND.into_response(),
            Self::Internal(e) => {
                tracing::error!("database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { message: "서버 오류가 발생했습니다.".into() }),
                )
                    .into_response()
            }
        }
    }
}

fn validate_original_url(url: &str) -> Result<(), ApiError> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        Err(ApiError::BadRequest("원본 URL은 http 또는 https로 시작해야 합니다."))
    }
}

async fn create_link(
    State(state): State<AppState>,
    Json(req): Json<CreateLinkRequest>,
) -> Result<(StatusCode, Json<CreateLinkResponse>), ApiError> {
    let original_url = req.url.trim();
    validate_original_url(original_url)?;

    let number: i64 = sqlx::query_scalar(
        "INSERT INTO links (original_url) VALUES ($1) RETURNING number",
    )
    .bind(original_url)
    .fetch_one(&state.pool)
    .await?;

    let base = state.config.public_base_url.trim_end_matches('/');
    Ok((
        StatusCode::CREATED,
        Json(CreateLinkResponse { number, short_url: format!("{base}/{number}") }),
    ))
}

async fn redirect(
    State(state): State<AppState>,
    Path(number): Path<String>,
) -> Result<Response, ApiError> {
    let number: i64 = number.parse().map_err(|_| ApiError::NotFound)?;
    let url: Option<String> =
        sqlx::query_scalar("SELECT original_url FROM links WHERE number = $1")
            .bind(number)
            .fetch_optional(&state.pool)
            .await?;
    let url = url.ok_or(ApiError::NotFound)?;
    Ok((StatusCode::FOUND, [(header::LOCATION, url)]).into_response())
}
