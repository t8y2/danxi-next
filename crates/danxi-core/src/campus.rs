use std::{
    collections::HashMap,
    env,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use base64::Engine as _;
use reqwest::{Client, Request, Response, cookie::Jar, header};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AppError, SessionStatus};

pub const DEFAULT_ID_HOST: &str = "id.fudan.edu.cn";
/// A known-registered SSO service used to initiate the authentication chain.
const DEFAULT_AUTH_SERVICE: &str = "https://fdjwgl.fudan.edu.cn/student/sso/login?refer=https://fdjwgl.fudan.edu.cn/student/for-std/course-table";
const EHALL_PROFILE_URL: &str = "https://ehall.fudan.edu.cn/manage/common/login/index?redirect=https%3A%2F%2Fehall.fudan.edu.cn";
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.4844.51 Safari/537.36";
const DIRECT_ROUTE_CACHE_TTL: Duration = Duration::from_secs(60);
const SECOND_FACTOR_MESSAGE: &str = "该账户需要二次验证，请先在复旦统一身份认证网页完成验证后重试";

#[derive(Clone, Copy)]
struct DirectRoute {
    available: bool,
    checked_at: Instant,
}

/// Campus (id.fudan.edu.cn) credentials, persisted only behind an explicit
/// credential-store boundary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampusCredentials {
    pub id: String,
    pub password: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Chosen at login, as in the Flutter client's two buttons; it decides
    /// which timetable system (fdjwgl vs yjsxk) serves this student.
    #[serde(default)]
    pub is_graduate: bool,
}

