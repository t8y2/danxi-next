import { backend, toTransportError } from "$lib/api";
import { communityNetwork } from "$lib/stores/community-network.svelte";
import type {
  ForumDivision,
  ForumFloor,
  ForumHole,
  ForumReaction,
  ForumTag,
  ForumThreadPage,
  SessionStatus,
} from "$lib/types/app";

const SESSION_SNAPSHOT_KEY = "danxi.session.public.v1";

const loggedOut: SessionStatus = {
  communityLoggedIn: false,
  communityUser: null,
  campusLoggedIn: false,
  campusId: null,
  campusName: null,
};

const initialStatus = loadSessionSnapshot();

function loadSessionSnapshot(): SessionStatus | null {
  if (typeof localStorage === "undefined") return null;
  try {
    const raw = localStorage.getItem(SESSION_SNAPSHOT_KEY);
    if (!raw) return null;
    const status = JSON.parse(raw) as unknown;
    return isSessionStatus(status) ? status : null;
  } catch {
    return null;
  }
}

function persistSessionSnapshot(status: SessionStatus) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(SESSION_SNAPSHOT_KEY, JSON.stringify(status));
  } catch {
    // Public session metadata is only a startup optimization.
  }
}

function isSessionStatus(value: unknown): value is SessionStatus {
  if (!value || typeof value !== "object") return false;
  const status = value as Partial<SessionStatus>;
  return (
    typeof status.communityLoggedIn === "boolean" &&
    typeof status.campusLoggedIn === "boolean" &&
    (status.communityUser === null || typeof status.communityUser === "object") &&
    (status.campusId === null || typeof status.campusId === "string") &&
    (status.campusName === null || typeof status.campusName === "string")
  );
}

/**
 * Community session state shared by the settings panel, the login dialog and
 * the forum panel. Credentials only pass through here on their way to the
 * transport; they are never kept in this store.
 */
export class SessionStore {
  status = $state<SessionStatus>(initialStatus ?? loggedOut);
  ready = $state(initialStatus !== null);
  #refreshRequest = 0;
  #communityMutation = 0;
  #campusMutation = 0;

  async restore() {
    const request = ++this.#refreshRequest;
    const hadSnapshot = this.ready;
    const restored = await this.#restoreLocal(request);
    if (request !== this.#refreshRequest) return;
    void this.#verify(request, restored || hadSnapshot);
  }

  async refresh() {
    const request = ++this.#refreshRequest;
    const hadSnapshot = this.ready;
    const restored = await this.#restoreLocal(request);
    if (request !== this.#refreshRequest) return;
    await this.#verify(request, restored || hadSnapshot);
  }

  async #restoreLocal(request: number) {
    try {
      const status = await backend.sessionStatus({
        useWebvpn: communityNetwork.useWebvpn,
        validate: false,
      });
      if (request !== this.#refreshRequest) return false;
      this.#setStatus(status);
      return true;
    } catch {
      return false;
    }
  }

  async #verify(request: number, restored: boolean) {
    try {
      const status = await backend.sessionStatus({
        useWebvpn: communityNetwork.useWebvpn,
        validate: true,
      });
      if (request === this.#refreshRequest) this.#setStatus(status);
    } catch {
      if (request === this.#refreshRequest && !restored) this.#setStatus(loggedOut);
    } finally {
      if (request === this.#refreshRequest) this.ready = true;
    }
  }

  async login(email: string, password: string) {
    const request = ++this.#communityMutation;
    this.#refreshRequest += 1;
    const status = await backend.loginCommunity(email, password, {
      useWebvpn: communityNetwork.useWebvpn,
    });
    if (request !== this.#communityMutation) return status;
    return this.#applyCommunityStatus(status);
  }

  async register(email: string, password: string, verificationCode: string) {
    const request = ++this.#communityMutation;
    this.#refreshRequest += 1;
    const status = await backend.registerCommunity(email, password, verificationCode, {
      useWebvpn: communityNetwork.useWebvpn,
    });
    if (request !== this.#communityMutation) return status;
    return this.#applyCommunityStatus(status);
  }

  async loginCampus(id: string, password: string) {
    const request = ++this.#campusMutation;
    this.#refreshRequest += 1;
    const result = await backend.campusLogin(id, password);
    if (request !== this.#campusMutation) return result;
    if (result.state !== "authenticated") return result;
    const status = result.status;
    this.#setStatus({
      ...this.status,
      campusLoggedIn: status.loggedIn,
      campusId: status.id,
      campusName: status.name,
    });
    return result;
  }

  async completeCampusSecondFactor() {
    const request = ++this.#campusMutation;
    this.#refreshRequest += 1;
    const status = await backend.completeCampusSecondFactor();
    if (request !== this.#campusMutation) return status;
    this.#setStatus({
      ...this.status,
      campusLoggedIn: status.loggedIn,
      campusId: status.id,
      campusName: status.name,
    });
    return status;
  }

  async logoutCampus() {
    const request = ++this.#campusMutation;
    this.#refreshRequest += 1;
    await backend.campusLogout();
    if (request !== this.#campusMutation) return;
    this.#setStatus({
      ...this.status,
      campusLoggedIn: false,
      campusId: null,
      campusName: null,
    });
  }

  async checkEmailRegistered(email: string) {
    return backend.checkEmailRegistered(email, { useWebvpn: communityNetwork.useWebvpn });
  }

  async sendVerificationCode(email: string) {
    return backend.sendVerificationCode(email, { useWebvpn: communityNetwork.useWebvpn });
  }

  async logout() {
    const request = ++this.#communityMutation;
    this.#refreshRequest += 1;
    await backend.logoutCommunity();
    if (request !== this.#communityMutation) return;
    this.#setStatus({
      ...this.status,
      communityLoggedIn: false,
      communityUser: null,
    });
  }

  #applyCommunityStatus(status: SessionStatus): SessionStatus {
    this.#setStatus({
      ...this.status,
      communityLoggedIn: status.communityLoggedIn,
      communityUser: status.communityUser,
    });
    return this.status;
  }

  #setStatus(status: SessionStatus) {
    this.status = status;
    this.ready = true;
    persistSessionSnapshot(status);
  }
}

