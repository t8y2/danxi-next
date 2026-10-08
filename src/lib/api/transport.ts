import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  BackendError,
  CampusBus,
  CampusLoginResult,
  CampusLocation,
  CampusStatus,
  DiningCrowdedness,
  EmptyClassroom,
  EvaluationCourseDetail,
  EvaluationCourseGroup,
  EvaluationReview,
  ForumDivision,
  ForumFloor,
  ForumHole,
  ForumReaction,
  ForumTag,
  ForumThreadPage,
  HoleListQuery,
  SessionStatus,
  LibraryOccupancy,
  Timetable,
} from "$lib/types/app";

const WEB_REQUEST_TIMEOUT_MS = 45_000;

export interface BackendTransport {
  sessionStatus(options?: SessionStatusOptions): Promise<SessionStatus>;
  loginCommunity(
    email: string,
    password: string,
    options?: CommunityNetworkOptions,
  ): Promise<SessionStatus>;
  registerCommunity(
    email: string,
    password: string,
    verificationCode: string,
    options?: CommunityNetworkOptions,
  ): Promise<SessionStatus>;
  checkEmailRegistered(email: string, options?: CommunityNetworkOptions): Promise<boolean>;
  sendVerificationCode(email: string, options?: CommunityNetworkOptions): Promise<void>;
  campusLogin(id: string, password: string): Promise<CampusLoginResult>;
  completeCampusSecondFactor(): Promise<CampusStatus>;
  campusLogout(): Promise<void>;
  loadTimetable(): Promise<Timetable>;
  loadLibraryOccupancy(): Promise<LibraryOccupancy[]>;
  loadDiningCrowdedness(campus: CampusLocation): Promise<DiningCrowdedness>;
  beginDiningEnhancedAuth(): Promise<void>;
  loadCampusBuses(holiday: boolean): Promise<CampusBus[]>;
  loadEmptyClassrooms(
    building: string,
    date: string,
    options?: CommunityNetworkOptions,
  ): Promise<EmptyClassroom[]>;
  logoutCommunity(): Promise<void>;
  loadForumHoles(
    query?: HoleListQuery,
    options?: CommunityNetworkOptions,
  ): Promise<ForumHole[]>;
  loadForumThread(
    holeId: number,
    offset?: number,
    size?: number,
    options?: CommunityNetworkOptions,
  ): Promise<ForumThreadPage>;
  loadForumDivisions(options?: CommunityNetworkOptions): Promise<ForumDivision[]>;
  loadForumTags(options?: CommunityNetworkOptions): Promise<ForumTag[]>;
  createForumHole(
    divisionId: number,
    content: string,
    tags: ForumTag[],
    options?: CommunityNetworkOptions,
  ): Promise<void>;
  createForumFloor(
    holeId: number,
    content: string,
    options?: CommunityNetworkOptions,
  ): Promise<void>;
  reactForumFloor(
    floorId: number,
    reaction: ForumReaction,
    options?: CommunityNetworkOptions,
  ): Promise<ForumFloor>;
  loadForumFavoriteIds(options?: CommunityNetworkOptions): Promise<number[]>;
  setForumFavorite(
    holeId: number,
    favorite: boolean,
    options?: CommunityNetworkOptions,
  ): Promise<void>;
  reportForumFloor(
    floorId: number,
    reason: string,
    options?: CommunityNetworkOptions,
  ): Promise<void>;
  searchEvaluationCourses(
    query: string,
    page?: number,
    pageSize?: number,
    options?: CommunityNetworkOptions,
  ): Promise<EvaluationCourseGroup[]>;
  loadEvaluationCourse(
    groupId: number,
    options?: CommunityNetworkOptions,
  ): Promise<EvaluationCourseDetail>;
  loadRandomEvaluationReview(
    options?: CommunityNetworkOptions,
  ): Promise<EvaluationReview>;
}

export interface CommunityNetworkOptions {
  useWebvpn: boolean;
}

export interface SessionStatusOptions extends CommunityNetworkOptions {
  validate?: boolean;
}

export class TransportError extends Error {
  constructor(
    message: string,
    readonly kind: string,
  ) {
    super(message);
    this.name = "TransportError";
  }
}

/** Normalize Tauri command / HTTP errors into a typed error. */
export function toTransportError(raw: unknown): TransportError {
  if (raw instanceof TransportError) return raw;
  const detail = raw as BackendError | string | undefined;
  if (detail && typeof detail === "object" && typeof detail.message === "string") {
    return new TransportError(detail.message, detail.kind ?? "unknown");
  }
  if (typeof detail === "string" && detail.trim().length > 0) {
    return new TransportError(detail, "unknown");
  }
  return new TransportError("请求失败，请稍后重试", "unknown");
}

