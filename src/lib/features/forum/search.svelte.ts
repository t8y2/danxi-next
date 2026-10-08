import { backend, toTransportError } from "$lib/api";
import type { CommunityNetworkOptions } from "$lib/api/transport";
import { communityNetwork } from "$lib/stores/community-network.svelte";
import type { ForumFloor, ForumSearchPage } from "$lib/types/app";
import { parseForumSearchInput, type ForumSearchIntent } from "./search";

export type ForumSearchState =
  | { phase: "idle" | "loading" | "unauthenticated" }
  | { phase: "error"; message: string }
  | { phase: "ready"; floors: ForumFloor[]; nextOffset: number | null; loadingMore: boolean; moreError: string | null };

type SearchLoader = (query: string, offset: number, options: CommunityNetworkOptions) => Promise<ForumSearchPage>;

export class ForumSearchController {
  input = $state("");
  query = $state("");
  inputError = $state<string | null>(null);
  state = $state<ForumSearchState>({ phase: "idle" });
  scrollTop = 0;
  private generation = 0;
  private readonly loadPage: SearchLoader;

  constructor(loadPage: SearchLoader = (query, offset, options) => backend.searchForumFloors(query, offset, options)) {
    this.loadPage = loadPage;
  }

  submit(): ForumSearchIntent {
    const intent = parseForumSearchInput(this.input);
    this.inputError = intent.kind === "invalid" ? intent.message : null;
    if (intent.kind === "empty") this.clear();
    if (intent.kind === "search") {
      this.input = intent.query;
      this.query = intent.query;
      void this.retry();
    }
    return intent;
  }

  clear() {
    this.generation += 1;
    this.input = "";
    this.query = "";
    this.inputError = null;
    this.state = { phase: "idle" };
    this.scrollTop = 0;
  }

  async retry() {
    if (!this.query) return;
    const generation = ++this.generation;
    this.scrollTop = 0;
    this.state = { phase: "loading" };
    try {
      const page = await this.loadPage(this.query, 0, { useWebvpn: communityNetwork.useWebvpn });
      if (generation !== this.generation) return;
      this.state = { phase: "ready", floors: this.uniqueFloors(page.floors), nextOffset: page.nextOffset, loadingMore: false, moreError: null };
    } catch (raw) {
      if (generation !== this.generation) return;
      this.fail(raw);
    }
  }

  async loadMore(retry = false) {
    if (this.state.phase !== "ready" || this.state.loadingMore || this.state.nextOffset == null || (this.state.moreError && !retry)) return;
    const generation = this.generation;
    const offset = this.state.nextOffset;
    this.state = { ...this.state, loadingMore: true, moreError: null };
    try {
      const page = await this.loadPage(this.query, offset, { useWebvpn: communityNetwork.useWebvpn });
      if (generation !== this.generation || this.state.phase !== "ready") return;
      this.state = {
        phase: "ready",
        floors: this.uniqueFloors([...this.state.floors, ...page.floors]),
        nextOffset: page.nextOffset != null && page.nextOffset > offset ? page.nextOffset : null,
        loadingMore: false,
        moreError: null,
      };
    } catch (raw) {
      if (generation !== this.generation || this.state.phase !== "ready") return;
      const error = toTransportError(raw);
      if (error.kind === "auth") this.fail(raw);
      else this.state = { ...this.state, loadingMore: false, moreError: error.message };
    }
  }

  private fail(raw: unknown) {
    const error = toTransportError(raw);
    this.state = error.kind === "auth" ? { phase: "unauthenticated" } : { phase: "error", message: error.message };
  }

  private uniqueFloors(floors: ForumFloor[]): ForumFloor[] {
    return [...new Map(floors.map((floor) => [floor.floorId, floor])).values()];
  }
}

export const forumSearch = new ForumSearchController();