/// What the UI may know about the campus session.
#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CampusStatus {
    pub logged_in: bool,
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CampusLoginResult {
    Authenticated { status: CampusStatus },
    RequiresSecondFactor { message: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampusAuthenticationResult {
    Authenticated,
    RequiresSecondFactor,
}

enum AuthMethodSelection {
    Password(String),
    RequiresSecondFactor,
}

/// Persistence boundary for campus credentials.
pub trait CampusCredentialStore: Send + Sync {
    fn load(&self) -> Result<Option<CampusCredentials>, AppError>;
    fn save(&self, credentials: &CampusCredentials) -> Result<(), AppError>;
    fn clear(&self) -> Result<(), AppError>;
}

/// A browser-like session with Fudan's identity service (id.fudan.edu.cn).
///
/// Mirrors the Flutter client's V2 flow:
/// 1. `GET /idp/authCenter/authenticate?service=…` redirects to a URL whose
///    fragment carries `lck` and `entityId`;
/// 2. `POST /idp/authn/queryAuthMethods` returns the `userAndPwd` chain code;
/// 3. `POST /idp/authn/getJsPublicKey` returns an RSA public key;
/// 4. `POST /idp/authn/authExecute` with the RSA-encrypted password.
pub struct CampusService {
    http: Client,
    cookies: Arc<Jar>,
    // Shares the cookie jar with `http` but never follows redirects, so the
    // authenticate hop's Location (with its fragment) stays readable.
    http_nofollow: Client,
    id_host: String,
    service: String,
    webvpn_authenticated: AtomicBool,
    webvpn_auth_lock: tokio::sync::Mutex<()>,
    direct_routes: RwLock<HashMap<String, DirectRoute>>,
}

impl CampusService {
    pub fn new() -> Result<Self, AppError> {
        let jar = Arc::new(Jar::default());
        let builder = |redirects: reqwest::redirect::Policy| {
            let mut builder = Client::builder()
                .cookie_provider(jar.clone())
                .redirect(redirects)
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .user_agent(BROWSER_USER_AGENT);
            if let Ok(proxy_url) = env::var("DANXI_HTTP_PROXY") {
                if !proxy_url.trim().is_empty() {
                    let proxy = reqwest::Proxy::all(&proxy_url).map_err(|_| {
                        AppError::Configuration("DANXI_HTTP_PROXY 配置无效".to_owned())
                    })?;
                    builder = builder.proxy(proxy);
                }
            }
            builder
                .build()
                .map_err(|_| AppError::Configuration("HTTP 客户端初始化失败".to_owned()))
        };

        Ok(Self {
            http: builder(reqwest::redirect::Policy::limited(12))?,
            http_nofollow: builder(reqwest::redirect::Policy::none())?,
            cookies: jar,
            id_host: env::var("DANXI_ID_HOST")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_ID_HOST.to_owned()),
            service: env::var("DANXI_AUTH_SERVICE")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_AUTH_SERVICE.to_owned()),
            webvpn_authenticated: AtomicBool::new(false),
            webvpn_auth_lock: tokio::sync::Mutex::new(()),
            direct_routes: RwLock::new(HashMap::new()),
        })
    }

    pub async fn login(
        &self,
        id: &str,
        password: &str,
    ) -> Result<CampusAuthenticationResult, AppError> {
        let (lck, entity_id) = self.authentication_context().await?;
        let AuthMethodSelection::Password(chain_code) = self.auth_method(&lck, &entity_id).await?
        else {
            return Ok(CampusAuthenticationResult::RequiresSecondFactor);
        };
        let public_key = self.public_key().await?;
        let encrypted = encrypt_password(&public_key, password)?;

        let response = self
            .http
            .post(format!("https://{}/idp/authn/authExecute", self.id_host))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({
                "authModuleCode": "userAndPwd",
                "authChainCode": chain_code,
                "entityId": entity_id,
                "requestType": "chain_type",
                "lck": lck,
                "authPara": {
                    "loginName": id,
                    "password": encrypted,
                    "verifyCode": "",
                }
            }))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::upstream_status(status));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| AppError::Upstream("认证响应格式无法解析".to_owned()))?;
        auth_execute_error(&body)?;
        // Exchange the login token for a ticket-issuing SSO session.
        let login_token = body
            .get("loginToken")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Auth("认证失败：服务未颁发会话".to_owned()))?;
        let response = self
            .http
            .post(format!(
                "https://{}/idp/authCenter/authnEngine",
                self.id_host
            ))
            .form(&[("loginToken", login_token)])
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::upstream_status(status));
        }
        Ok(CampusAuthenticationResult::Authenticated)
    }

    /// Follow the authenticate redirect and pull `lck`/`entityId` out of the
    /// `#/index?lck=…&entityId=…` fragment.
    async fn authentication_context(&self) -> Result<(String, String), AppError> {
        let location = self.authentication_url(&self.service).await?;
        let fragment = location
            .fragment()
            .ok_or_else(|| AppError::Upstream("认证跳转缺少会话参数".to_owned()))?;
        let query = fragment
            .split_once('?')
            .map(|(_, query)| query)
            .unwrap_or("");
        let mut lck = None;
        let mut entity_id = None;
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "lck" => lck = Some(value.to_owned()),
                "entityId" => entity_id = Some(value.to_owned()),
                _ => {}
            }
        }
        match (lck, entity_id) {
            (Some(lck), Some(entity_id)) if !lck.is_empty() && !entity_id.is_empty() => {
                Ok((lck, entity_id))
            }
            _ => Err(AppError::Upstream("认证跳转缺少会话参数".to_owned())),
        }
    }

    async fn authentication_url(&self, service_url: &str) -> Result<reqwest::Url, AppError> {
        let response = self
            .http_nofollow
            .get(format!(
                "https://{}/idp/authCenter/authenticate?service={}",
                self.id_host,
                urlencoding::encode(service_url)
            ))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() && !status.is_redirection() {
            return Err(AppError::upstream_status(status));
        }
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::Upstream("认证服务未返回跳转地址".to_owned()))?;
        response
            .url()
            .join(location)
            .map_err(|_| AppError::Upstream("认证跳转地址无效".to_owned()))
    }

    async fn auth_method(
        &self,
        lck: &str,
        entity_id: &str,
    ) -> Result<AuthMethodSelection, AppError> {
        let response = self
            .http
            .post(format!(
                "https://{}/idp/authn/queryAuthMethods",
                self.id_host
            ))
            .json(&serde_json::json!({ "lck": lck, "entityId": entity_id }))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::upstream_status(status));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| AppError::Upstream("认证方式响应格式无法解析".to_owned()))?;
        if body.get("second").and_then(Value::as_bool) == Some(true) {
            return Ok(AuthMethodSelection::RequiresSecondFactor);
        }
        body.get("data")
            .and_then(Value::as_array)
            .and_then(|methods| {
                methods.iter().find(|method| {
                    method.get("moduleCode").and_then(Value::as_str) == Some("userAndPwd")
                })
            })
            .and_then(|method| method.get("authChainCode"))
            .and_then(Value::as_str)
            .map(|code| AuthMethodSelection::Password(code.to_owned()))
            .ok_or_else(|| AppError::Upstream("未找到密码登录方式".to_owned()))
    }

    /// Load a service page through the SSO ticket dance.
    ///
    /// If the service redirects to the identity host, the (already
    /// authenticated) response embeds an auto-submit `#logon` form carrying a
    /// ticket; submitting it establishes the service session.
    pub async fn service_page(&self, service_url: &str) -> Result<String, AppError> {
        let service_host = host_of(service_url)?;
        let response = self.http.get(service_url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::upstream_status(status));
        }
        let final_host = response
            .url()
            .host_str()
            .ok_or_else(|| AppError::Upstream("服务响应缺少地址".to_owned()))?
            .to_owned();
        let body = response
            .text()
            .await
            .map_err(|_| AppError::Upstream("服务页面读取失败".to_owned()))?;
        if final_host == service_host {
            return Ok(body);
        }
        if final_host != self.id_host {
            return Err(AppError::Upstream(format!("意外的跳转目标：{final_host}")));
        }
        if std::env::var("DANXI_DEBUG").is_ok() {
            eprintln!("[debug] campus authentication redirected to {final_host}");
        }

        let ticket_url = ticket_redirect(&body)?;
        let response = self.http.get(ticket_url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::upstream_status(status));
        }
        let final_host = response.url().host_str().unwrap_or_default().to_owned();
        let body = response
            .text()
            .await
            .map_err(|_| AppError::Upstream("服务页面读取失败".to_owned()))?;
        if final_host != service_host {
            return Err(AppError::Auth("校园会话已失效，请重新登录".to_owned()));
        }
        Ok(body)
    }

    /// Execute a request through Fudan WebVPN with the live UIS cookie jar.
    pub async fn webvpn_request(&self, request: Request) -> Result<Response, AppError> {
        self.ensure_webvpn_session().await?;
        let retry = request
            .try_clone()
            .ok_or_else(|| AppError::Configuration("WebVPN 请求正文无法安全重试".to_owned()))?;
        let response = self.execute_webvpn_request(request).await?;
        if !crate::webvpn::is_login_redirect(response.url()) {
            return Ok(response);
        }

        self.webvpn_authenticated.store(false, Ordering::Release);
        self.ensure_webvpn_session().await?;
        let response = self.execute_webvpn_request(retry).await?;
        if crate::webvpn::is_login_redirect(response.url()) {
            return Err(AppError::Auth(
                "WebVPN 会话建立失败，请重新登录复旦 UIS".to_owned(),
            ));
        }
        Ok(response)
    }

    /// Execute a request against a Fudan service, establishing that service's
    /// SSO cookie and replaying the original method when authentication is needed.
    pub async fn authenticated_request(&self, request: Request) -> Result<Response, AppError> {
        let service_host = request
            .url()
            .host_str()
            .map(str::to_owned)
            .ok_or_else(|| AppError::Configuration("校园服务地址缺少主机名".to_owned()))?;
        let retry = request
            .try_clone()
            .ok_or_else(|| AppError::Configuration("校园服务请求正文无法安全重试".to_owned()))?;
        let response = self.http.execute(request).await?;
        let final_host = response.url().host_str().unwrap_or_default().to_owned();
        if final_host == service_host {
            return Ok(response);
        }
        if final_host != self.id_host {
            return Err(AppError::Upstream(format!("意外的跳转目标：{final_host}")));
        }

        let body = response
            .text()
            .await
            .map_err(|_| AppError::Upstream("校园认证页面读取失败".to_owned()))?;
        let ticket_url = ticket_redirect(&body)
            .map_err(|_| AppError::EnhancedAuth("该校园服务需要完成复旦双因素认证".to_owned()))?;
        let callback = self.http.get(ticket_url).send().await?;
        if callback.url().host_str() != Some(service_host.as_str()) {
            return Err(AppError::Auth(
                "校园服务登录失败，请重新登录复旦 UIS".to_owned(),
            ));
        }
        drop(callback);

        let response = self.http.execute(retry).await?;
        if response.url().host_str() == Some(service_host.as_str()) {
            Ok(response)
        } else {
            Err(AppError::Auth(format!("无法登录校园服务：{service_host}")))
        }
    }

    pub async fn direct_request(&self, request: Request) -> Result<Response, AppError> {
        self.http.execute(request).await.map_err(AppError::from)
    }

    fn import_cookie_headers(&self, url: &str, cookies: &[String]) -> Result<(), AppError> {
        let url = reqwest::Url::parse(url)
            .map_err(|_| AppError::Configuration("认证 Cookie 地址无效".to_owned()))?;
        for cookie in cookies {
            self.cookies.add_cookie_str(cookie, &url);
        }
        Ok(())
    }

    async fn ensure_webvpn_session(&self) -> Result<(), AppError> {
        if self.webvpn_authenticated.load(Ordering::Acquire) {
            return Ok(());
        }
        let _guard = self.webvpn_auth_lock.lock().await;
        if self.webvpn_authenticated.load(Ordering::Acquire) {
            return Ok(());
        }
        self.service_page(crate::webvpn::WEBVPN_LOGIN_URL).await?;
        self.webvpn_authenticated.store(true, Ordering::Release);
        Ok(())
    }

    async fn execute_webvpn_request(&self, mut request: Request) -> Result<Response, AppError> {
        let translated = crate::webvpn::translate(request.url())?.ok_or_else(|| {
            AppError::Configuration("该地址不支持通过复旦 WebVPN 访问".to_owned())
        })?;
        *request.url_mut() = translated;
        self.http.execute(request).await.map_err(AppError::from)
    }

    async fn direct_route_available(&self, url: &reqwest::Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        if let Ok(routes) = self.direct_routes.read()
            && let Some(route) = routes.get(host)
            && route.checked_at.elapsed() < DIRECT_ROUTE_CACHE_TTL
        {
            return route.available;
        }

        let mut probe_url = url.clone();
        probe_url.set_path("/");
        probe_url.set_query(None);
        probe_url.set_fragment(None);
        let available = self
            .http
            .get(probe_url)
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .is_ok();
        if let Ok(mut routes) = self.direct_routes.write() {
            routes.insert(
                host.to_owned(),
                DirectRoute {
                    available,
                    checked_at: Instant::now(),
                },
            );
        }
        available
    }

    fn remember_direct_route(&self, url: &reqwest::Url, available: bool) {
        if let Some(host) = url.host_str()
            && let Ok(mut routes) = self.direct_routes.write()
        {
            routes.insert(
                host.to_owned(),
                DirectRoute {
                    available,
                    checked_at: Instant::now(),
                },
            );
        }
    }

    async fn webvpn_request_pair(
        &self,
        first: Request,
        second: Request,
    ) -> Result<(Response, Response), AppError> {
        self.ensure_webvpn_session().await?;
        let first_retry = first
            .try_clone()
            .ok_or_else(|| AppError::Configuration("WebVPN 请求正文无法安全重试".to_owned()))?;
        let second_retry = second
            .try_clone()
            .ok_or_else(|| AppError::Configuration("WebVPN 请求正文无法安全重试".to_owned()))?;
        let (first_response, second_response) = futures::try_join!(
            self.execute_webvpn_request(first),
            self.execute_webvpn_request(second)
        )?;
        if !crate::webvpn::is_login_redirect(first_response.url())
            && !crate::webvpn::is_login_redirect(second_response.url())
        {
            return Ok((first_response, second_response));
        }

        self.webvpn_authenticated.store(false, Ordering::Release);
        self.ensure_webvpn_session().await?;
        let (first_response, second_response) = futures::try_join!(
            self.execute_webvpn_request(first_retry),
            self.execute_webvpn_request(second_retry)
        )?;
        if crate::webvpn::is_login_redirect(first_response.url())
            || crate::webvpn::is_login_redirect(second_response.url())
        {
            return Err(AppError::Auth(
                "WebVPN 会话建立失败，请重新登录复旦 UIS".to_owned(),
            ));
        }
        Ok((first_response, second_response))
    }

    /// Fetch the timetable for this student.
    pub async fn fetch_timetable(&self, is_graduate: bool) -> Result<crate::Timetable, AppError> {
        if is_graduate {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or_default();
            let page = self
                .service_page(&format!(
                    "http://yjsxk.fudan.edu.cn/yjsxkapp/sys/xsxkappfudan/xsxkCourse/loadKbxx.do?_={timestamp}"
                ))
                .await?;
            let payload: Value = serde_json::from_str(&page)
                .map_err(|_| AppError::Upstream("课表响应格式无法解析".to_owned()))?;
            crate::parse_postgraduate(&payload)
        } else {
            let course_table_url = "https://fdjwgl.fudan.edu.cn/student/for-std/course-table";
            let page = self.service_page(course_table_url).await?;
            let semester_start = crate::parse_semester_start(&page);
            let Some(semester_id) = semester_id_from_html(&page) else {
                return Err(AppError::Upstream("未能识别当前学期".to_owned()));
            };
            let print_url = format!(
                "https://fdjwgl.fudan.edu.cn/student/for-std/course-table/semester/{semester_id}/print-data"
            );
            let response = self.http.get(print_url).send().await?;
            let status = response.status();
            if !status.is_success() {
                return Err(AppError::upstream_status(status));
            }
            let payload: Value = response
                .json()
                .await
                .map_err(|_| AppError::Upstream("课表响应格式无法解析".to_owned()))?;
            let mut timetable = crate::parse_jwgl(&payload)?;
            timetable.semester_start_date = semester_start;
            Ok(timetable)
        }
    }

    /// Fetch the signed-in user's real name from the official eHall profile.
    pub async fn fetch_user_name(&self) -> Result<String, AppError> {
        let page = self.service_page(EHALL_PROFILE_URL).await?;
        ehall_user_name(&page).ok_or_else(|| AppError::Upstream("未能读取校园账户姓名".to_owned()))
    }

    async fn public_key(&self) -> Result<Vec<u8>, AppError> {
        let response = self
            .http
            .post(format!("https://{}/idp/authn/getJsPublicKey", self.id_host))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::upstream_status(status));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| AppError::Upstream("公钥响应格式无法解析".to_owned()))?;
        let encoded = body
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Upstream("公钥响应缺少数据".to_owned()))?;
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| AppError::Upstream("公钥解码失败".to_owned()))
    }
}