class TauriTransport implements BackendTransport {
  sessionStatus(options?: SessionStatusOptions) {
    return invoke<SessionStatus>("session_status", { request: sessionStatusOptions(options) });
  }

  loginCommunity(email: string, password: string, options?: CommunityNetworkOptions) {
    return invoke<SessionStatus>("community_login", {
      request: { email, password, ...networkOptions(options) },
    });
  }

  registerCommunity(
    email: string,
    password: string,
    verificationCode: string,
    options?: CommunityNetworkOptions,
  ) {
    return invoke<SessionStatus>("community_register", {
      request: { email, password, verificationCode, ...networkOptions(options) },
    });
  }

  checkEmailRegistered(email: string, options?: CommunityNetworkOptions) {
    return invoke<boolean>("community_check_email", {
      request: { email, ...networkOptions(options) },
    });
  }

  sendVerificationCode(email: string, options?: CommunityNetworkOptions) {
    return invoke<void>("community_send_verify_code", {
      request: { email, ...networkOptions(options) },
    });
  }

  campusLogin(id: string, password: string) {
    return invoke<CampusLoginResult>("campus_login", { request: { id, password } });
  }

  completeCampusSecondFactor() {
    return invoke<CampusStatus>("complete_campus_second_factor");
  }

  loadTimetable() {
    return invoke<Timetable>("load_timetable");
  }

  loadLibraryOccupancy() {
    return invoke<LibraryOccupancy[]>("load_library_occupancy");
  }

  loadDiningCrowdedness(campus: CampusLocation) {
    return invoke<DiningCrowdedness>("load_dining_crowdedness", { request: { campus } });
  }

  beginDiningEnhancedAuth() {
    return invoke<void>("begin_dining_enhanced_auth");
  }

  loadCampusBuses(holiday: boolean) {
    return invoke<CampusBus[]>("load_campus_buses", { request: { holiday } });
  }

  loadEmptyClassrooms(
    building: string,
    date: string,
    options?: CommunityNetworkOptions,
  ) {
    return invoke<EmptyClassroom[]>("load_empty_classrooms", {
      request: { building, date, ...networkOptions(options) },
    });
  }

  async campusLogout() {
    await invoke("campus_logout");
  }

  async logoutCommunity() {
    await invoke("community_logout");
  }

  loadForumHoles(query: HoleListQuery = {}, options?: CommunityNetworkOptions) {
    return invoke<ForumHole[]>("load_forum_holes", {
      request: { ...query, ...networkOptions(options) },
    });
  }

  loadForumThread(
    holeId: number,
    offset = 0,
    size = 10,
    options?: CommunityNetworkOptions,
  ) {
    return invoke<ForumThreadPage>("load_forum_thread", {
      request: { holeId, offset, size, ...networkOptions(options) },
    });
  }

  loadForumDivisions(options?: CommunityNetworkOptions) {
    return invoke<ForumDivision[]>("load_forum_divisions", {
      request: networkOptions(options),
    });
  }

  loadForumTags(options?: CommunityNetworkOptions) {
    return invoke<ForumTag[]>("load_forum_tags", { request: networkOptions(options) });
  }

  createForumHole(
    divisionId: number,
    content: string,
    tags: ForumTag[],
    options?: CommunityNetworkOptions,
  ) {
    return invoke<void>("create_forum_hole", {
      request: { divisionId, content, tags, ...networkOptions(options) },
    });
  }

  createForumFloor(holeId: number, content: string, options?: CommunityNetworkOptions) {
    return invoke<void>("create_forum_floor", {
      request: { holeId, content, ...networkOptions(options) },
    });
  }

  reactForumFloor(
    floorId: number,
    reaction: ForumReaction,
    options?: CommunityNetworkOptions,
  ) {
    return invoke<ForumFloor>("react_forum_floor", {
      request: { floorId, reaction, ...networkOptions(options) },
    });
  }

  loadForumFavoriteIds(options?: CommunityNetworkOptions) {
    return invoke<number[]>("load_forum_favorite_ids", {
      request: networkOptions(options),
    });
  }

  setForumFavorite(holeId: number, favorite: boolean, options?: CommunityNetworkOptions) {
    return invoke<void>("set_forum_favorite", {
      request: { holeId, favorite, ...networkOptions(options) },
    });
  }

