import { forumReferenceIds, type ForumReferenceTarget } from "./content.ts";

export interface ConversationFloor extends ForumReferenceTarget {
  complete?: boolean;
}

export interface ConversationEntry {
  floorId: number;
  floor: ConversationFloor | null;
  replyToIds: number[];
  depth: number;
  unavailable: boolean;
}

export interface ConversationResult {
  entries: ConversationEntry[];
  floors: ConversationFloor[];
  limited: boolean;
}

export interface ConversationRequest {
  sourceId: number;
  targetId: number;
  floors: ConversationFloor[];
  loadFloor: (floorId: number) => Promise<ForumReferenceTarget>;
  limit?: number;
  isActive?: () => boolean;
}

export async function loadConversation({
  sourceId, targetId, floors, loadFloor, limit = 30, isActive = () => true,
}: ConversationRequest): Promise<ConversationResult> {
  const known = new Map(floors.map((floor) => [floor.floorId, floor]));
  const includeMentions = (floor: ConversationFloor) => {
    if (floor.deleted) return;
    for (const mention of floor.mentions ?? []) {
      if (!known.has(mention.floorId)) known.set(mention.floorId, { ...mention, complete: false });
    }
  };
  floors.forEach(includeMentions);
  const related = new Set([targetId, sourceId]);
  let added = true;
  while (added && related.size < limit) {
    added = false;
    for (const floor of floors) {
      if (related.size >= limit) break;
      if (related.has(floor.floorId) || floor.deleted || floor.complete === false) continue;
      if (forumReferenceIds(floor.content).some((floorId) => related.has(floorId))) {
        related.add(floor.floorId);
        added = true;
      }
    }
  }
  const queue = [...related];
  const visited = new Set<number>();
  const unavailable = new Set<number>();
  const parents = new Map<number, number[]>();
  while (queue.length && visited.size < limit && isActive()) {
    const floorId = queue.shift()!;
    if (visited.has(floorId)) continue;
    visited.add(floorId);
    let floor = known.get(floorId);
    if (!floor || floor.complete === false) {
      try {
        const loaded = await loadFloor(floorId);
        if (!isActive()) break;
        if (loaded.floorId !== floorId) throw new Error("楼层响应不匹配");
        floor = { ...loaded, floorNumber: floor?.floorNumber, complete: true };
        known.set(floorId, floor);
        includeMentions(floor);
      } catch {
        unavailable.add(floorId);
      }
    }
    const references = floor && !floor.deleted && !unavailable.has(floorId) ? forumReferenceIds(floor.content) : [];
    parents.set(floorId, references);
    for (const referenceId of references) {
      if (!visited.has(referenceId) && !queue.includes(referenceId)) queue.push(referenceId);
    }
  }
  const entries: ConversationEntry[] = [];
  const ordered = new Set<number>();
  const visiting = new Set<number>();
  const depths = new Map<number, number>();
  const visit = (floorId: number) => {
    if (ordered.has(floorId) || visiting.has(floorId) || !visited.has(floorId)) return;
    visiting.add(floorId);
    const replyToIds = parents.get(floorId) ?? [];
    replyToIds.forEach(visit);
    visiting.delete(floorId);
    const parentDepths = replyToIds.filter((parentId) => depths.has(parentId)).map((parentId) => depths.get(parentId)!);
    const depth = parentDepths.length ? Math.min(2, Math.min(...parentDepths) + 1) : 0;
    depths.set(floorId, depth);
    ordered.add(floorId);
    entries.push({ floorId, floor: known.get(floorId) ?? null, replyToIds, depth, unavailable: unavailable.has(floorId) });
  };
  visited.forEach(visit);
  return { entries, floors: [...known.values()], limited: queue.length > 0 || related.size >= limit };
}