export const session = new SessionStore();

export type ForumState =
  | { phase: "idle" }
  | { phase: "loading" }
  | { phase: "unauthenticated" }
  | { phase: "error"; message: string }
  | {
      phase: "ready";
      holes: ForumHole[];
      order: "time_updated" | "time_created";
      divisionId: number | null;
      loadingMore: boolean;
      hasMore: boolean;
      moreError: string | null;
    };

const idleForum: ForumState = { phase: "idle" };

export type ForumDetailState =
  | { phase: "idle" }
  | { phase: "loading"; holeId: number }
  | { phase: "error"; holeId: number; message: string }
  | {
      phase: "ready";
      thread: ForumThreadPage;
      loadingMore: boolean;
      moreError: string | null;
    };

export type ForumMetaState =
  | { phase: "idle" }
  | { phase: "loading" }
  | { phase: "error"; message: string }
  | { phase: "ready"; divisions: ForumDivision[]; tags: ForumTag[] };

export type ForumActionResult = { ok: true } | { ok: false; message: string };

/** Hole listing state for the forum panel. */
export class ForumStore {
  state = $state<ForumState>(idleForum);
  detail = $state<ForumDetailState>({ phase: "idle" });
  meta = $state<ForumMetaState>({ phase: "idle" });
  favoriteIds = $state<number[] | null>(null);
  favoritesLoading = $state(false);
  favoriteBusy = $state(false);
  reactingFloorIds = $state<number[]>([]);
  actionError = $state<string | null>(null);
  private listRequest = 0;
  private detailRequest = 0;

  requireLogin() {
    this.listRequest += 1;
    this.detailRequest += 1;
    this.state = { phase: "unauthenticated" };
    this.detail = { phase: "idle" };
    this.meta = { phase: "idle" };
    this.favoriteIds = null;
    this.actionError = null;
  }

  async load(
    size = 10,
    order: "time_updated" | "time_created" = "time_updated",
    divisionId: number | null = null,
  ) {
    const request = ++this.listRequest;
    this.state = { phase: "loading" };
    try {
      const holes = await backend.loadForumHoles(
        { size, order, ...(divisionId == null ? {} : { divisionId }) },
        { useWebvpn: communityNetwork.useWebvpn },
      );
      if (request !== this.listRequest) return;
      this.state = {
        phase: "ready",
        holes,
        order,
        divisionId,
        loadingMore: false,
        hasMore: holes.length === size,
        moreError: null,
      };
    } catch (raw) {
      if (request !== this.listRequest) return;
      const error = toTransportError(raw);
      this.state =
        error.kind === "auth"
          ? { phase: "unauthenticated" }
          : { phase: "error", message: error.message };
    }
  }

