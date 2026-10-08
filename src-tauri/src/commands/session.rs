use danxi_core::{
    AppError, CampusBus, CampusCredentials, CampusLifeService, CampusLocation, CampusLoginResult,
    CampusSession, CampusStatus, DiningCrowdedness, EmptyClassroom, EvaluationCourseDetail,
    EvaluationCourseGroup, EvaluationReview, ForumDivision, ForumFloor, ForumHole, ForumTag,
    ForumThreadPage, HoleSortOrder, LibraryOccupancy, SessionManager, SessionStatus,
};
use serde::Deserialize;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::{
    AppHandle, Manager, State, Url, WebviewUrl, WebviewWindowBuilder, webview::PageLoadEvent,
};

const DINING_URL: &str = "https://my.fudan.edu.cn/simple_list/stqk";
const CAMPUS_LOGIN_AUTH_WINDOW_LABEL: &str = "campus-login-second-factor";
const DINING_AUTH_WINDOW_LABEL: &str = "campus-dining-enhanced-auth";

#[derive(Clone, Copy)]
enum EnhancedAuthPurpose {
    CampusLogin { generation: u64 },
    Dining,
}

impl EnhancedAuthPurpose {
    fn window_label(self) -> &'static str {
        match self {
            Self::CampusLogin { .. } => CAMPUS_LOGIN_AUTH_WINDOW_LABEL,
            Self::Dining => DINING_AUTH_WINDOW_LABEL,
        }
    }

    fn missing_cookie_message(self) -> &'static str {
        match self {
            Self::CampusLogin { .. } => "未读取到教务系统登录状态",
            Self::Dining => "未读取到食堂服务登录状态",
        }
    }
}

