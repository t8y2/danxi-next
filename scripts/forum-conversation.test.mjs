import assert from "node:assert/strict";
import test from "node:test";
import { loadConversation } from "../src/lib/features/forum/conversation.ts";
import { forumReferenceIds, forumReplyExcerpt } from "../src/lib/features/forum/content.ts";

function floor(floorId, content, overrides = {}) {
  return { floorId, holeId: 10, content, anonyname: `同学${floorId}`, deleted: false, mentions: [], ...overrides };
}

test("conversation orders ancestors first, includes loaded related replies, and caps indentation", async () => {
  const floors = [floor(1, "起点"), floor(2, "##1 第二层"), floor(3, "##2 第三层"), floor(4, "##3 第四层"), floor(5, "另一条对话")];
  const result = await loadConversation({ sourceId: 3, targetId: 2, floors, loadFloor: async () => assert.fail("loaded floors should not be fetched") });
  assert.deepEqual(result.entries.map((entry) => entry.floorId), [1, 2, 3, 4]);
  assert.deepEqual(result.entries.map((entry) => entry.depth), [0, 1, 2, 2]);
  assert.equal(result.limited, false);
});

test("ancestors outside the current page are loaded with deduplication", async () => {
  const requests = [];
  const result = await loadConversation({
    sourceId: 3, targetId: 2,
    floors: [floor(3, "##2 回复", { mentions: [floor(2, "##1 引用摘要")] })],
    loadFloor: async (floorId) => { requests.push(floorId); return floor(floorId, floorId === 2 ? "##1 最新正文" : "起点"); },
  });
  assert.deepEqual(requests, [2, 1]);
  assert.deepEqual(result.entries.map((entry) => entry.floorId), [1, 2, 3]);
  assert.equal(result.entries[1].floor.content, "##1 最新正文");
});

test("multiple references remain separate links and cycles do not duplicate floors", async () => {
  const floors = [floor(1, "##3 循环"), floor(2, "第二个引用"), floor(3, "##1 ##2 回复")];
  const result = await loadConversation({ sourceId: 3, targetId: 1, floors, loadFloor: async () => assert.fail() });
  assert.equal(result.entries.length, 3);
  assert.equal(new Set(result.entries.map((entry) => entry.floorId)).size, 3);
  assert.deepEqual(result.entries.find((entry) => entry.floorId === 3).replyToIds, [1, 2]);
});

test("deleted floors and failed fetches terminate their branch without losing readable replies", async () => {
  const result = await loadConversation({
    sourceId: 3, targetId: 2,
    floors: [floor(3, "##2 ##1 回复"), floor(1, "##999 已删除", { deleted: true })],
    loadFloor: async (floorId) => { assert.equal(floorId, 2); throw new Error("not available"); },
  });
  assert.equal(result.entries.length, 3);
  assert.equal(result.entries.find((entry) => entry.floorId === 2).unavailable, true);
  assert.deepEqual(result.entries.find((entry) => entry.floorId === 1).replyToIds, []);
  assert.equal(result.entries.find((entry) => entry.floorId === 3).unavailable, false);
});

test("a bounded batch can resume and rejects mismatched responses", async () => {
  const requests = [];
  const loadFloor = async (floorId) => { requests.push(floorId); return floor(floorId, floorId > 1 ? `##${floorId - 1} 回复` : "起点"); };
  const first = await loadConversation({ sourceId: 8, targetId: 7, floors: [floor(8, "##7 回复")], loadFloor, limit: 3 });
  assert.equal(first.limited, true);
  assert.equal(first.entries.length, 3);
  const second = await loadConversation({ sourceId: 8, targetId: 7, floors: first.floors, loadFloor, limit: 12 });
  assert.equal(second.entries.length, 8);
  assert.equal(second.limited, false);
  assert.equal(new Set(requests).size, requests.length);
  const mismatch = await loadConversation({ sourceId: 1, targetId: 1, floors: [], loadFloor: async () => floor(100, "错误楼层") });
  assert.equal(mismatch.entries[0].unavailable, true);
  assert.equal(mismatch.entries[0].floor, null);
});

test("closing the context stops additional network requests", async () => {
  let active = true;
  let requests = 0;
  await loadConversation({ sourceId: 3, targetId: 2, floors: [], isActive: () => active, loadFloor: async (floorId) => {
    requests += 1;
    active = false;
    return floor(floorId, "##1 引用");
  } });
  assert.equal(requests, 1);
});

test("context labels exclude nested quote placeholders and code references", () => {
  assert.equal(forumReplyExcerpt("##1\n##2\n实际回复 ![](dx_handsup)"), "实际回复 [表情]");
  assert.equal(forumReplyExcerpt("##1\n`[引用回复]` 是文字"), "[引用回复] 是文字");
  assert.deepEqual(forumReferenceIds("`##1` [##2](https://example.com) ##3 ##3"), [3]);
});