/// Orchestrates the campus session on top of a credential store, keeping one
/// live [CampusService] (cookie jar) in memory and re-logging in on demand.
pub struct CampusSession {
    store: Arc<dyn CampusCredentialStore>,
    service: tokio::sync::Mutex<Option<Arc<CampusService>>>,
}

impl CampusSession {
    pub fn new(store: Arc<dyn CampusCredentialStore>) -> Result<Self, AppError> {
        Ok(Self {
            store,
            service: tokio::sync::Mutex::new(None),
        })
    }

    pub async fn login(
        &self,
        id: &str,
        password: &str,
        is_graduate: bool,
    ) -> Result<CampusLoginResult, AppError> {
        let mut slot = self.service.lock().await;
        let mut credentials = CampusCredentials {
            id: id.trim().to_owned(),
            password: password.to_owned(),
            name: None,
            is_graduate,
        };
        let service = Arc::new(CampusService::new()?);
        if matches!(
            service
                .login(&credentials.id, &credentials.password)
                .await?,
            CampusAuthenticationResult::RequiresSecondFactor
        ) {
            return Ok(CampusLoginResult::RequiresSecondFactor {
                message: SECOND_FACTOR_MESSAGE.to_owned(),
            });
        }
        credentials.name = service.fetch_user_name().await.ok();
        self.store.save(&credentials)?;
        *slot = Some(service);
        Ok(CampusLoginResult::Authenticated {
            status: CampusStatus {
                logged_in: true,
                id: Some(credentials.id),
                name: credentials.name,
            },
        })
    }

