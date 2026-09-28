use std::{
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use reqwest::{Client, Request, Response, StatusCode, header, redirect::Policy};
use serde::Deserialize;

use crate::{
    AppError, CampusSession, CommunityUser, EvaluationCourseDetail, EvaluationCourseGroup,
    EvaluationRating, EvaluationReview, ForumFloor, ForumFloorPreview, ForumHole, ForumTag,
    ForumThreadPage, HoleSortOrder, SessionStatus, SessionStore, TokenPair,
};

pub const DEFAULT_FORUM_BASE_URL: &str = "https://forum.fduhole.com/api";
pub const DEFAULT_AUTH_BASE_URL: &str = "https://auth.fduhole.com/api";
pub const DEFAULT_DANKE_BASE_URL: &str = "https://danke.fduhole.com/api";
const DIRECT_CONNECT_TEST_URL: &str = "https://forum.fduhole.com";
const ROUTE_UNKNOWN: u8 = 0;
const ROUTE_DIRECT: u8 = 1;
const ROUTE_WEBVPN: u8 = 2;
const ROUTE_CACHE_TTL: Duration = Duration::from_secs(60);

struct RouteCache {
    route: u8,
    checked_at: Option<Instant>,
}

impl Default for RouteCache {
    fn default() -> Self {
        Self {
            route: ROUTE_UNKNOWN,
            checked_at: None,
        }
    }
}

/// Stateless client for the community forum and auth upstreams.
///
/// Tokens are passed per call instead of being held here, so a single instance
/// can be shared by the Tauri runtime and the multi-session web gateway.
pub struct ForumService {
    http: Client,
    forum_base: String,
    auth_base: String,
    danke_base: String,
    route: tokio::sync::Mutex<RouteCache>,
}

impl ForumService {
    pub fn new() -> Result<Self, AppError> {
        // No cookie store: the web gateway shares this client across sessions
        // and must not mix upstream cookies between users.
        let mut builder = Client::builder()
            .redirect(Policy::limited(12))
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .user_agent(format!("DanXi Next/{}", env!("CARGO_PKG_VERSION")));

        if let Ok(proxy_url) = env::var("DANXI_HTTP_PROXY") {
            if !proxy_url.trim().is_empty() {
                let proxy = reqwest::Proxy::all(&proxy_url)
                    .map_err(|_| AppError::Configuration("DANXI_HTTP_PROXY 配置无效".to_owned()))?;
                builder = builder.proxy(proxy);
            }
        }

        Ok(Self {
            http: builder
                .build()
                .map_err(|_| AppError::Configuration("HTTP 客户端初始化失败".to_owned()))?,
            forum_base: env_base_url("DANXI_FORUM_BASE_URL", DEFAULT_FORUM_BASE_URL),
            auth_base: env_base_url("DANXI_AUTH_BASE_URL", DEFAULT_AUTH_BASE_URL),
            danke_base: env_base_url("DANXI_DANKE_BASE_URL", DEFAULT_DANKE_BASE_URL),
            route: tokio::sync::Mutex::new(RouteCache::default()),
        })
    }

    /// Exchange community email/password for an access/refresh token pair.
    pub async fn login(
        &self,
        email: &str,
        password: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<TokenPair, AppError> {
        let request = self
            .http
            .post(format!("{}/login", self.auth_base))
            .timeout(Duration::from_secs(10))
            .json(&serde_json::json!({ "email": email, "password": password }))
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(login_error(status));
        }
        decode_token(response.json::<TokenPair>().await)
    }

    /// Exchange a refresh token for a new token pair.
    pub async fn refresh(
        &self,
        refresh_token: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<TokenPair, AppError> {
        let request = self
            .http
            .post(format!("{}/refresh", self.auth_base))
            .timeout(Duration::from_secs(10))
            .header(header::AUTHORIZATION, format!("Bearer {refresh_token}"))
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(login_error(status));
        }
        decode_token(response.json::<TokenPair>().await)
    }

    /// Check whether an email is already registered, driving the
    /// login-or-register decision in the UI.
    pub async fn check_register_status(
        &self,
        email: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<bool, AppError> {
        #[derive(serde::Deserialize)]
        struct RegisterStatus {
            #[serde(default)]
            registered: Option<bool>,
            #[serde(default)]
            message: Option<String>,
        }

        let request = self
            .http
            .get(format!("{}/verify/email", self.auth_base))
            .timeout(Duration::from_secs(6))
            .query(&[("email", email), ("check", "true")])
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(login_error(status));
        }
        let body: RegisterStatus = response
            .json()
            .await
            .map_err(|_| AppError::Upstream("上游响应格式无法解析".to_owned()))?;
        body.registered
            .ok_or_else(|| AppError::Upstream(body.message.unwrap_or_default()))
    }

    /// Send a one-time verification code to an unregistered email.
    pub async fn send_verification_code(
        &self,
        email: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<(), AppError> {
        let request = self
            .http
            .get(format!("{}/verify/email", self.auth_base))
            .timeout(Duration::from_secs(10))
            .query(&[("email", email)])
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(login_error(status));
        }
        Ok(())
    }

    /// Register a new community account and sign it in immediately.
    pub async fn register(
        &self,
        email: &str,
        password: &str,
        verification_code: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<TokenPair, AppError> {
        let code: i64 = verification_code
            .trim()
            .parse()
            .map_err(|_| AppError::Auth("验证码格式不正确".to_owned()))?;
        let request = self
            .http
            .post(format!("{}/register", self.auth_base))
            .timeout(Duration::from_secs(10))
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "verification": code,
            }))
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(login_error(status));
        }
        decode_token(response.json::<TokenPair>().await)
    }

    /// Fetch the profile of the user behind an access token.
    pub async fn user_profile(
        &self,
        access_token: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<CommunityUser, AppError> {
        let request = self
            .http
            .get(format!("{}/users/me", self.forum_base))
            .bearer_auth(access_token)
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(request_error(status));
        }
        let raw: RawUser = response.json().await?;
        Ok(CommunityUser {
            user_id: raw.user_id.unwrap_or_default(),
            nickname: raw.nickname.unwrap_or_default(),
            is_admin: raw.is_admin.unwrap_or(false),
        })
    }

    /// Load one page of hole listings. The caller owns the token lifecycle.
    pub async fn load_holes(
        &self,
        access_token: &str,
        division_id: Option<i64>,
        size: u32,
        order: HoleSortOrder,
        before: Option<&str>,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<Vec<ForumHole>, AppError> {
        let start_time = before.map(str::to_owned).unwrap_or_else(|| {
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        });
        let size = forum_page_size(size).to_string();
        let request = if let Some(division_id) = division_id {
            self.http
                .get(format!("{}/holes", self.forum_base))
                .bearer_auth(access_token)
                .query(&[
                    ("start_time", start_time.as_str()),
                    ("division_id", &division_id.to_string()),
                    ("length", &size),
                    ("order", order.as_str()),
                ])
        } else {
            self.http
                .get(format!("{}/holes/_homepage", self.forum_base))
                .bearer_auth(access_token)
                .query(&[
                    ("offset", start_time.as_str()),
                    ("size", &size),
                    ("order", order.as_str()),
                ])
        };

        let response = self.execute(request.build()?, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(request_error(status));
        }
        let raw: Vec<RawHole> = response.json().await?;
        Ok(raw.into_iter().map(Into::into).collect())
    }

    /// Load the topic metadata and one page of floors for a readable thread view.
    pub async fn load_thread(
        &self,
        access_token: &str,
        hole_id: i64,
        offset: u32,
        size: u32,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<ForumThreadPage, AppError> {
        let size = forum_page_size(size);
        let hole_request = self
            .http
            .get(format!("{}/holes/{hole_id}", self.forum_base))
            .bearer_auth(access_token)
            .build()?;
        let floors_request = self
            .http
            .get(format!("{}/holes/{hole_id}/floors", self.forum_base))
            .bearer_auth(access_token)
            .query(&[("offset", offset), ("size", size)])
            .build()?;

        let (hole_response, floors_response) = futures::join!(
            self.execute(hole_request, campus, use_webvpn),
            self.execute(floors_request, campus, use_webvpn),
        );
        let hole_response = hole_response?;
        let floors_response = floors_response?;
        if !hole_response.status().is_success() {
            return Err(request_error(hole_response.status()));
        }
        if !floors_response.status().is_success() {
            return Err(request_error(floors_response.status()));
        }

        let hole: ForumHole = hole_response.json::<RawHole>().await?.into();
        let floors = floors_response
            .json::<Vec<RawFloor>>()
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let next_offset =
            (floors.len() == size as usize).then_some(offset.saturating_add(floors.len() as u32));

        Ok(ForumThreadPage {
            hole,
            floors,
            next_offset,
        })
    }

    pub async fn search_course_groups(
        &self,
        access_token: &str,
        query: &str,
        page: u32,
        page_size: u32,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<Vec<EvaluationCourseGroup>, AppError> {
        let request = self
            .http
            .get(format!("{}/v3/course_groups/search", self.danke_base))
            .bearer_auth(access_token)
            .query(&[
                ("query", query),
                ("page", &page.to_string()),
                ("page_size", &page_size.to_string()),
            ])
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(request_error(status));
        }
        let raw: RawCourseSearchResults = response.json().await?;
        Ok(raw.items.into_iter().map(Into::into).collect())
    }

    pub async fn course_group_detail(
        &self,
        access_token: &str,
        group_id: i64,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<EvaluationCourseDetail, AppError> {
        let request = self
            .http
            .get(format!("{}/v3/course_groups/{group_id}", self.danke_base))
            .bearer_auth(access_token)
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(request_error(status));
        }
        let raw: RawCourseGroup = response.json().await?;
        Ok(raw.into_detail())
    }

    pub async fn random_course_review(
        &self,
        access_token: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<EvaluationReview, AppError> {
        let request = self
            .http
            .get(format!("{}/reviews/random", self.danke_base))
            .bearer_auth(access_token)
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(request_error(status));
        }
        let raw: RawRandomCourseReview = response.json().await?;
        Ok(raw.into())
    }

    async fn execute(
        &self,
        request: Request,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<Response, AppError> {
        if !use_webvpn || !crate::webvpn::supports(request.url()) {
            return self.execute_direct(request, use_webvpn).await;
        }

        if self.direct_available().await {
            let direct_request = request
                .try_clone()
                .ok_or_else(|| AppError::Configuration("旦挞请求正文无法安全重试".to_owned()))?;
            match self.http.execute(direct_request).await {
                Ok(response) => return Ok(response),
                Err(error) if is_connection_error(&error) => {
                    self.remember_route(ROUTE_WEBVPN).await;
                }
                Err(error) => return Err(AppError::from(error)),
            }
        }

        let campus = campus.ok_or_else(webvpn_login_required)?;
        match campus.webvpn_request(request).await {
            Ok(response) => Ok(response),
            Err(AppError::Auth(_)) => Err(webvpn_login_required()),
            Err(AppError::Network(_)) => Err(AppError::Network(
                "通过复旦 WebVPN 连接旦挞失败，请稍后重试".to_owned(),
            )),
            Err(error) => Err(error),
        }
    }

    async fn execute_direct(
        &self,
        request: Request,
        use_webvpn: bool,
    ) -> Result<Response, AppError> {
        self.http.execute(request).await.map_err(|_| {
            if use_webvpn {
                AppError::Network("无法连接旦挞服务，请稍后重试".to_owned())
            } else {
                AppError::Network("无法连接旦挞服务，请开启自动 WebVPN 或检查网络".to_owned())
            }
        })
    }

    async fn direct_available(&self) -> bool {
        let mut route = self.route.lock().await;
        if route
            .checked_at
            .is_some_and(|checked_at| checked_at.elapsed() < ROUTE_CACHE_TTL)
        {
            return route.route == ROUTE_DIRECT;
        }

        let direct = self
            .http
            .get(DIRECT_CONNECT_TEST_URL)
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .is_ok();
        route.route = if direct { ROUTE_DIRECT } else { ROUTE_WEBVPN };
        route.checked_at = Some(Instant::now());
        direct
    }

    async fn remember_route(&self, route: u8) {
        let mut cached = self.route.lock().await;
        cached.route = route;
        cached.checked_at = Some(Instant::now());
    }
}

/// Orchestrates forum authentication on top of a [SessionStore].
///
/// It keeps the UI-facing workflow (login, status, logout, authenticated
/// reads with one automatic refresh round-trip) in one place.
pub struct SessionManager {
    forum: Arc<ForumService>,
    store: Arc<dyn SessionStore>,
    refresh_lock: tokio::sync::Mutex<()>,
}

impl SessionManager {
    pub fn new(store: Arc<dyn SessionStore>) -> Result<Self, AppError> {
        Ok(Self::with_forum(store, Arc::new(ForumService::new()?)))
    }

    pub fn with_forum(store: Arc<dyn SessionStore>, forum: Arc<ForumService>) -> Self {
        Self {
            forum,
            store,
            refresh_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub fn forum_service(&self) -> &ForumService {
        &self.forum
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<SessionStatus, AppError> {
        let _guard = self.refresh_lock.lock().await;
        let token = self
            .forum
            .login(email, password, campus, use_webvpn)
            .await?;
        self.store.save_token(&token)?;
        drop(_guard);
        Ok(self.authenticated_status(&token, campus, use_webvpn).await)
    }

    /// Register a new account and enter the logged-in state right away.
    pub async fn register(
        &self,
        email: &str,
        password: &str,
        verification_code: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<SessionStatus, AppError> {
        let _guard = self.refresh_lock.lock().await;
        let token = self
            .forum
            .register(email, password, verification_code, campus, use_webvpn)
            .await?;
        self.store.save_token(&token)?;
        drop(_guard);
        Ok(self.authenticated_status(&token, campus, use_webvpn).await)
    }

    pub async fn check_register_status(
        &self,
        email: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<bool, AppError> {
        self.forum
            .check_register_status(email, campus, use_webvpn)
            .await
    }

    pub async fn send_verification_code(
        &self,
        email: &str,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<(), AppError> {
        self.forum
            .send_verification_code(email, campus, use_webvpn)
            .await
    }

    pub async fn session_status(
        &self,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<SessionStatus, AppError> {
        let Some(token) = self.store.load_token()? else {
            return Ok(logged_out());
        };
        if !token.is_valid() {
            return Ok(logged_out());
        }
        Ok(self.authenticated_status(&token, campus, use_webvpn).await)
    }

    /// Restore the locally persisted community login state without making a
    /// network request. The profile is populated by a later verified refresh.
    pub fn local_session_status(&self) -> Result<SessionStatus, AppError> {
        let token = self.store.load_token()?;
        let community_logged_in = token.as_ref().is_some_and(TokenPair::is_valid);

        Ok(SessionStatus {
            community_logged_in,
            community_user: None,
            campus_logged_in: false,
            campus_id: None,
            campus_name: None,
        })
    }

    pub async fn logout(&self) -> Result<(), AppError> {
        let _guard = self.refresh_lock.lock().await;
        self.store.clear_token()
    }

    pub async fn load_holes(
        &self,
        division_id: Option<i64>,
        size: u32,
        order: HoleSortOrder,
        before: Option<&str>,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<Vec<ForumHole>, AppError> {
        let token = self
            .store
            .load_token()?
            .filter(TokenPair::is_valid)
            .ok_or_else(|| AppError::Auth("尚未登录旦挞账户".to_owned()))?;

        match self
            .forum
            .load_holes(
                &token.access,
                division_id,
                size,
                order,
                before,
                campus,
                use_webvpn,
            )
            .await
        {
            Ok(holes) => Ok(holes),
            Err(AppError::Auth(_)) => {
                let refreshed = self.refresh_token(&token, campus, use_webvpn).await?;
                self.forum
                    .load_holes(
                        &refreshed.access,
                        division_id,
                        size,
                        order,
                        before,
                        campus,
                        use_webvpn,
                    )
                    .await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn load_thread(
        &self,
        hole_id: i64,
        offset: u32,
        size: u32,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<ForumThreadPage, AppError> {
        let token = self.community_token()?;
        match self
            .forum
            .load_thread(&token.access, hole_id, offset, size, campus, use_webvpn)
            .await
        {
            Ok(thread) => Ok(thread),
            Err(AppError::Auth(_)) => {
                let refreshed = self.refresh_token(&token, campus, use_webvpn).await?;
                self.forum
                    .load_thread(&refreshed.access, hole_id, offset, size, campus, use_webvpn)
                    .await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn search_course_groups(
        &self,
        query: &str,
        page: u32,
        page_size: u32,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<Vec<EvaluationCourseGroup>, AppError> {
        let token = self.community_token()?;
        match self
            .forum
            .search_course_groups(&token.access, query, page, page_size, campus, use_webvpn)
            .await
        {
            Ok(groups) => Ok(groups),
            Err(AppError::Auth(_)) => {
                let refreshed = self.refresh_token(&token, campus, use_webvpn).await?;
                self.forum
                    .search_course_groups(
                        &refreshed.access,
                        query,
                        page,
                        page_size,
                        campus,
                        use_webvpn,
                    )
                    .await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn course_group_detail(
        &self,
        group_id: i64,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<EvaluationCourseDetail, AppError> {
        let token = self.community_token()?;
        match self
            .forum
            .course_group_detail(&token.access, group_id, campus, use_webvpn)
            .await
        {
            Ok(detail) => Ok(detail),
            Err(AppError::Auth(_)) => {
                let refreshed = self.refresh_token(&token, campus, use_webvpn).await?;
                self.forum
                    .course_group_detail(&refreshed.access, group_id, campus, use_webvpn)
                    .await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn random_course_review(
        &self,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<EvaluationReview, AppError> {
        let token = self.community_token()?;
        match self
            .forum
            .random_course_review(&token.access, campus, use_webvpn)
            .await
        {
            Ok(review) => Ok(review),
            Err(AppError::Auth(_)) => {
                let refreshed = self.refresh_token(&token, campus, use_webvpn).await?;
                self.forum
                    .random_course_review(&refreshed.access, campus, use_webvpn)
                    .await
            }
            Err(error) => Err(error),
        }
    }

    fn community_token(&self) -> Result<TokenPair, AppError> {
        self.store
            .load_token()?
            .filter(TokenPair::is_valid)
            .ok_or_else(|| AppError::Auth("尚未登录旦挞账号".to_owned()))
    }

    async fn refresh_token(
        &self,
        current: &TokenPair,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<TokenPair, AppError> {
        let _guard = self.refresh_lock.lock().await;
        let latest = self.community_token()?;
        if latest.access != current.access || latest.refresh != current.refresh {
            return Ok(latest);
        }

        match self
            .forum
            .refresh(&latest.refresh, campus, use_webvpn)
            .await
        {
            Ok(token) => {
                self.store.save_token(&token)?;
                Ok(token)
            }
            Err(error @ AppError::Auth(_)) => {
                // Only an explicit authentication rejection invalidates the
                // refresh token. Transient network failures must not log out.
                let _ = self.store.clear_token();
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    async fn authenticated_status(
        &self,
        token: &TokenPair,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> SessionStatus {
        match self
            .forum
            .user_profile(&token.access, campus, use_webvpn)
            .await
        {
            Ok(user) => SessionStatus {
                community_logged_in: true,
                community_user: Some(user),
                campus_logged_in: false,
                campus_id: None,
                campus_name: None,
            },
            Err(AppError::Auth(_)) => match self.refresh_token(token, campus, use_webvpn).await {
                Ok(refreshed) => match self
                    .forum
                    .user_profile(&refreshed.access, campus, use_webvpn)
                    .await
                {
                    Ok(user) => SessionStatus {
                        community_logged_in: true,
                        community_user: Some(user),
                        campus_logged_in: false,
                        campus_id: None,
                        campus_name: None,
                    },
                    Err(AppError::Auth(_)) => {
                        self.clear_token_if_current(&refreshed).await;
                        logged_out()
                    }
                    Err(_) => SessionStatus {
                        community_logged_in: true,
                        community_user: None,
                        campus_logged_in: false,
                        campus_id: None,
                        campus_name: None,
                    },
                },
                Err(AppError::Auth(_)) => logged_out(),
                Err(_) => logged_in_without_profile(),
            },
            Err(_) => logged_in_without_profile(),
        }
    }

    async fn clear_token_if_current(&self, current: &TokenPair) {
        let _guard = self.refresh_lock.lock().await;
        let Ok(Some(latest)) = self.store.load_token() else {
            return;
        };
        if latest.access == current.access && latest.refresh == current.refresh {
            let _ = self.store.clear_token();
        }
    }
}

fn logged_out() -> SessionStatus {
    SessionStatus {
        community_logged_in: false,
        community_user: None,
        campus_logged_in: false,
        campus_id: None,
        campus_name: None,
    }
}

fn logged_in_without_profile() -> SessionStatus {
    SessionStatus {
        community_logged_in: true,
        community_user: None,
        campus_logged_in: false,
        campus_id: None,
        campus_name: None,
    }
}

fn env_base_url(var: &str, default: &str) -> String {
    env::var(var)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

fn is_connection_error(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout()
}

fn webvpn_login_required() -> AppError {
    AppError::Network("校外访问旦挞需要先登录复旦 UIS；登录后会自动通过 WebVPN 连接".to_owned())
}

fn login_error(status: StatusCode) -> AppError {
    match status.as_u16() {
        400 | 401 | 422 => AppError::Auth("邮箱或密码不正确".to_owned()),
        _ => AppError::upstream_status(status),
    }
}

fn request_error(status: StatusCode) -> AppError {
    if status == StatusCode::UNAUTHORIZED {
        AppError::Auth("登录状态已过期".to_owned())
    } else {
        AppError::upstream_status(status)
    }
}

fn forum_page_size(size: u32) -> u32 {
    size.clamp(1, 10)
}

fn decode_token(value: Result<TokenPair, reqwest::Error>) -> Result<TokenPair, AppError> {
    match value {
        Ok(token) if token.is_valid() => Ok(token),
        Ok(_) => Err(AppError::Auth("上游返回了无效的凭证".to_owned())),
        Err(_) => Err(AppError::Upstream("上游响应格式无法解析".to_owned())),
    }
}

#[derive(Deserialize)]
struct RawUser {
    user_id: Option<i64>,
    nickname: Option<String>,
    is_admin: Option<bool>,
}

#[derive(Deserialize)]
struct RawHole {
    hole_id: i64,
    division_id: Option<i64>,
    time_created: Option<String>,
    time_updated: Option<String>,
    view: Option<i64>,
    reply: Option<i64>,
    #[serde(default)]
    tags: Vec<RawTag>,
    floors: Option<RawFloors>,
}

#[derive(Deserialize)]
struct RawTag {
    name: Option<String>,
    temperature: Option<f32>,
}

#[derive(Clone, Deserialize)]
struct RawFloors {
    first_floor: Option<RawFloor>,
    last_floor: Option<RawFloor>,
}

#[derive(Clone, Deserialize)]
struct RawFloor {
    floor_id: Option<i64>,
    hole_id: Option<i64>,
    content: Option<String>,
    anonyname: Option<String>,
    time_created: Option<String>,
    time_updated: Option<String>,
    special_tag: Option<String>,
    deleted: Option<bool>,
    is_me: Option<bool>,
    liked: Option<bool>,
    like: Option<i64>,
    disliked: Option<bool>,
    dislike: Option<i64>,
    modified: Option<i64>,
    #[serde(default)]
    fold: Vec<String>,
}

#[derive(Deserialize)]
struct RawCourseSearchResults {
    #[serde(default)]
    items: Vec<RawCourseGroup>,
}

#[derive(Deserialize)]
struct RawCourseGroup {
    id: Option<i64>,
    name: Option<String>,
    code: Option<String>,
    department: Option<String>,
    week_hour: Option<i64>,
    #[serde(default)]
    credits: Vec<f64>,
    course_count: Option<i64>,
    review_count: Option<i64>,
    #[serde(default)]
    course_list: Vec<RawCourseOffering>,
}

#[derive(Deserialize)]
struct RawCourseOffering {
    name: Option<String>,
    teachers: Option<String>,
    year: Option<i64>,
    semester: Option<i64>,
    coursegroup_id: Option<i64>,
    #[serde(default)]
    review_list: Vec<RawCourseReview>,
}

#[derive(Deserialize)]
struct RawCourseReview {
    id: Option<i64>,
    title: Option<String>,
    content: Option<String>,
    time_created: Option<String>,
    time_updated: Option<String>,
    rank: Option<RawCourseRating>,
    remark: Option<i64>,
    vote: Option<i64>,
}

#[derive(Deserialize)]
struct RawRandomCourseReview {
    id: Option<i64>,
    title: Option<String>,
    content: Option<String>,
    time_created: Option<String>,
    time_updated: Option<String>,
    rank: Option<RawCourseRating>,
    remark: Option<i64>,
    vote: Option<i64>,
    group_id: Option<i64>,
    course: Option<RawReviewCourse>,
}

#[derive(Deserialize)]
struct RawReviewCourse {
    name: Option<String>,
    teachers: Option<String>,
    year: Option<i64>,
    semester: Option<i64>,
    coursegroup_id: Option<i64>,
}

#[derive(Deserialize)]
struct RawCourseRating {
    overall: Option<i64>,
    content: Option<i64>,
    workload: Option<i64>,
    assessment: Option<i64>,
}

impl From<RawTag> for ForumTag {
    fn from(value: RawTag) -> Self {
        ForumTag {
            name: value.name.unwrap_or_default(),
            temperature: value.temperature.unwrap_or_default(),
        }
    }
}

impl From<RawFloor> for ForumFloorPreview {
    fn from(value: RawFloor) -> Self {
        ForumFloorPreview {
            floor_id: value.floor_id.unwrap_or_default(),
            content: value.content.unwrap_or_default(),
            anonyname: value.anonyname.unwrap_or_default(),
            time_created: value.time_created.unwrap_or_default(),
        }
    }
}

impl From<RawFloor> for ForumFloor {
    fn from(value: RawFloor) -> Self {
        ForumFloor {
            floor_id: value.floor_id.unwrap_or_default(),
            hole_id: value.hole_id.unwrap_or_default(),
            content: value.content.unwrap_or_default(),
            anonyname: value.anonyname.unwrap_or_default(),
            time_created: value.time_created.unwrap_or_default(),
            time_updated: value.time_updated.unwrap_or_default(),
            special_tag: value.special_tag.unwrap_or_default(),
            deleted: value.deleted.unwrap_or(false),
            is_me: value.is_me.unwrap_or(false),
            liked: value.liked.unwrap_or(false),
            like: value.like.unwrap_or_default(),
            disliked: value.disliked.unwrap_or(false),
            dislike: value.dislike.unwrap_or_default(),
            modified: value.modified.unwrap_or_default(),
            fold: value.fold,
        }
    }
}

impl From<RawHole> for ForumHole {
    fn from(value: RawHole) -> Self {
        let floors = value.floors;
        ForumHole {
            hole_id: value.hole_id,
            division_id: value.division_id.unwrap_or_default(),
            time_created: value.time_created.unwrap_or_default(),
            time_updated: value.time_updated.unwrap_or_default(),
            view: value.view.unwrap_or_default(),
            reply: value.reply.unwrap_or_default(),
            tags: value.tags.into_iter().map(Into::into).collect(),
            first_floor: floors
                .as_ref()
                .and_then(|floors| floors.first_floor.clone())
                .map(Into::into),
            last_floor: floors.and_then(|floors| floors.last_floor).map(Into::into),
        }
    }
}

impl From<RawCourseGroup> for EvaluationCourseGroup {
    fn from(value: RawCourseGroup) -> Self {
        let course_count = value.course_count.unwrap_or(value.course_list.len() as i64);
        let review_count = value.review_count.unwrap_or_else(|| {
            value
                .course_list
                .iter()
                .map(|course| course.review_list.len() as i64)
                .sum()
        });
        EvaluationCourseGroup {
            group_id: value.id.unwrap_or_default(),
            name: value.name.unwrap_or_default(),
            code: value.code.unwrap_or_default(),
            department: value.department.unwrap_or_default(),
            week_hour: value.week_hour,
            credits: value.credits,
            course_count,
            review_count,
        }
    }
}

impl RawCourseGroup {
    fn into_detail(self) -> EvaluationCourseDetail {
        let group_id = self.id.unwrap_or_default();
        let group_name = self.name.clone().unwrap_or_default();
        let reviews = self
            .course_list
            .iter()
            .flat_map(|course| {
                let course_group_id = course.coursegroup_id.or(Some(group_id));
                let course_name = course
                    .name
                    .clone()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| group_name.clone());
                let teachers = course.teachers.clone().unwrap_or_default();
                let term = format_term(course.year, course.semester);
                course.review_list.iter().map(move |review| {
                    review.to_evaluation(
                        course_group_id,
                        course_name.clone(),
                        teachers.clone(),
                        term.clone(),
                    )
                })
            })
            .collect();
        EvaluationCourseDetail {
            group: self.into(),
            reviews,
        }
    }
}

impl RawCourseReview {
    fn to_evaluation(
        &self,
        course_group_id: Option<i64>,
        course_name: String,
        teachers: String,
        term: String,
    ) -> EvaluationReview {
        EvaluationReview {
            review_id: self.id.unwrap_or_default(),
            title: self.title.clone().unwrap_or_default(),
            content: self.content.clone().unwrap_or_default(),
            time_created: self.time_created.clone().unwrap_or_default(),
            time_updated: self.time_updated.clone().unwrap_or_default(),
            rating: self.rank.as_ref().map(Into::into).unwrap_or_default(),
            vote: self.vote.unwrap_or_default(),
            remark: self.remark.unwrap_or_default(),
            course_group_id,
            course_name,
            teachers,
            term,
        }
    }
}

impl From<RawRandomCourseReview> for EvaluationReview {
    fn from(value: RawRandomCourseReview) -> Self {
        let course = value.course;
        let course_group_id = course
            .as_ref()
            .and_then(|course| course.coursegroup_id)
            .or(value.group_id);
        let course_name = course
            .as_ref()
            .and_then(|course| course.name.clone())
            .unwrap_or_default();
        let teachers = course
            .as_ref()
            .and_then(|course| course.teachers.clone())
            .unwrap_or_default();
        let term = course
            .as_ref()
            .map(|course| format_term(course.year, course.semester))
            .unwrap_or_default();
        EvaluationReview {
            review_id: value.id.unwrap_or_default(),
            title: value.title.unwrap_or_default(),
            content: value.content.unwrap_or_default(),
            time_created: value.time_created.unwrap_or_default(),
            time_updated: value.time_updated.unwrap_or_default(),
            rating: value.rank.as_ref().map(Into::into).unwrap_or_default(),
            vote: value.vote.unwrap_or_default(),
            remark: value.remark.unwrap_or_default(),
            course_group_id,
            course_name,
            teachers,
            term,
        }
    }
}

impl From<&RawCourseRating> for EvaluationRating {
    fn from(value: &RawCourseRating) -> Self {
        EvaluationRating {
            overall: normalized_rating(value.overall),
            content: inverted_rating(value.content),
            workload: inverted_rating(value.workload),
            assessment: normalized_rating(value.assessment),
        }
    }
}

fn normalized_rating(value: Option<i64>) -> Option<i64> {
    value.filter(|value| (1..=5).contains(value))
}

fn inverted_rating(value: Option<i64>) -> Option<i64> {
    normalized_rating(value).map(|value| 6 - value)
}

fn format_term(year: Option<i64>, semester: Option<i64>) -> String {
    let Some(year) = year else {
        return String::new();
    };
    let semester = match semester {
        Some(1) => "秋季",
        Some(2) => "寒假",
        Some(3) => "春季",
        Some(4) => "暑假",
        _ => "",
    };
    if semester.is_empty() {
        format!("{year}~{}学年", year + 1)
    } else {
        format!("{year}~{}学年 · {semester}", year + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_error_maps_upstream_statuses() {
        assert!(matches!(
            login_error(StatusCode::UNAUTHORIZED),
            AppError::Auth(_)
        ));
        assert!(matches!(
            login_error(StatusCode::BAD_REQUEST),
            AppError::Auth(_)
        ));
        assert!(matches!(
            login_error(StatusCode::INTERNAL_SERVER_ERROR),
            AppError::Upstream(_)
        ));
    }

    #[test]
    fn request_error_maps_unauthorized_to_auth() {
        assert!(matches!(
            request_error(StatusCode::UNAUTHORIZED),
            AppError::Auth(_)
        ));
        assert!(matches!(
            request_error(StatusCode::FORBIDDEN),
            AppError::Upstream(_)
        ));
    }

    #[test]
    fn forum_page_size_matches_upstream_limit() {
        assert_eq!(forum_page_size(0), 1);
        assert_eq!(forum_page_size(10), 10);
        assert_eq!(forum_page_size(20), 10);
    }

    #[test]
    fn local_session_status_uses_stored_token_without_profile_request() {
        let store = Arc::new(crate::MemorySessionStore::new());
        store
            .save_token(&TokenPair {
                access: "access-token".to_owned(),
                refresh: "refresh-token".to_owned(),
            })
            .expect("token should be stored");
        let manager = SessionManager::new(store).expect("session manager should initialize");

        let status = manager
            .local_session_status()
            .expect("local session status should load");

        assert!(status.community_logged_in);
        assert!(status.community_user.is_none());
    }

    #[test]
    fn raw_hole_maps_to_dto() {
        let raw = serde_json::from_value::<RawHole>(serde_json::json!({
            "hole_id": 104821,
            "division_id": 1,
            "time_created": "2026-09-28T08:00:00Z",
            "time_updated": "2026-09-28T09:30:00Z",
            "view": 321,
            "reply": 18,
            "tags": [{ "name": "校园生活", "temperature": 42.0 }],
            "floors": {
                "first_floor": {
                    "floor_id": 1,
                    "content": "今天傍晚邯郸校区的云很漂亮",
                    "anonyname": "Alice",
                    "time_created": "2026-09-28T08:00:00Z"
                }
            }
        }))
        .expect("raw hole fixture should deserialize");

        let hole: ForumHole = raw.into();
        assert_eq!(hole.hole_id, 104821);
        assert_eq!(hole.reply, 18);
        assert_eq!(hole.tags.len(), 1);
        assert_eq!(hole.tags[0].name, "校园生活");
        assert_eq!(
            hole.first_floor.as_ref().expect("first floor").content,
            "今天傍晚邯郸校区的云很漂亮"
        );
        assert!(hole.last_floor.is_none());
    }
}