  async loadMoreHoles() {
    if (
      this.state.phase !== "ready" ||
      this.state.loadingMore ||
      !this.state.hasMore ||
      this.state.moreError
    ) {
      return;
    }

    const lastHole = this.state.holes.at(-1);
    const before =
      this.state.order === "time_created" ? lastHole?.timeCreated : lastHole?.timeUpdated;
    if (!before) {
      this.state = { ...this.state, hasMore: false };
      return;
    }

    const request = this.listRequest;
    const order = this.state.order;
    const divisionId = this.state.divisionId;
    this.state = { ...this.state, loadingMore: true, moreError: null };
    try {
      const page = await backend.loadForumHoles(
        { size: 10, order, before, ...(divisionId == null ? {} : { divisionId }) },
        { useWebvpn: communityNetwork.useWebvpn },
      );
      if (request !== this.listRequest || this.state.phase !== "ready") return;
      const holeIds = new Set(this.state.holes.map((hole) => hole.holeId));
      const appended = page.filter((hole) => !holeIds.has(hole.holeId));
      this.state = {
        ...this.state,
        holes: [...this.state.holes, ...appended],
        loadingMore: false,
        hasMore: page.length === 10 && appended.length > 0,
        moreError: null,
      };
    } catch (raw) {
      if (request !== this.listRequest || this.state.phase !== "ready") return;
      const error = toTransportError(raw);
      if (error.kind === "auth") {
        this.state = { phase: "unauthenticated" };
        this.detail = { phase: "idle" };
      } else {
        this.state = { ...this.state, loadingMore: false, moreError: error.message };
      }
    }
  }

  retryMoreHoles() {
    if (this.state.phase !== "ready") return;
    this.state = { ...this.state, moreError: null };
    void this.loadMoreHoles();
  }

  async loadMeta(force = false) {
    if (!force && (this.meta.phase === "loading" || this.meta.phase === "ready")) return;
    this.meta = { phase: "loading" };
    try {
      const [divisions, tags] = await Promise.all([
        backend.loadForumDivisions({ useWebvpn: communityNetwork.useWebvpn }),
        backend
          .loadForumTags({ useWebvpn: communityNetwork.useWebvpn })
          .catch(() => [] as ForumTag[]),
      ]);
      this.meta = { phase: "ready", divisions, tags };
    } catch (raw) {
      const error = toTransportError(raw);
      this.meta = { phase: "error", message: error.message };
    }
  }

  async loadFavorites(force = false) {
    if (!force && (this.favoriteIds !== null || this.favoritesLoading)) return;
    this.favoritesLoading = true;
    try {
      this.favoriteIds = await backend.loadForumFavoriteIds({
        useWebvpn: communityNetwork.useWebvpn,
      });
    } catch (raw) {
      this.actionError = toTransportError(raw).message;
    } finally {
      this.favoritesLoading = false;
    }
  }

  isFavorite(holeId: number): boolean {
    return this.favoriteIds?.includes(holeId) ?? false;
  }

  async createHole(
    divisionId: number,
    content: string,
    tags: ForumTag[],
  ): Promise<ForumActionResult> {
    try {
      await backend.createForumHole(divisionId, content, tags, {
        useWebvpn: communityNetwork.useWebvpn,
      });
      return { ok: true };
    } catch (raw) {
      return { ok: false, message: toTransportError(raw).message };
    }
  }

  async createFloor(
    holeId: number,
    content: string,
    replyToFloorId?: number,
  ): Promise<ForumActionResult> {
    const body = replyToFloorId == null ? content : `##${replyToFloorId}\n${content}`;
    const currentReplyCount =
      this.detail.phase === "ready" && this.detail.thread.hole.holeId === holeId
        ? this.detail.thread.hole.reply
        : 0;
    try {
      await backend.createForumFloor(holeId, body, {
        useWebvpn: communityNetwork.useWebvpn,
      });
      const latestOffset = Math.max(0, currentReplyCount + 2 - 10);
      void this.open(holeId, latestOffset);
      if (this.state.phase === "ready") {
        void this.load(10, this.state.order, this.state.divisionId);
      }
      return { ok: true };
    } catch (raw) {
      return { ok: false, message: toTransportError(raw).message };
    }
  }

  async reactFloor(floor: ForumFloor, kind: "like" | "dislike") {
    if (this.reactingFloorIds.includes(floor.floorId)) return;
    const reaction: ForumReaction =
      kind === "like" ? (floor.liked ? 0 : 1) : floor.disliked ? 0 : -1;
    this.actionError = null;
    this.reactingFloorIds = [...this.reactingFloorIds, floor.floorId];
    try {
      const updated = await backend.reactForumFloor(floor.floorId, reaction, {
        useWebvpn: communityNetwork.useWebvpn,
      });
      this.replaceFloor(updated);
    } catch (raw) {
      this.actionError = toTransportError(raw).message;
    } finally {
      this.reactingFloorIds = this.reactingFloorIds.filter((id) => id !== floor.floorId);
    }
  }