    pub async fn status(&self) -> Result<CampusStatus, AppError> {
        let Some(mut credentials) = self.store.load()? else {
            return Ok(CampusStatus::default());
        };

        let service = match self.authenticated_service().await {
            Ok(service) => Some(service),
            Err(AppError::Auth(_)) => {
                self.logout().await?;
                return Ok(CampusStatus::default());
            }
            Err(_) => None,
        };

        if credentials.name.is_none() {
            if let Some(service) = service
                && let Ok(name) = service.fetch_user_name().await
            {
                credentials.name = Some(name);
                let _ = self.store.save(&credentials);
            }
        }

        Ok(CampusStatus {
            logged_in: true,
            id: Some(credentials.id),
            name: credentials.name,
        })
    }

    /// Restore the locally persisted campus identity without re-authenticating
    /// against the university service.
    pub fn local_status(&self) -> Result<CampusStatus, AppError> {
        let Some(credentials) = self.store.load()? else {
            return Ok(CampusStatus::default());
        };

        Ok(CampusStatus {
            logged_in: true,
            id: Some(credentials.id),
            name: credentials.name,
        })
    }

    pub async fn logout(&self) -> Result<(), AppError> {
        let mut service = self.service.lock().await;
        self.store.clear()?;
        *service = None;
        Ok(())
    }

