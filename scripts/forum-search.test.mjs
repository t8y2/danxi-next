import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { parseForumSearchInput } from "../src/lib/features/forum/search.ts";

let server;
let ForumSearchController;
let backend;
before(async () => {
  const { createServer } = await import("vite");
  server = await createServer({ server: { middlewareMode: true, hmr: false, ws: false }, appType: "custom", logLevel: "error" });
  ({ ForumSearchController } = await server.ssrLoadModule("/src/lib/features/forum/search.svelte.ts"));
  ({ backend } = await server.ssrLoadModule("/src/lib/api/transport.ts"));
});
after(async () => { await server?.close(); });

const settle = () => new Promise((resolve) => setImmediate(resolve));
const floor = (floorId, overrides = {}) => ({ floorId, holeId: 42, content: "选课经验", anonyname: "同学", deleted: false, timeCreated: "2026-10-08T00:00:00Z", ...overrides });
const page = (ids, offset = 0, nextOffset = null) => ({ floors: ids.map((floorId) => floor(floorId)), offset, nextOffset });

test("input distinguishes explicit post IDs from keyword and numeric searches", () => {
  assert.deepEqual(parseForumSearchInput("  #12345  "), { kind: "hole", holeId: 12345 });
  assert.deepEqual(parseForumSearchInput("2026"), { kind: "search", query: "2026" });
  assert.deepEqual(parseForumSearchInput("  选课 & C++  "), { kind: "search", query: "选课 & C++" });
  assert.deepEqual(parseForumSearchInput("\n  "), { kind: "empty" });
  for (const input of ["#0", "#9007199254740992", "中".repeat(201)]) assert.equal(parseForumSearchInput(input).kind, "invalid");
  assert.equal(parseForumSearchInput("😀".repeat(200)).kind, "search");
});

test("only submission searches; direct navigation and invalid input do not send search requests", async () => {
  const calls = [];
  const controller = new ForumSearchController(async (...args) => { calls.push(args); return page([1]); });
  controller.input = "选课";
  assert.equal(calls.length, 0);
  assert.equal(controller.submit().kind, "search");
  await settle();
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0], ["选课", 0, { useWebvpn: true }]);
  controller.scrollTop = 160;
  controller.input = "#42";
  assert.deepEqual(controller.submit(), { kind: "hole", holeId: 42 });
  assert.equal(calls.length, 1);
  assert.equal(controller.query, "选课");
  assert.equal(controller.state.floors[0].floorId, 1);
  assert.equal(controller.scrollTop, 160);
  controller.input = "#0";
  controller.submit();
  assert.equal(calls.length, 1);
  assert.ok(controller.inputError);
});

test("new searches and clearing discard stale successes and failures", async () => {
  const requests = [];
  const controller = new ForumSearchController(() => new Promise((resolve, reject) => requests.push({ resolve, reject })));
  controller.input = "旧关键词";
  controller.submit();
  controller.input = "新关键词";
  controller.submit();
  requests[1].resolve(page([2]));
  await settle();
  requests[0].resolve(page([1]));
  await settle();
  assert.equal(controller.query, "新关键词");
  assert.deepEqual(controller.state.floors.map((item) => item.floorId), [2]);
  controller.input = "第三次";
  controller.submit();
  controller.clear();
  requests[2].reject({ kind: "network", message: "过期错误" });
  await settle();
  assert.equal(controller.query, "");
  assert.equal(controller.input, "");
  assert.equal(controller.state.phase, "idle");
});