  async toggleFavorite(holeId: number) {
    if (this.favoriteBusy) return;
    if (this.favoriteIds === null) await this.loadFavorites(true);
    if (this.favoriteIds === null) return;

    const wasFavorite = this.favoriteIds.includes(holeId);
    const nextFavorite = !wasFavorite;
    const previousIds = this.favoriteIds;
    this.actionError = null;
    this.favoriteBusy = true;
    this.favoriteIds = nextFavorite
      ? [...this.favoriteIds, holeId]
      : this.favoriteIds.filter((id) => id !== holeId);
    this.adjustFavoriteCount(holeId, nextFavorite ? 1 : -1);
    try {
      await backend.setForumFavorite(holeId, nextFavorite, {
        useWebvpn: communityNetwork.useWebvpn,
      });
    } catch (raw) {
      this.favoriteIds = previousIds;
      this.adjustFavoriteCount(holeId, nextFavorite ? -1 : 1);
      this.actionError = toTransportError(raw).message;
    } finally {
      this.favoriteBusy = false;
    }
  }

  async reportFloor(floorId: number, reason: string): Promise<ForumActionResult> {
    try {
      await backend.reportForumFloor(floorId, reason, {
        useWebvpn: communityNetwork.useWebvpn,
      });
      return { ok: true };
    } catch (raw) {
      return { ok: false, message: toTransportError(raw).message };
    }
  }

  async open(holeId: number, offset = 0) {
    const request = ++this.detailRequest;
    this.detail = { phase: "loading", holeId };
    this.actionError = null;
    try {
      const thread = await backend.loadForumThread(holeId, offset, 10, {
        useWebvpn: communityNetwork.useWebvpn,
      });
      if (request !== this.detailRequest) return;
      this.detail = { phase: "ready", thread, loadingMore: false, moreError: null };
    } catch (raw) {
      if (request !== this.detailRequest) return;
      const error = toTransportError(raw);
      if (error.kind === "auth") {
        this.state = { phase: "unauthenticated" };
        this.detail = { phase: "idle" };
      } else {
        this.detail = { phase: "error", holeId, message: error.message };
      }
    }
  }

  closeDetail() {
    this.detailRequest += 1;
    this.detail = { phase: "idle" };
  }

  async loadMore(force = false) {
    if (this.detail.phase !== "ready") return;
    const { thread } = this.detail;
    if (
      thread.nextOffset == null ||
      this.detail.loadingMore ||
      (this.detail.moreError && !force)
    ) {
      return;
    }

    const request = this.detailRequest;
    this.detail = { ...this.detail, loadingMore: true, moreError: null };
    try {
      const page = await backend.loadForumThread(thread.hole.holeId, thread.nextOffset, 10, {
        useWebvpn: communityNetwork.useWebvpn,
      });
      if (request !== this.detailRequest || this.detail.phase !== "ready") return;
      const floorIds = new Set(this.detail.thread.floors.map((floor) => floor.floorId));
      const appended = page.floors.filter((floor) => !floorIds.has(floor.floorId));
      this.detail = {
        phase: "ready",
        thread: {
          hole: page.hole,
          floors: [...this.detail.thread.floors, ...appended],
          offset: this.detail.thread.offset,
          nextOffset: appended.length > 0 ? page.nextOffset : null,
        },
        loadingMore: false,
        moreError: null,
      };
    } catch (raw) {
      if (request !== this.detailRequest || this.detail.phase !== "ready") return;
      const error = toTransportError(raw);
      this.detail = { ...this.detail, loadingMore: false, moreError: error.message };
    }
  }

  private replaceFloor(updated: ForumFloor) {
    if (this.detail.phase !== "ready") return;
    this.detail = {
      ...this.detail,
      thread: {
        ...this.detail.thread,
        floors: this.detail.thread.floors.map((floor) =>
          floor.floorId === updated.floorId ? updated : floor,
        ),
      },
    };
  }

  private adjustFavoriteCount(holeId: number, delta: number) {
    const updateHole = (hole: ForumHole): ForumHole =>
      hole.holeId === holeId
        ? { ...hole, favoriteCount: Math.max(0, hole.favoriteCount + delta) }
        : hole;
    if (this.state.phase === "ready") {
      this.state = { ...this.state, holes: this.state.holes.map(updateHole) };
    }
    if (this.detail.phase === "ready" && this.detail.thread.hole.holeId === holeId) {
      this.detail = {
        ...this.detail,
        thread: { ...this.detail.thread, hole: updateHole(this.detail.thread.hole) },
      };
    }
  }
}

export const forum = new ForumStore();