    /// Return the live cookie session, silently restoring it from the stored
    /// credentials when the process restarted.
    pub async fn authenticated_service(&self) -> Result<Arc<CampusService>, AppError> {
        let mut slot = self.service.lock().await;
        if let Some(service) = slot.as_ref() {
            return Ok(service.clone());
        }

        let credentials = self
            .store
            .load()?
            .ok_or_else(|| AppError::Auth("尚未登录复旦 UIS".to_owned()))?;
        let service = Arc::new(CampusService::new()?);
        if matches!(
            service
                .login(&credentials.id, &credentials.password)
                .await?,
            CampusAuthenticationResult::RequiresSecondFactor
        ) {
            return Err(AppError::Auth(SECOND_FACTOR_MESSAGE.to_owned()));
        }
        *slot = Some(service.clone());
        Ok(service)
    }

    pub fn credentials_for_enhanced_auth(&self) -> Result<CampusCredentials, AppError> {
        self.store
            .load()?
            .ok_or_else(|| AppError::Auth("尚未登录复旦 UIS".to_owned()))
    }

    pub async fn enhanced_auth_context(
        &self,
        service_url: &str,
    ) -> Result<(String, String), AppError> {
        let service = self.authenticated_service().await?;
        let target_host = host_of(service_url)?;
        let login_url = service.authentication_url(service_url).await?;
        let login_host = login_url.host_str().unwrap_or_default();
        if login_host == target_host {
            return Err(AppError::Configuration(
                "该校园服务已完成双因素认证".to_owned(),
            ));
        }
        if login_host != service.id_host {
            return Err(AppError::Upstream(format!(
                "意外的双因素认证跳转目标：{login_host}"
            )));
        }
        Ok((login_url.to_string(), target_host))
    }

