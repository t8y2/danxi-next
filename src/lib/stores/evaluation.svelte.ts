import { backend, toTransportError } from "$lib/api";
import { communityNetwork } from "$lib/stores/community-network.svelte";
import type {
  EvaluationCourseDetail,
  EvaluationCourseGroup,
  EvaluationReview,
} from "$lib/types/app";

export type EvaluationResource<T> =
  | { phase: "idle" }
  | { phase: "loading" }
  | { phase: "unauthenticated" }
  | { phase: "error"; message: string }
  | { phase: "ready"; data: T };

export class EvaluationStore {
  query = $state("");
  randomReview = $state<EvaluationResource<EvaluationReview>>({ phase: "idle" });
  searchResults = $state<EvaluationResource<EvaluationCourseGroup[]>>({ phase: "idle" });
  courseDetail = $state<EvaluationResource<EvaluationCourseDetail>>({ phase: "idle" });
  selectedGroupId = $state<number | null>(null);
  #randomRequest = 0;
  #searchRequest = 0;
  #detailRequest = 0;

  requireLogin() {
    this.#randomRequest += 1;
    this.#searchRequest += 1;
    this.#detailRequest += 1;
    this.randomReview = { phase: "unauthenticated" };
    this.searchResults = { phase: "unauthenticated" };
    this.courseDetail = { phase: "unauthenticated" };
    this.selectedGroupId = null;
  }

  async loadRandom(force = false) {
    if (
      !force &&
      this.randomReview.phase !== "idle" &&
      this.randomReview.phase !== "unauthenticated"
    ) {
      return;
    }
    const request = ++this.#randomRequest;
    this.randomReview = { phase: "loading" };
    try {
      const data = await backend.loadRandomEvaluationReview({
        useWebvpn: communityNetwork.useWebvpn,
      });
      if (request === this.#randomRequest) this.randomReview = { phase: "ready", data };
    } catch (raw) {
      if (request !== this.#randomRequest) return;
      this.randomReview = errorState(raw);
    }
  }

  async search(query: string) {
    const normalized = query.trim();
    this.query = normalized;
    this.#detailRequest += 1;
    if (!normalized) {
      this.#searchRequest += 1;
      this.searchResults = { phase: "idle" };
      this.courseDetail = { phase: "idle" };
      this.selectedGroupId = null;
      await this.loadRandom();
      return;
    }

    const request = ++this.#searchRequest;
    this.searchResults = { phase: "loading" };
    this.courseDetail = { phase: "idle" };
    this.selectedGroupId = null;
    try {
      const data = await backend.searchEvaluationCourses(normalized, 1, 20, {
        useWebvpn: communityNetwork.useWebvpn,
      });
      if (request !== this.#searchRequest) return;
      this.searchResults = { phase: "ready", data };
      if (data[0]) await this.selectGroup(data[0].groupId);
    } catch (raw) {
      if (request !== this.#searchRequest) return;
      this.searchResults = errorState(raw);
    }
  }

  async selectGroup(groupId: number) {
    this.selectedGroupId = groupId;
    const request = ++this.#detailRequest;
    this.courseDetail = { phase: "loading" };
    try {
      const data = await backend.loadEvaluationCourse(groupId, {
        useWebvpn: communityNetwork.useWebvpn,
      });
      if (request === this.#detailRequest && this.selectedGroupId === groupId) {
        this.courseDetail = { phase: "ready", data };
      }
    } catch (raw) {
      if (request !== this.#detailRequest || this.selectedGroupId !== groupId) return;
      this.courseDetail = errorState(raw);
    }
  }
}

function errorState<T>(raw: unknown): EvaluationResource<T> {
  const error = toTransportError(raw);
  return error.kind === "auth"
    ? { phase: "unauthenticated" }
    : { phase: "error", message: error.message };
}

export const evaluation = new EvaluationStore();