test("pagination uses upstream offsets, deduplicates floors, and preserves results after failure", async () => {
  const offsets = [];
  let fail = true;
  const controller = new ForumSearchController(async (_query, offset) => {
    offsets.push(offset);
    if (!offset) return page([1, 2, 2], 0, 10);
    if (fail) throw { kind: "network", message: "稍后重试" };
    return page([2, 3], 10, null);
  });
  controller.input = "选课";
  controller.submit();
  await settle();
  assert.deepEqual(controller.state.floors.map((item) => item.floorId), [1, 2]);
  await controller.loadMore();
  assert.equal(controller.state.moreError, "稍后重试");
  assert.equal(controller.state.floors.length, 2);
  await controller.loadMore();
  assert.deepEqual(offsets, [0, 10]);
  fail = false;
  await controller.loadMore(true);
  assert.deepEqual(offsets, [0, 10, 10]);
  assert.deepEqual(controller.state.floors.map((item) => item.floorId), [1, 2, 3]);
  assert.equal(controller.state.moreError, null);
  await controller.loadMore();
  assert.equal(offsets.length, 3);
});

test("pagination is single-flight and an old page cannot append to a new query", async () => {
  let resolveMore;
  let requests = 0;
  const controller = new ForumSearchController(async (query, offset) => {
    requests += 1;
    if (query === "新") return page([9]);
    if (!offset) return page([1], 0, 10);
    return new Promise((resolve) => { resolveMore = resolve; });
  });
  controller.input = "旧";
  controller.submit();
  await settle();
  const pending = controller.loadMore();
  await controller.loadMore();
  assert.equal(requests, 2);
  controller.input = "新";
  controller.submit();
  await settle();
  resolveMore(page([2], 10));
  await pending;
  assert.deepEqual(controller.state.floors.map((item) => item.floorId), [9]);
});

test("empty results, retry, and expired sessions have distinct states", async () => {
  let response = "network";
  const controller = new ForumSearchController(async () => {
    if (response === "empty") return page([]);
    throw { kind: response, message: "失败" };
  });
  controller.input = "选课";
  controller.submit();
  await settle();
  assert.equal(controller.state.phase, "error");
  response = "empty";
  await controller.retry();
  assert.equal(controller.state.phase, "ready");
  assert.equal(controller.state.floors.length, 0);
  response = "auth";
  await controller.retry();
  assert.equal(controller.state.phase, "unauthenticated");
});

test("HTTP transport encodes query and keeps credentials in the gateway session", async (context) => {
  let request;
  context.mock.method(globalThis, "fetch", async (url, options) => {
    request = { url: new URL(url, "http://localhost"), options };
    return new Response(JSON.stringify(page([1], 20)), { headers: { "Content-Type": "application/json" } });
  });
  const result = await backend.searchForumFloors("选课 & C++", 20, { useWebvpn: false });
  assert.equal(request.url.pathname, "/api/v1/forum/search");
  assert.equal(request.url.searchParams.get("query"), "选课 & C++");
  assert.equal(request.url.searchParams.get("offset"), "20");
  assert.equal(request.url.searchParams.get("use_webvpn"), "false");
  assert.equal(request.options.credentials, "include");
  assert.equal(request.options.cache, "no-store");
  assert.equal(result.floors[0].floorId, 1);
});

test("search results render safe excerpts, hide deleted content, and link to the actual post", async () => {
  const { default: Results } = await server.ssrLoadModule("/src/lib/components/forum/ForumSearchResults.svelte");
  const { render } = await server.ssrLoadModule("svelte/server");
  const controller = new ForumSearchController(async () => page([]));
  controller.query = "选课";
  controller.state = { phase: "ready", floors: [floor(1, { content: "##99 **选课** ![](dx_handsup) <script>alert(1)</script>" }), floor(2, { deleted: true, content: "删除后不可展示的内容" })], nextOffset: null, loadingMore: false, moreError: null };
  const body = render(Results, { props: { controller, selectedHoleId: null, onOpenHole: () => {}, onLogin: () => {} } }).body;
  assert.match(body, /查看帖子 #42/);
  assert.match(body, /选课.*\[表情\]/);
  assert.match(body, /该楼层已删除/);
  assert.doesNotMatch(body, /<script|删除后不可展示的内容|##99/);
});