    pub async fn import_enhanced_auth_cookies(
        &self,
        url: &str,
        cookies: &[String],
    ) -> Result<(), AppError> {
        self.authenticated_service()
            .await?
            .import_cookie_headers(url, cookies)
    }

    pub async fn webvpn_request(&self, request: Request) -> Result<Response, AppError> {
        self.authenticated_service()
            .await?
            .webvpn_request(request)
            .await
    }

    pub async fn authenticated_request(&self, request: Request) -> Result<Response, AppError> {
        self.authenticated_service()
            .await?
            .authenticated_request(request)
            .await
    }

    pub async fn routed_request(
        &self,
        request: Request,
        use_webvpn: bool,
    ) -> Result<Response, AppError> {
        let service = self.authenticated_service().await?;
        if !use_webvpn || !crate::webvpn::supports(request.url()) {
            return service.direct_request(request).await;
        }
        let retry = request
            .try_clone()
            .ok_or_else(|| AppError::Configuration("校园服务请求正文无法安全重试".to_owned()))?;
        match service.direct_request(request).await {
            Ok(response) => Ok(response),
            Err(AppError::Network(_)) => service.webvpn_request(retry).await,
            Err(error) => Err(error),
        }
    }

    pub async fn routed_request_pair(
        &self,
        first: Request,
        second: Request,
        use_webvpn: bool,
    ) -> Result<(Response, Response), AppError> {
        let service = self.authenticated_service().await?;
        let first_url = first.url().clone();
        if !use_webvpn
            || !crate::webvpn::supports(&first_url)
            || !crate::webvpn::supports(second.url())
        {
            return futures::try_join!(
                service.direct_request(first),
                service.direct_request(second)
            );
        }

        if !service.direct_route_available(&first_url).await {
            return service.webvpn_request_pair(first, second).await;
        }

        let first_retry = first
            .try_clone()
            .ok_or_else(|| AppError::Configuration("校园服务请求正文无法安全重试".to_owned()))?;
        let second_retry = second
            .try_clone()
            .ok_or_else(|| AppError::Configuration("校园服务请求正文无法安全重试".to_owned()))?;
        match futures::try_join!(
            service.direct_request(first),
            service.direct_request(second)
        ) {
            Ok(responses) => Ok(responses),
            Err(AppError::Network(_)) => {
                service.remember_direct_route(&first_url, false);
                service.webvpn_request_pair(first_retry, second_retry).await
            }
            Err(error) => Err(error),
        }
    }