  reportForumFloor(floorId: number, reason: string, options?: CommunityNetworkOptions) {
    return invoke<void>("report_forum_floor", {
      request: { floorId, reason, ...networkOptions(options) },
    });
  }

  searchEvaluationCourses(
    query: string,
    page = 1,
    pageSize = 20,
    options?: CommunityNetworkOptions,
  ) {
    return invoke<EvaluationCourseGroup[]>("search_evaluation_courses", {
      request: { query, page, pageSize, ...networkOptions(options) },
    });
  }

  loadEvaluationCourse(groupId: number, options?: CommunityNetworkOptions) {
    return invoke<EvaluationCourseDetail>("load_evaluation_course", {
      request: { groupId, ...networkOptions(options) },
    });
  }

  loadRandomEvaluationReview(options?: CommunityNetworkOptions) {
    return invoke<EvaluationReview>("load_random_evaluation_review", {
      request: networkOptions(options),
    });
  }
}

class WebTransport implements BackendTransport {
  constructor(private readonly baseUrl: string) {}

  sessionStatus(options?: SessionStatusOptions) {
    const query = new URLSearchParams({
      use_webvpn: String(networkOptions(options).useWebvpn),
      validate: String(options?.validate ?? true),
    });
    return this.get<SessionStatus>(`/v1/session/status?${query.toString()}`);
  }

  loginCommunity(email: string, password: string, options?: CommunityNetworkOptions) {
    return this.post<SessionStatus>("/v1/session/login", {
      email,
      password,
      ...networkOptions(options),
    });
  }

  registerCommunity(
    email: string,
    password: string,
    verificationCode: string,
    options?: CommunityNetworkOptions,
  ) {
    return this.post<SessionStatus>("/v1/session/register", {
      email,
      password,
      verificationCode,
      ...networkOptions(options),
    });
  }

  async checkEmailRegistered(email: string, options?: CommunityNetworkOptions) {
    const body = await this.post<{ registered: boolean }>("/v1/session/check-email", {
      email,
      ...networkOptions(options),
    });
    return body.registered;
  }

  async sendVerificationCode(email: string, options?: CommunityNetworkOptions) {
    await this.post<void>("/v1/session/verify-code", {
      email,
      ...networkOptions(options),
    });
  }

  campusLogin(id: string, password: string) {
    return this.post<CampusLoginResult>("/v1/session/campus/login", { id, password });
  }

  completeCampusSecondFactor(): Promise<CampusStatus> {
    return Promise.reject(
      new TransportError("浏览器版暂不支持复旦双因素认证，请使用桌面客户端", "unsupported"),
    );
  }

  loadTimetable() {
    return this.get<Timetable>("/v1/campus/timetable");
  }

  loadLibraryOccupancy() {
    return this.get<LibraryOccupancy[]>("/v1/campus/library");
  }

  loadDiningCrowdedness(campus: CampusLocation) {
    return this.get<DiningCrowdedness>(`/v1/campus/dining?campus=${campus}`);
  }

  beginDiningEnhancedAuth(): Promise<void> {
    return Promise.reject(
      new TransportError("浏览器版暂不支持复旦双因素认证，请使用桌面客户端", "unsupported"),
    );
  }

  loadCampusBuses(holiday: boolean) {
    return this.get<CampusBus[]>(`/v1/campus/buses?holiday=${holiday}`);
  }

  loadEmptyClassrooms(
    building: string,
    date: string,
    options?: CommunityNetworkOptions,
  ) {
    const params = new URLSearchParams({
      building,
      date,
      use_webvpn: String(networkOptions(options).useWebvpn),
    });
    return this.get<EmptyClassroom[]>(`/v1/campus/classrooms?${params.toString()}`);
  }

  async campusLogout() {
    await this.post<void>("/v1/session/campus/logout", {});
  }

  async logoutCommunity() {
    await this.post<void>("/v1/session/logout", {});
  }

  loadForumHoles(query: HoleListQuery = {}, options?: CommunityNetworkOptions) {
    const params = new URLSearchParams();
    if (query.divisionId != null) params.set("division_id", String(query.divisionId));
    if (query.size != null) params.set("size", String(query.size));
    if (query.order) params.set("order", query.order);
    if (query.before) params.set("before", query.before);
    params.set("use_webvpn", String(networkOptions(options).useWebvpn));
    const suffix = params.size > 0 ? `?${params.toString()}` : "";
    return this.get<ForumHole[]>(`/v1/forum/holes${suffix}`);
  }

