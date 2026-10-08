use std::{env, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use danxi_core::{CampusLifeService, CampusLocation, ForumService, ForumTag, HoleSortOrder};
use serde::Deserialize;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod session;

use session::{
    SessionRegistry, WebSession, clear_session_cookie_header, session_cookie_header,
    session_id_from_headers,
};

#[derive(Clone)]
struct ApiState {
    sessions: Arc<SessionRegistry>,
    campus_life: Arc<CampusLifeService>,
    secure_cookies: bool,
}

type Response = axum::response::Response;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "danxi_server=info,tower_http=info".into()),
        )
        .init();

    let allowed_origin =
        env::var("DANXI_ALLOWED_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:1420".to_owned());
    let secure_cookies = env::var("DANXI_COOKIE_SECURE")
        .map(|value| {
            value
                .parse::<bool>()
                .unwrap_or_else(|_| panic!("invalid DANXI_COOKIE_SECURE: expected true or false"))
        })
        .unwrap_or_else(|_| allowed_origin.starts_with("https://"));

    let forum = Arc::new(
        ForumService::new()
            .unwrap_or_else(|error| panic!("failed to initialize community services: {error}")),
    );
    let state = ApiState {
        sessions: Arc::new(SessionRegistry::new(forum)),
        campus_life: Arc::new(
            CampusLifeService::new()
                .unwrap_or_else(|error| panic!("failed to initialize campus services: {error}")),
        ),
        secure_cookies,
    };

    // Periodically drop idle browser sessions.
    tokio::spawn({
        let sweeper = state.sessions.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_secs(300)).await;
                sweeper.sweep();
            }
        }
    });

    let allowed_origin = allowed_origin
        .parse::<HeaderValue>()
        .unwrap_or_else(|error| panic!("invalid DANXI_ALLOWED_ORIGIN: {error}"));

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/session/status", get(session_status))
        .route("/v1/session/login", post(session_login))
        .route("/v1/session/register", post(session_register))
        .route("/v1/session/check-email", post(session_check_email))
        .route("/v1/session/verify-code", post(session_send_verify_code))
        .route("/v1/session/campus/login", post(campus_login))
        .route("/v1/session/campus/logout", post(campus_logout))
        .route("/v1/campus/timetable", get(campus_timetable))
        .route("/v1/campus/library", get(campus_library))
        .route("/v1/campus/dining", get(campus_dining))
        .route("/v1/campus/buses", get(campus_buses))
        .route("/v1/campus/classrooms", get(campus_classrooms))
        .route("/v1/session/logout", post(session_logout))
        .route("/v1/forum/divisions", get(forum_divisions))
        .route("/v1/forum/tags", get(forum_tags))
        .route("/v1/forum/holes", get(forum_holes).post(forum_create_hole))
        .route("/v1/forum/holes/{hole_id}", get(forum_thread))
        .route("/v1/forum/holes/{hole_id}/floors", post(forum_create_floor))
        .route(
            "/v1/forum/floors/{floor_id}/reaction",
            post(forum_react_floor),
        )
        .route(
            "/v1/forum/favorites",
            get(forum_favorite_ids)
                .post(forum_add_favorite)
                .delete(forum_remove_favorite),
        )
        .route("/v1/forum/reports", post(forum_report_floor))
        .route("/v1/evaluation/random", get(evaluation_random))
        .route("/v1/evaluation/search", get(evaluation_search))
        .route(
            "/v1/evaluation/course-groups/{group_id}",
            get(evaluation_course_group),
        )
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(allowed_origin)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([header::ACCEPT, header::CONTENT_TYPE])
                .allow_credentials(true),
        )
        .layer(TraceLayer::new_for_http());

    let address = env::var("DANXI_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".to_owned());
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .unwrap_or_else(|error| panic!("failed to bind Web API gateway at {address}: {error}"));

    tracing::info!(%address, "DanXi Web API gateway started");
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|error| panic!("Web API gateway stopped unexpectedly: {error}"));
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginBody {
    email: String,
    password: String,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterBody {
    email: String,
    password: String,
    verification_code: String,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmailBody {
    email: String,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn session_register(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<RegisterBody>,
) -> Response {
    let (session_id, web_session, created) = match resolve_or_create(&state, &headers) {
        Ok(created) => created,
        Err(error) => return error_response(&error),
    };
    let manager = web_session.manager();

    let campus = web_session.campus().await;
    match manager
        .register(
            &body.email,
            &body.password,
            &body.verification_code,
            Some(campus.as_ref()),
            body.use_webvpn,
        )
        .await
    {
        Ok(status) => match authenticated_session_id(&state, session_id, web_session, created) {
            Ok(session_id) => (
                [(
                    header::SET_COOKIE,
                    session_cookie_header(&session_id, state.secure_cookies),
                )],
                Json(status),
            )
                .into_response(),
            Err(error) => error_response(&error),
        },
        Err(error) => {
            if created {
                state.sessions.remove(Some(&session_id));
            }
            error_response(&error)
        }
    }
}

async fn session_check_email(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<EmailBody>,
) -> Response {
    let web_session = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref());
    let result = if let Some(web_session) = web_session {
        let manager = web_session.manager();
        let campus = web_session.campus().await;
        manager
            .check_register_status(&body.email, Some(campus.as_ref()), body.use_webvpn)
            .await
    } else {
        let manager = state.sessions.anonymous_manager();
        manager
            .check_register_status(&body.email, None, body.use_webvpn)
            .await
    };
    match result {
        Ok(registered) => Json(serde_json::json!({ "registered": registered })).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn session_send_verify_code(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<EmailBody>,
) -> Response {
    let web_session = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref());
    let result = if let Some(web_session) = web_session {
        let manager = web_session.manager();
        let campus = web_session.campus().await;
        manager
            .send_verification_code(&body.email, Some(campus.as_ref()), body.use_webvpn)
            .await
    } else {
        let manager = state.sessions.anonymous_manager();
        manager
            .send_verification_code(&body.email, None, body.use_webvpn)
            .await
    };
    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

async fn session_login(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<LoginBody>,
) -> Response {
    let (session_id, web_session, created) = match resolve_or_create(&state, &headers) {
        Ok(created) => created,
        Err(error) => return error_response(&error),
    };
    let manager = web_session.manager();

    let campus = web_session.campus().await;
    match manager
        .login(
            &body.email,
            &body.password,
            Some(campus.as_ref()),
            body.use_webvpn,
        )
        .await
    {
        Ok(status) => match authenticated_session_id(&state, session_id, web_session, created) {
            Ok(session_id) => (
                [(
                    header::SET_COOKIE,
                    session_cookie_header(&session_id, state.secure_cookies),
                )],
                Json(status),
            )
                .into_response(),
            Err(error) => error_response(&error),
        },
        Err(error) => {
            if created {
                state.sessions.remove(Some(&session_id));
            }
            error_response(&error)
        }
    }
}

async fn session_logout(State(state): State<ApiState>, headers: axum::http::HeaderMap) -> Response {
    state
        .sessions
        .remove(session_id_from_headers(&headers).as_deref());
    (
        [(
            header::SET_COOKIE,
            clear_session_cookie_header(state.secure_cookies),
        )],
        StatusCode::NO_CONTENT,
    )
        .into_response()
}

#[derive(Deserialize)]
struct CommunityNetworkQuery {
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
    #[serde(default = "default_validate_session")]
    validate: bool,
}

async fn session_status(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<CommunityNetworkQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return Json(danxi_core::SessionStatus {
            community_logged_in: false,
            community_user: None,
            campus_logged_in: false,
            campus_id: None,
            campus_name: None,
        })
        .into_response();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    if !query.validate {
        let base = match manager.local_session_status() {
            Ok(status) => status,
            Err(error) => return error_response(&error),
        };
        let campus_status = match campus.local_status() {
            Ok(status) => status,
            Err(error) => return error_response(&error),
        };
        return Json(danxi_core::merge_campus_status(base, &campus_status)).into_response();
    }
    let (base, campus_status) = tokio::join!(
        manager.session_status(Some(campus.as_ref()), query.use_webvpn),
        campus.status(),
    );
    let base = match base {
        Ok(status) => status,
        Err(error) => return error_response(&error),
    };
    let campus_status = match campus_status {
        Ok(status) => status,
        Err(error) => return error_response(&error),
    };
    Json(danxi_core::merge_campus_status(base, &campus_status)).into_response()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CampusLoginBody {
    id: String,
    password: String,
    #[serde(default)]
    is_graduate: bool,
}

async fn campus_login(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CampusLoginBody>,
) -> Response {
    let (session_id, web_session, created) = match resolve_or_create(&state, &headers) {
        Ok(created) => created,
        Err(error) => return error_response(&error),
    };
    let campus = web_session.campus().await;
    match campus
        .login(&body.id, &body.password, body.is_graduate)
        .await
    {
        Ok(status @ danxi_core::CampusLoginResult::Authenticated { .. }) => {
            match authenticated_session_id(&state, session_id, web_session, created) {
                Ok(session_id) => (
                    [(
                        header::SET_COOKIE,
                        session_cookie_header(&session_id, state.secure_cookies),
                    )],
                    Json(status),
                )
                    .into_response(),
                Err(error) => error_response(&error),
            }
        }
        Ok(status @ danxi_core::CampusLoginResult::RequiresSecondFactor { .. }) => {
            campus.clear_pending_second_factor().await;
            if created {
                state.sessions.remove(Some(&session_id));
            }
            Json(status).into_response()
        }
        Err(error) => {
            if created {
                state.sessions.remove(Some(&session_id));
            }
            error_response(&error)
        }
    }
}

async fn campus_timetable(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return campus_unauthorized();
    };
    let campus = web_session.campus().await;
    match campus.load_timetable().await {
        Ok(timetable) => Json(timetable).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn campus_library(State(state): State<ApiState>) -> Response {
    match state.campus_life.library_occupancy().await {
        Ok(occupancy) => Json(occupancy).into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
struct CampusLocationQuery {
    campus: CampusLocation,
}

async fn campus_dining(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<CampusLocationQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return campus_unauthorized();
    };
    let campus = web_session.campus().await;
    match state
        .campus_life
        .dining_crowdedness(campus.as_ref(), query.campus)
        .await
    {
        Ok(crowdedness) => Json(crowdedness).into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
struct BusQuery {
    #[serde(default)]
    holiday: bool,
}

async fn campus_buses(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<BusQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return campus_unauthorized();
    };
    let campus = web_session.campus().await;
    match state
        .campus_life
        .bus_schedule(campus.as_ref(), query.holiday)
        .await
    {
        Ok(buses) => Json(buses).into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
struct ClassroomQuery {
    building: String,
    date: String,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn campus_classrooms(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ClassroomQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return campus_unauthorized();
    };
    let campus = web_session.campus().await;
    match state
        .campus_life
        .empty_classrooms(
            campus.as_ref(),
            &query.building,
            &query.date,
            query.use_webvpn,
        )
        .await
    {
        Ok(classrooms) => Json(classrooms).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn campus_logout(State(state): State<ApiState>, headers: axum::http::HeaderMap) -> Response {
    if let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    {
        if let Err(error) = web_session.campus().await.logout().await {
            return error_response(&error);
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
struct ForumNetworkQuery {
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn forum_divisions(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ForumNetworkQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .load_divisions(Some(campus.as_ref()), query.use_webvpn)
        .await
    {
        Ok(divisions) => Json(divisions).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn forum_tags(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ForumNetworkQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .load_tags(Some(campus.as_ref()), query.use_webvpn)
        .await
    {
        Ok(tags) => Json(tags).into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
struct HoleQuery {
    division_id: Option<i64>,
    size: Option<u32>,
    order: Option<String>,
    before: Option<String>,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn forum_holes(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<HoleQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    let order = match query.order.as_deref() {
        Some("time_created") => HoleSortOrder::LastCreated,
        _ => HoleSortOrder::LastReplied,
    };
    match manager
        .load_holes(
            query.division_id,
            query.size.unwrap_or(10),
            order,
            query.before.as_deref(),
            Some(campus.as_ref()),
            query.use_webvpn,
        )
        .await
    {
        Ok(holes) => Json(holes).into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
struct ForumThreadQuery {
    offset: Option<u32>,
    size: Option<u32>,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn forum_thread(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Path(hole_id): Path<i64>,
    Query(query): Query<ForumThreadQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .load_thread(
            hole_id,
            query.offset.unwrap_or(0),
            query.size.unwrap_or(10),
            Some(campus.as_ref()),
            query.use_webvpn,
        )
        .await
    {
        Ok(thread) => Json(thread).into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateForumHoleBody {
    division_id: i64,
    content: String,
    #[serde(default)]
    tags: Vec<ForumTag>,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn forum_create_hole(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateForumHoleBody>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .create_hole(
            body.division_id,
            &body.content,
            &body.tags,
            Some(campus.as_ref()),
            body.use_webvpn,
        )
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateForumFloorBody {
    content: String,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn forum_create_floor(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Path(hole_id): Path<i64>,
    Json(body): Json<CreateForumFloorBody>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .create_floor(
            hole_id,
            &body.content,
            Some(campus.as_ref()),
            body.use_webvpn,
        )
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReactForumFloorBody {
    reaction: i8,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn forum_react_floor(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Path(floor_id): Path<i64>,
    Json(body): Json<ReactForumFloorBody>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .react_floor(
            floor_id,
            body.reaction,
            Some(campus.as_ref()),
            body.use_webvpn,
        )
        .await
    {
        Ok(floor) => Json(floor).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn forum_favorite_ids(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ForumNetworkQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .favorite_hole_ids(Some(campus.as_ref()), query.use_webvpn)
        .await
    {
        Ok(ids) => Json(ids).into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetForumFavoriteBody {
    hole_id: i64,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn forum_add_favorite(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<SetForumFavoriteBody>,
) -> Response {
    forum_set_favorite(state, headers, body, true).await
}

async fn forum_remove_favorite(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<SetForumFavoriteBody>,
) -> Response {
    forum_set_favorite(state, headers, body, false).await
}

async fn forum_set_favorite(
    state: ApiState,
    headers: axum::http::HeaderMap,
    body: SetForumFavoriteBody,
    favorite: bool,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .set_favorite(
            body.hole_id,
            favorite,
            Some(campus.as_ref()),
            body.use_webvpn,
        )
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportForumFloorBody {
    floor_id: i64,
    reason: String,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn forum_report_floor(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<ReportForumFloorBody>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .report_floor(
            body.floor_id,
            &body.reason,
            Some(campus.as_ref()),
            body.use_webvpn,
        )
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

#[derive(Deserialize)]
struct EvaluationSearchQuery {
    query: String,
    page: Option<u32>,
    page_size: Option<u32>,
    #[serde(default = "default_use_webvpn")]
    use_webvpn: bool,
}

async fn evaluation_search(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<EvaluationSearchQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .search_course_groups(
            query.query.trim(),
            query.page.unwrap_or(1),
            query.page_size.unwrap_or(20).clamp(1, 50),
            Some(campus.as_ref()),
            query.use_webvpn,
        )
        .await
    {
        Ok(groups) => Json(groups).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn evaluation_course_group(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Path(group_id): Path<i64>,
    Query(query): Query<CommunityNetworkQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .course_group_detail(group_id, Some(campus.as_ref()), query.use_webvpn)
        .await
    {
        Ok(detail) => Json(detail).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn evaluation_random(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<CommunityNetworkQuery>,
) -> Response {
    let Some(web_session) = state
        .sessions
        .resolve(session_id_from_headers(&headers).as_deref())
    else {
        return unauthorized();
    };
    let manager = web_session.manager();
    let campus = web_session.campus().await;
    match manager
        .random_course_review(Some(campus.as_ref()), query.use_webvpn)
        .await
    {
        Ok(review) => Json(review).into_response(),
        Err(error) => error_response(&error),
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "kind": "auth", "message": "尚未登录旦挞账户" })),
    )
        .into_response()
}

fn campus_unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "kind": "auth", "message": "尚未登录复旦 UIS" })),
    )
        .into_response()
}

fn resolve_or_create(
    state: &ApiState,
    headers: &axum::http::HeaderMap,
) -> Result<(String, Arc<WebSession>, bool), danxi_core::AppError> {
    if let Some(session_id) = session_id_from_headers(headers)
        && let Some(session) = state.sessions.resolve(Some(&session_id))
    {
        return Ok((session_id, session, false));
    }
    let (session_id, session) = state.sessions.create()?;
    Ok((session_id, session, true))
}

fn authenticated_session_id(
    state: &ApiState,
    session_id: String,
    session: Arc<WebSession>,
    created: bool,
) -> Result<String, danxi_core::AppError> {
    if created {
        Ok(session_id)
    } else {
        state.sessions.rotate(&session_id, session)
    }
}

fn default_use_webvpn() -> bool {
    true
}

fn default_validate_session() -> bool {
    true
}

fn error_response(error: &danxi_core::AppError) -> Response {
    let status = match error {
        danxi_core::AppError::Auth(_) => StatusCode::UNAUTHORIZED,
        danxi_core::AppError::Validation(_) => StatusCode::BAD_REQUEST,
        danxi_core::AppError::EnhancedAuth(_) => StatusCode::PRECONDITION_REQUIRED,
        danxi_core::AppError::Upstream(_) => StatusCode::BAD_GATEWAY,
        danxi_core::AppError::Network(_) => StatusCode::BAD_GATEWAY,
        danxi_core::AppError::Configuration(_) | danxi_core::AppError::Storage(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (status, Json(error)).into_response()
}