    /// Fetch this student's timetable via the live campus session.
    ///
    /// The user never picks a student type: we try the stored guess first and
    /// transparently fall back to the other system (a postgraduate has no
    /// record in the undergraduate JWGL and vice versa), remembering what
    /// worked for next time.
    pub async fn load_timetable(&self) -> Result<crate::Timetable, AppError> {
        let mut credentials = self
            .store
            .load()?
            .ok_or_else(|| AppError::Auth("尚未登录复旦 UIS".to_owned()))?;
        let service = self.authenticated_service().await?;

        match service.fetch_timetable(credentials.is_graduate).await {
            Ok(timetable) => Ok(timetable),
            Err(first_error) => {
                let flipped = !credentials.is_graduate;
                match service.fetch_timetable(flipped).await {
                    Ok(timetable) => {
                        credentials.is_graduate = flipped;
                        let _ = self.store.save(&credentials);
                        Ok(timetable)
                    }
                    Err(_) => Err(first_error),
                }
            }
        }
    }
}

/// Merge campus status into the UI-facing session status.
pub fn merge_campus_status(base: SessionStatus, campus: &CampusStatus) -> SessionStatus {
    SessionStatus {
        community_logged_in: base.community_logged_in,
        community_user: base.community_user,
        campus_logged_in: campus.logged_in,
        campus_id: campus.id.clone(),
        campus_name: campus.name.clone(),
    }
}

fn encrypt_password(public_key_der: &[u8], password: &str) -> Result<String, AppError> {
    use rsa::pkcs8::DecodePublicKey;
    use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

    let public_key = RsaPublicKey::from_public_key_der(public_key_der)
        .map_err(|_| AppError::Upstream("认证公钥无法解析".to_owned()))?;
    let mut rng = rand_core::OsRng;
    let encrypted = public_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, password.as_bytes())
        .map_err(|_| AppError::Upstream("密码加密失败".to_owned()))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(encrypted))
}

/// Map the authExecute response to a user-facing result, as the Flutter client does.
fn auth_execute_error(body: &Value) -> Result<(), AppError> {
    let message = body.get("message").and_then(Value::as_str).unwrap_or("");
    let code = body.get("code").map(Value::to_string).unwrap_or_default();

    if message.contains("请输入验证码") {
        Err(AppError::Auth(
            "触发验证码保护，请稍后重试或先在网页端完成登录".to_owned(),
        ))
    } else if message.contains("用户名或密码错误") {
        Err(AppError::Auth("学号或密码不正确".to_owned()))
    } else if code.contains("200") || code.contains("0") {
        Ok(())
    } else if message.is_empty() {
        Err(AppError::Upstream("认证失败".to_owned()))
    } else {
        Err(AppError::Upstream(format!("认证失败：{message}")))
    }
}

fn host_of(url: &str) -> Result<String, AppError> {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_owned))
        .ok_or_else(|| AppError::Configuration(format!("无效的服务地址：{url}")))
}

/// Resolve the auto-submit page's redirect URL.
///
/// The identity host answers an authenticated service hop with a tiny page
/// whose script carries the ticket: `var locationValue = "…&ticket=ST-…"`.
/// A `<form id="logon">` variant exists as fallback on some services.
fn ticket_redirect(body: &str) -> Result<reqwest::Url, AppError> {
    if let Some(start) = body.find("var locationValue = \"") {
        let value_start = start + "var locationValue = \"".len();
        let value = &body[value_start..];
        let end = value
            .find('"')
            .ok_or_else(|| AppError::Upstream("认证跳转脚本格式无法解析".to_owned()))?;
        let url = value[..end].replace("&amp;", "&");
        return reqwest::Url::parse(&url)
            .map_err(|_| AppError::Upstream("认证跳转地址无效".to_owned()));
    }
    let (action, ticket) = logon_form(body)?;
    let mut url = action;
    url.set_query(None);
    url.query_pairs_mut().append_pair("ticket", &ticket);
    Ok(url)
}