  loadForumThread(
    holeId: number,
    offset = 0,
    size = 10,
    options?: CommunityNetworkOptions,
  ) {
    const params = new URLSearchParams({
      offset: String(offset),
      size: String(size),
      use_webvpn: String(networkOptions(options).useWebvpn),
    });
    return this.get<ForumThreadPage>(`/v1/forum/holes/${holeId}?${params.toString()}`);
  }

  loadForumDivisions(options?: CommunityNetworkOptions) {
    return this.get<ForumDivision[]>(
      `/v1/forum/divisions?use_webvpn=${networkOptions(options).useWebvpn}`,
    );
  }

  loadForumTags(options?: CommunityNetworkOptions) {
    return this.get<ForumTag[]>(
      `/v1/forum/tags?use_webvpn=${networkOptions(options).useWebvpn}`,
    );
  }

  createForumHole(
    divisionId: number,
    content: string,
    tags: ForumTag[],
    options?: CommunityNetworkOptions,
  ) {
    return this.post<void>("/v1/forum/holes", {
      divisionId,
      content,
      tags,
      ...networkOptions(options),
    });
  }

  createForumFloor(holeId: number, content: string, options?: CommunityNetworkOptions) {
    return this.post<void>(`/v1/forum/holes/${holeId}/floors`, {
      content,
      ...networkOptions(options),
    });
  }

  reactForumFloor(
    floorId: number,
    reaction: ForumReaction,
    options?: CommunityNetworkOptions,
  ) {
    return this.post<ForumFloor>(`/v1/forum/floors/${floorId}/reaction`, {
      reaction,
      ...networkOptions(options),
    });
  }

  loadForumFavoriteIds(options?: CommunityNetworkOptions) {
    return this.get<number[]>(
      `/v1/forum/favorites?use_webvpn=${networkOptions(options).useWebvpn}`,
    );
  }

  setForumFavorite(holeId: number, favorite: boolean, options?: CommunityNetworkOptions) {
    const body = { holeId, ...networkOptions(options) };
    return favorite
      ? this.post<void>("/v1/forum/favorites", body)
      : this.delete<void>("/v1/forum/favorites", body);
  }

  reportForumFloor(floorId: number, reason: string, options?: CommunityNetworkOptions) {
    return this.post<void>("/v1/forum/reports", {
      floorId,
      reason,
      ...networkOptions(options),
    });
  }

  searchEvaluationCourses(
    query: string,
    page = 1,
    pageSize = 20,
    options?: CommunityNetworkOptions,
  ) {
    const params = new URLSearchParams({
      query,
      page: String(page),
      page_size: String(pageSize),
      use_webvpn: String(networkOptions(options).useWebvpn),
    });
    return this.get<EvaluationCourseGroup[]>(`/v1/evaluation/search?${params.toString()}`);
  }

  loadEvaluationCourse(groupId: number, options?: CommunityNetworkOptions) {
    return this.get<EvaluationCourseDetail>(
      `/v1/evaluation/course-groups/${groupId}?use_webvpn=${networkOptions(options).useWebvpn}`,
    );
  }

  loadRandomEvaluationReview(options?: CommunityNetworkOptions) {
    return this.get<EvaluationReview>(
      `/v1/evaluation/random?use_webvpn=${networkOptions(options).useWebvpn}`,
    );
  }

  private async request<T>(path: string, init?: RequestInit): Promise<T> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), WEB_REQUEST_TIMEOUT_MS);
    let response: Response;
    try {
      response = await fetch(`${this.baseUrl}${path}`, {
        ...init,
        headers: {
          Accept: "application/json",
          ...(init?.body ? { "Content-Type": "application/json" } : {}),
        },
        credentials: "include",
        cache: "no-store",
        signal: controller.signal,
      });
    } catch {
      throw new TransportError(
        controller.signal.aborted ? "请求超时，请检查网络后重试" : "无法连接后端服务，请稍后重试",
        "network",
      );
    } finally {
      clearTimeout(timeout);
    }

    if (!response.ok) {
      let kind = "unknown";
      let message = `Web API request failed with HTTP ${response.status}`;
      try {
        const body = (await response.json()) as BackendError;
        if (body && typeof body.message === "string") {
          kind = body.kind ?? kind;
          message = body.message;
        }
      } catch {
        // Non-JSON error body: keep the HTTP status message.
      }
      throw new TransportError(message, kind);
    }

    if (response.status === 204) return undefined as T;
    return response.json() as Promise<T>;
  }

  private get<T>(path: string): Promise<T> {
    return this.request<T>(path);
  }

  private post<T>(path: string, body: unknown): Promise<T> {
    return this.request<T>(path, { method: "POST", body: JSON.stringify(body) });
  }

  private delete<T>(path: string, body: unknown): Promise<T> {
    return this.request<T>(path, { method: "DELETE", body: JSON.stringify(body) });
  }
}