#[tauri::command]
pub async fn community_login(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: LoginRequest,
) -> Result<SessionStatus, danxi_core::AppError> {
    manager
        .login(
            &request.email,
            &request.password,
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub verification_code: String,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailRequest {
    pub email: String,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[tauri::command]
pub async fn community_register(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: RegisterRequest,
) -> Result<SessionStatus, danxi_core::AppError> {
    manager
        .register(
            &request.email,
            &request.password,
            &request.verification_code,
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn community_check_email(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: EmailRequest,
) -> Result<bool, danxi_core::AppError> {
    manager
        .check_register_status(&request.email, Some(campus.inner()), request.use_webvpn)
        .await
}

#[tauri::command]
pub async fn community_send_verify_code(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: EmailRequest,
) -> Result<(), danxi_core::AppError> {
    manager
        .send_verification_code(&request.email, Some(campus.inner()), request.use_webvpn)
        .await
}

#[tauri::command]
pub async fn community_logout(
    manager: State<'_, SessionManager>,
) -> Result<(), danxi_core::AppError> {
    manager.logout().await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampusLoginRequest {
    pub id: String,
    pub password: String,
    #[serde(default)]
    pub is_graduate: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiningRequest {
    pub campus: CampusLocation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusRequest {
    #[serde(default)]
    pub holiday: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyClassroomRequest {
    pub building: String,
    pub date: String,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[tauri::command]
pub async fn campus_login(
    campus: State<'_, CampusSession>,
    request: CampusLoginRequest,
) -> Result<CampusLoginResult, danxi_core::AppError> {
    campus
        .login(&request.id, &request.password, request.is_graduate)
        .await
}

#[tauri::command]
pub async fn load_timetable(
    campus: State<'_, CampusSession>,
) -> Result<danxi_core::Timetable, danxi_core::AppError> {
    campus.load_timetable().await
}

#[tauri::command]
pub async fn load_library_occupancy(
    campus_life: State<'_, CampusLifeService>,
) -> Result<Vec<LibraryOccupancy>, danxi_core::AppError> {
    campus_life.library_occupancy().await
}

#[tauri::command]
pub async fn load_dining_crowdedness(
    campus_life: State<'_, CampusLifeService>,
    campus: State<'_, CampusSession>,
    request: DiningRequest,
) -> Result<DiningCrowdedness, danxi_core::AppError> {
    campus_life
        .dining_crowdedness(campus.inner(), request.campus)
        .await
}

#[tauri::command]
pub async fn complete_campus_second_factor(
    app: AppHandle,
    campus: State<'_, CampusSession>,
) -> Result<CampusStatus, AppError> {
    let context = campus.second_factor_context().await?;
    let generation = context.generation;
    let result = run_enhanced_auth_window(
        app,
        context.credentials,
        context.login_url,
        context.target_host,
        context.identity_url,
        EnhancedAuthPurpose::CampusLogin { generation },
    )
    .await;
    if result.is_err() {
        campus.cancel_second_factor(generation).await;
    }
    result?.ok_or_else(|| AppError::Configuration("校园二次验证未返回登录状态".to_owned()))
}

#[tauri::command]
pub async fn begin_dining_enhanced_auth(
    app: AppHandle,
    campus: State<'_, CampusSession>,
) -> Result<(), AppError> {
    let credentials = campus.credentials_for_enhanced_auth()?;
    let (login_url, target_host, identity_url) = campus.enhanced_auth_context(DINING_URL).await?;
    run_enhanced_auth_window(
        app,
        credentials,
        login_url,
        target_host,
        identity_url,
        EnhancedAuthPurpose::Dining,
    )
    .await?;
    Ok(())
}

async fn run_enhanced_auth_window(
    app: AppHandle,
    credentials: CampusCredentials,
    login_url: String,
    target_host: String,
    identity_url: String,
    purpose: EnhancedAuthPurpose,
) -> Result<Option<CampusStatus>, AppError> {
    if let Some(window) = app.get_webview_window(purpose.window_label()) {
        let _ = window.set_focus();
        return Err(AppError::Configuration("复旦双因素认证正在进行".to_owned()));
    }

    let external_url = login_url
        .parse()
        .map_err(|_| AppError::Configuration("双因素认证地址无效".to_owned()))?;
    let identity_host = Url::parse(&identity_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .ok_or_else(|| AppError::Configuration("统一认证地址无效".to_owned()))?;
    let username = serde_json::to_string(&credentials.id)
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    let password = serde_json::to_string(&credentials.password)
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    let autofill_script = format!(
        r#"setTimeout(() => {{
          try {{
            const usernameInput = document.getElementById('login-username');
            const passwordInput = document.getElementById('login-password');
            if (!usernameInput || !passwordInput) return;
            usernameInput.value = {username};
            usernameInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
            passwordInput.value = {password};
            passwordInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
            document.querySelector('.el-button.content_submit')?.click();
          }} catch (_) {{}}
        }}, 800);"#
    );

    let (sender, receiver) =
        tokio::sync::oneshot::channel::<Result<Option<CampusStatus>, AppError>>();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let completed = Arc::new(AtomicBool::new(false));
    let app_for_load = app.clone();
    let target_host_for_load = target_host.clone();
    let identity_host_for_load = identity_host.clone();
    let identity_url_for_load = identity_url.clone();
    let sender_for_load = sender.clone();
    let completed_for_load = completed.clone();
    let autofill_done = Arc::new(AtomicBool::new(false));
    let autofill_done_for_load = autofill_done.clone();

    let window = WebviewWindowBuilder::new(
        &app,
        purpose.window_label(),
        WebviewUrl::External(external_url),
    )
    .title("复旦双因素认证")
    .inner_size(520.0, 720.0)
    .min_inner_size(420.0, 600.0)
    .center()
    .on_page_load(move |webview, payload| {
        if payload.event() != PageLoadEvent::Finished {
            return;
        }
        let url = payload.url();
        if url.host_str() == Some(identity_host_for_load.as_str())
            && !autofill_done_for_load.swap(true, Ordering::AcqRel)
        {
            let _ = webview.eval(autofill_script.clone());
            return;
        }
        if url.host_str() != Some(target_host_for_load.as_str())
            || completed_for_load.swap(true, Ordering::AcqRel)
        {
            return;
        }

        let target_url = url.clone();
        let app = app_for_load.clone();
        let sender = sender_for_load.clone();
        let identity_url = identity_url_for_load.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<Option<CampusStatus>, AppError> {
                let target_cookies = webview
                    .cookies_for_url(target_url.clone())
                    .map_err(|error| AppError::Storage(error.to_string()))?;
                let parsed_identity_url = identity_url
                    .parse()
                    .map_err(|_| AppError::Configuration("统一认证地址无效".to_owned()))?;
                let identity_cookies = webview
                    .cookies_for_url(parsed_identity_url)
                    .map_err(|error| AppError::Storage(error.to_string()))?;
                let target_headers = target_cookies
                    .into_iter()
                    .map(|cookie| cookie.to_string())
                    .collect::<Vec<_>>();
                let identity_headers = identity_cookies
                    .into_iter()
                    .map(|cookie| cookie.to_string())
                    .collect::<Vec<_>>();
                if target_headers.is_empty() {
                    return Err(AppError::Auth(purpose.missing_cookie_message().to_owned()));
                }

                let campus = app.state::<CampusSession>();
                tauri::async_runtime::block_on(async {
                    match purpose {
                        EnhancedAuthPurpose::CampusLogin { generation } => campus
                            .complete_second_factor(
                                generation,
                                target_url.as_str(),
                                &target_headers,
                                &identity_headers,
                            )
                            .await
                            .map(Some),
                        EnhancedAuthPurpose::Dining => {
                            campus
                                .import_enhanced_auth_cookies(target_url.as_str(), &target_headers)
                                .await?;
                            campus
                                .import_enhanced_auth_cookies(&identity_url, &identity_headers)
                                .await?;
                            Ok(None)
                        }
                    }
                })
            })();
            if let Ok(mut slot) = sender.lock()
                && let Some(sender) = slot.take()
            {
                let _ = sender.send(result);
            }
            let _ = webview.close();
        });
    })
    .build()
    .map_err(|error| AppError::Configuration(error.to_string()))?;

    let sender_for_close = sender.clone();
    let completed_for_close = completed.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed)
            && !completed_for_close.load(Ordering::Acquire)
            && let Ok(mut slot) = sender_for_close.lock()
            && let Some(sender) = slot.take()
        {
            let _ = sender.send(Err(AppError::Auth("双因素认证未完成".to_owned())));
        }
    });

    receiver
        .await
        .map_err(|_| AppError::Auth("双因素认证窗口意外关闭".to_owned()))?
}

#[tauri::command]
pub async fn load_campus_buses(
    campus_life: State<'_, CampusLifeService>,
    campus: State<'_, CampusSession>,
    request: BusRequest,
) -> Result<Vec<CampusBus>, danxi_core::AppError> {
    campus_life
        .bus_schedule(campus.inner(), request.holiday)
        .await
}

#[tauri::command]
pub async fn load_empty_classrooms(
    campus_life: State<'_, CampusLifeService>,
    campus: State<'_, CampusSession>,
    request: EmptyClassroomRequest,
) -> Result<Vec<EmptyClassroom>, danxi_core::AppError> {
    campus_life
        .empty_classrooms(
            campus.inner(),
            &request.building,
            &request.date,
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn campus_logout(campus: State<'_, CampusSession>) -> Result<(), danxi_core::AppError> {
    campus.logout().await
}

#[tauri::command]
pub async fn session_status(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: SessionStatusRequest,
) -> Result<SessionStatus, danxi_core::AppError> {
    if !request.validate {
        return Ok(danxi_core::merge_campus_status(
            manager.local_session_status()?,
            &campus.local_status()?,
        ));
    }

    let (base, campus_status) = tokio::join!(
        manager.session_status(Some(campus.inner()), request.use_webvpn),
        campus.status(),
    );
    Ok(danxi_core::merge_campus_status(base?, &campus_status?))
}

#[tauri::command]
pub async fn search_forum_floors(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: ForumSearchRequest,
) -> Result<danxi_core::ForumSearchPage, danxi_core::AppError> {
    manager
        .search_floors(
            &request.query,
            request.offset,
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumSearchRequest {
    pub query: String,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[tauri::command]
pub async fn load_forum_holes(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: HoleListRequest,
) -> Result<Vec<ForumHole>, danxi_core::AppError> {
    manager
        .load_holes(
            request.division_id,
            request.size.unwrap_or(10),
            order(request.order),
            request.before.as_deref(),
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn load_forum_thread(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: ForumThreadRequest,
) -> Result<ForumThreadPage, danxi_core::AppError> {
    manager
        .load_thread(
            request.hole_id,
            request.offset.unwrap_or(0),
            request.size.unwrap_or(10),
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn load_forum_divisions(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: CommunityNetworkRequest,
) -> Result<Vec<ForumDivision>, danxi_core::AppError> {
    manager
        .load_divisions(Some(campus.inner()), request.use_webvpn)
        .await
}

#[tauri::command]
pub async fn load_forum_floor(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: ForumFloorRequest,
) -> Result<danxi_core::ForumFloor, danxi_core::AppError> {
    manager
        .load_floor(request.floor_id, Some(campus.inner()), request.use_webvpn)
        .await
}

#[tauri::command]
pub async fn load_forum_tags(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: CommunityNetworkRequest,
) -> Result<Vec<ForumTag>, danxi_core::AppError> {
    manager
        .load_tags(Some(campus.inner()), request.use_webvpn)
        .await
}

#[tauri::command]
pub async fn create_forum_hole(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: CreateForumHoleRequest,
) -> Result<(), danxi_core::AppError> {
    manager
        .create_hole(
            request.division_id,
            &request.content,
            &request.tags,
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn create_forum_floor(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: CreateForumFloorRequest,
) -> Result<(), danxi_core::AppError> {
    manager
        .create_floor(
            request.hole_id,
            &request.content,
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn react_forum_floor(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: ReactForumFloorRequest,
) -> Result<ForumFloor, danxi_core::AppError> {
    manager
        .react_floor(
            request.floor_id,
            request.reaction,
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn load_forum_favorite_ids(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: CommunityNetworkRequest,
) -> Result<Vec<i64>, danxi_core::AppError> {
    manager
        .favorite_hole_ids(Some(campus.inner()), request.use_webvpn)
        .await
}

#[tauri::command]
pub async fn set_forum_favorite(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: SetForumFavoriteRequest,
) -> Result<(), danxi_core::AppError> {
    manager
        .set_favorite(
            request.hole_id,
            request.favorite,
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn report_forum_floor(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: ReportForumFloorRequest,
) -> Result<(), danxi_core::AppError> {
    manager
        .report_floor(
            request.floor_id,
            &request.reason,
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn search_evaluation_courses(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: EvaluationSearchRequest,
) -> Result<Vec<EvaluationCourseGroup>, danxi_core::AppError> {
    manager
        .search_course_groups(
            request.query.trim(),
            request.page.unwrap_or(1),
            request.page_size.unwrap_or(20).clamp(1, 50),
            Some(campus.inner()),
            request.use_webvpn,
        )
        .await
}

#[tauri::command]
pub async fn load_evaluation_course(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: EvaluationCourseRequest,
) -> Result<EvaluationCourseDetail, danxi_core::AppError> {
    manager
        .course_group_detail(request.group_id, Some(campus.inner()), request.use_webvpn)
        .await
}

#[tauri::command]
pub async fn load_random_evaluation_review(
    manager: State<'_, SessionManager>,
    campus: State<'_, CampusSession>,
    request: CommunityNetworkRequest,
) -> Result<EvaluationReview, danxi_core::AppError> {
    manager
        .random_course_review(Some(campus.inner()), request.use_webvpn)
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityNetworkRequest {
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusRequest {
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
    #[serde(default = "default_validate_session")]
    pub validate: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HoleListRequest {
    pub division_id: Option<i64>,
    pub size: Option<u32>,
    pub order: Option<String>,
    pub before: Option<String>,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumThreadRequest {
    pub hole_id: i64,
    pub offset: Option<u32>,
    pub size: Option<u32>,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumFloorRequest {
    pub floor_id: i64,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateForumHoleRequest {
    pub division_id: i64,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<ForumTag>,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateForumFloorRequest {
    pub hole_id: i64,
    pub content: String,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactForumFloorRequest {
    pub floor_id: i64,
    pub reaction: i8,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetForumFavoriteRequest {
    pub hole_id: i64,
    pub favorite: bool,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportForumFloorRequest {
    pub floor_id: i64,
    pub reason: String,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationSearchRequest {
    pub query: String,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationCourseRequest {
    pub group_id: i64,
    #[serde(default = "default_use_webvpn")]
    pub use_webvpn: bool,
}

fn default_use_webvpn() -> bool {
    true
}

fn default_validate_session() -> bool {
    true
}

fn order(value: Option<String>) -> HoleSortOrder {
    match value.as_deref() {
        Some("time_created") => HoleSortOrder::LastCreated,
        _ => HoleSortOrder::LastReplied,
    }
}