/// Extract the `#logon` auto-submit form's action and ticket.
fn logon_form(body: &str) -> Result<(reqwest::Url, String), AppError> {
    let form_start = body
        .find("id=\"logon\"")
        .and_then(|index| body[..index].rfind('<'))
        .ok_or_else(|| AppError::Auth("校园会话已失效，请重新登录".to_owned()))?;
    let form_end = body[form_start..]
        .find('>')
        .map(|index| form_start + index + 1)
        .unwrap_or(body.len());
    let form_tag = &body[form_start..form_end];
    let action = attribute(form_tag, "action")
        .ok_or_else(|| AppError::Upstream("认证表单缺少提交地址".to_owned()))?;
    let action = reqwest::Url::parse(&format!("https://{}", DEFAULT_ID_HOST))
        .ok()
        .and_then(|base| base.join(&action).ok())
        .or_else(|| reqwest::Url::parse(&action).ok())
        .ok_or_else(|| AppError::Upstream("认证表单提交地址无效".to_owned()))?;
    let ticket = body
        .find("id=\"ticket\"")
        .and_then(|index| body[..index].rfind('<'))
        .and_then(|input_start| {
            let tag = &body[input_start
                ..body[input_start..]
                    .find('>')
                    .map(|i| input_start + i + 1)
                    .unwrap_or(body.len())];
            attribute(tag, "value")
        })
        .ok_or_else(|| AppError::Auth("校园会话已失效，请重新登录".to_owned()))?;
    Ok((action, ticket))
}

/// The default semester id is the first option of `#allSemesters`.
fn semester_id_from_html(page: &str) -> Option<String> {
    let select = page.find("id=\"allSemesters\"")?;
    let option = page[select..].find("<option")? + select;
    let tag_end = page[option..].find('>')? + option;
    attribute(&page[option..tag_end], "value")
}

fn ehall_user_name(page: &str) -> Option<String> {
    let marker = page.find("window.userInfoDSL")?;
    let object_start = page[marker..].find('{')? + marker;
    let mut values = serde_json::Deserializer::from_str(&page[object_start..]).into_iter::<Value>();
    values
        .next()?
        .ok()?
        .get("name")?
        .as_str()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

fn attribute(tag: &str, key: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let pattern = format!("{key}={quote}");
        if let Some(start) = tag.find(&pattern) {
            let value_start = start + pattern.len();
            let value = &tag[value_start..];
            if let Some(end) = value.find(quote) {
                return Some(value[..end].replace("&amp;", "&"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct TestCampusStore {
        credentials: Mutex<Option<CampusCredentials>>,
    }

    impl CampusCredentialStore for TestCampusStore {
        fn load(&self) -> Result<Option<CampusCredentials>, AppError> {
            Ok(self
                .credentials
                .lock()
                .map_err(|_| AppError::Storage("test store lock poisoned".to_owned()))?
                .clone())
        }

        fn save(&self, credentials: &CampusCredentials) -> Result<(), AppError> {
            *self
                .credentials
                .lock()
                .map_err(|_| AppError::Storage("test store lock poisoned".to_owned()))? =
                Some(credentials.clone());
            Ok(())
        }

        fn clear(&self) -> Result<(), AppError> {
            *self
                .credentials
                .lock()
                .map_err(|_| AppError::Storage("test store lock poisoned".to_owned()))? = None;
            Ok(())
        }
    }

    #[test]
    fn auth_execute_error_maps_markers() {
        assert!(
            auth_execute_error(&serde_json::json!({
                "code": 4020, "message": "认证失败,您还有3次重试机会。原因分析： 用户名或密码错误 "
            }))
            .is_err()
        );
        assert!(
            auth_execute_error(&serde_json::json!({
                "code": "200", "message": "操作成功!"
            }))
            .is_ok()
        );
    }

    #[test]
    fn parses_ehall_user_name() {
        let page = r#"<script>
          window.userInfoDSL = {"name":"张三","identity":"本科生","depart":"计算机科学技术学院"};
        </script>"#;
        assert_eq!(ehall_user_name(page).as_deref(), Some("张三"));
    }

    #[test]
    fn local_status_uses_stored_identity_without_authentication() {
        let store = Arc::new(TestCampusStore::default());
        store
            .save(&CampusCredentials {
                id: "23300000000".to_owned(),
                password: "not-used".to_owned(),
                name: Some("张三".to_owned()),
                is_graduate: false,
            })
            .expect("credentials should be stored");
        let session = CampusSession::new(store).expect("campus session should initialize");

        let status = session.local_status().expect("local status should load");

        assert!(status.logged_in);
        assert_eq!(status.id.as_deref(), Some("23300000000"));
        assert_eq!(status.name.as_deref(), Some("张三"));
    }

    #[test]
    fn serializes_second_factor_as_structured_login_state() {
        let value = serde_json::to_value(CampusLoginResult::RequiresSecondFactor {
            message: "需要验证".to_owned(),
        })
        .expect("login result should serialize");

        assert_eq!(value["state"], "requiresSecondFactor");
        assert_eq!(value["message"], "需要验证");
    }
}
