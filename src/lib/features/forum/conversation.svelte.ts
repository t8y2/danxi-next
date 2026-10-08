import { backend } from "$lib/api";
import { communityNetwork } from "$lib/stores/community-network.svelte";
import { loadConversation, type ConversationFloor, type ConversationResult } from "./conversation";
import type { ForumReferenceTarget } from "./content";

export class ForumConversationController {
  opened = $state(false);
  loading = $state(false);
  sourceId = $state(0);
  targetId = $state(0);
  result = $state<ConversationResult>({ entries: [], floors: [], limited: false });
  private generation = 0;
  private limit = 30;
  private seed: ConversationFloor[] = [];
  private readonly fetchFloor: (floorId: number, useWebvpn: boolean) => Promise<ForumReferenceTarget>;

  constructor(fetchFloor = (floorId: number, useWebvpn: boolean) => backend.loadForumFloor(floorId, { useWebvpn })) {
    this.fetchFloor = fetchFloor;
  }

  open(sourceId: number, targetId: number, floors: ConversationFloor[]) {
    this.generation += 1;
    this.loading = false;
    this.sourceId = sourceId;
    this.targetId = targetId;
    this.seed = floors;
    this.limit = 30;
    this.result = { entries: [], floors: [], limited: false };
    this.opened = true;
    void this.load();
  }

  close() {
    this.generation += 1;
    this.opened = false;
    this.loading = false;
    this.seed = [];
    this.result = { entries: [], floors: [], limited: false };
  }

  async load(more = false) {
    if (this.loading || !this.opened) return;
    if (more) this.limit += 30;
    const generation = this.generation;
    const options = { useWebvpn: communityNetwork.useWebvpn };
    this.loading = true;
    try {
      const result = await loadConversation({
        sourceId: this.sourceId,
        targetId: this.targetId,
        floors: this.seed,
        limit: this.limit,
        loadFloor: (floorId) => this.fetchFloor(floorId, options.useWebvpn),
        isActive: () => generation === this.generation && this.opened,
      });
      if (generation !== this.generation) return;
      this.result = result;
      this.seed = result.floors;
    } finally {
      if (generation === this.generation) this.loading = false;
    }
  }
}