class PreviewTransport implements BackendTransport {
  async sessionStatus(_options?: SessionStatusOptions): Promise<SessionStatus> {
    return {
      communityLoggedIn: false,
      communityUser: null,
      campusLoggedIn: false,
      campusId: null,
      campusName: null,
    };
  }

  loginCommunity(
    _email: string,
    _password: string,
    _options?: CommunityNetworkOptions,
  ): Promise<SessionStatus> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  registerCommunity(
    _email: string,
    _password: string,
    _verificationCode: string,
    _options?: CommunityNetworkOptions,
  ): Promise<SessionStatus> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  checkEmailRegistered(_email: string, _options?: CommunityNetworkOptions): Promise<boolean> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  sendVerificationCode(_email: string, _options?: CommunityNetworkOptions): Promise<void> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  campusLogin(_id: string, _password: string): Promise<CampusLoginResult> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  completeCampusSecondFactor(): Promise<CampusStatus> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动桌面客户端", "unsupported"));
  }

  loadTimetable(): Promise<Timetable> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  loadLibraryOccupancy(): Promise<LibraryOccupancy[]> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  loadDiningCrowdedness(_campus: CampusLocation): Promise<DiningCrowdedness> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  beginDiningEnhancedAuth(): Promise<void> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动桌面客户端", "unsupported"));
  }

  loadCampusBuses(_holiday: boolean): Promise<CampusBus[]> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  loadEmptyClassrooms(
    _building: string,
    _date: string,
    _options?: CommunityNetworkOptions,
  ): Promise<EmptyClassroom[]> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  async campusLogout() {}

  async logoutCommunity() {}

  loadForumHoles(
    _query?: HoleListQuery,
    _options?: CommunityNetworkOptions,
  ): Promise<ForumHole[]> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  loadForumThread(
    _holeId: number,
    _offset?: number,
    _size?: number,
    _options?: CommunityNetworkOptions,
  ): Promise<ForumThreadPage> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  loadForumDivisions(_options?: CommunityNetworkOptions): Promise<ForumDivision[]> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  loadForumTags(_options?: CommunityNetworkOptions): Promise<ForumTag[]> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  createForumHole(
    _divisionId: number,
    _content: string,
    _tags: ForumTag[],
    _options?: CommunityNetworkOptions,
  ): Promise<void> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  createForumFloor(
    _holeId: number,
    _content: string,
    _options?: CommunityNetworkOptions,
  ): Promise<void> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  reactForumFloor(
    _floorId: number,
    _reaction: ForumReaction,
    _options?: CommunityNetworkOptions,
  ): Promise<ForumFloor> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  loadForumFavoriteIds(_options?: CommunityNetworkOptions): Promise<number[]> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  setForumFavorite(
    _holeId: number,
    _favorite: boolean,
    _options?: CommunityNetworkOptions,
  ): Promise<void> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  reportForumFloor(
    _floorId: number,
    _reason: string,
    _options?: CommunityNetworkOptions,
  ): Promise<void> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  searchEvaluationCourses(
    _query: string,
    _page?: number,
    _pageSize?: number,
    _options?: CommunityNetworkOptions,
  ): Promise<EvaluationCourseGroup[]> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  loadEvaluationCourse(
    _groupId: number,
    _options?: CommunityNetworkOptions,
  ): Promise<EvaluationCourseDetail> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }

  loadRandomEvaluationReview(
    _options?: CommunityNetworkOptions,
  ): Promise<EvaluationReview> {
    return Promise.reject(new TransportError("预览模式下不可用，请启动后端网关", "unsupported"));
  }
}

function networkOptions(options?: CommunityNetworkOptions): CommunityNetworkOptions {
  return { useWebvpn: options?.useWebvpn ?? true };
}

function sessionStatusOptions(options?: SessionStatusOptions): Required<SessionStatusOptions> {
  return {
    useWebvpn: options?.useWebvpn ?? true,
    validate: options?.validate ?? true,
  };
}

function createTransport(): BackendTransport {
  if (isTauri()) {
    return new TauriTransport();
  }

  const baseUrl = import.meta.env.VITE_DANXI_API_BASE_URL?.replace(/\/$/, "");
  if (baseUrl) {
    return new WebTransport(baseUrl);
  }

  if (import.meta.env.DEV) {
    return new WebTransport("/api");
  }

  return new PreviewTransport();
}

export const backend = createTransport();
