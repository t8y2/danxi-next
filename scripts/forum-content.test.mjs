import assert from "node:assert/strict";
import test from "node:test";
import { forumContentExcerpt, parseForumContent, resolveForumReference, safeForumUrl } from "../src/lib/features/forum/content.ts";

function flatten(nodes) {
  return nodes.flatMap((node) => [node, ...("children" in node ? flatten(node.children) : [])]);
}

test("floor references and image Markdown become distinct content nodes", () => {
  const nodes = flatten(parseForumContent("##123 回复正文\n\n![风景](https://image.fduhole.com/photo.png)"));
  assert.deepEqual(nodes.find((node) => node.type === "reference"), { type: "reference", floorId: 123 });
  assert.deepEqual(nodes.find((node) => node.type === "image"), {
    type: "image", url: "https://image.fduhole.com/photo.png", alt: "风景",
  });
  assert.ok(nodes.some((node) => node.type === "text" && node.text.includes("回复正文")));
});

test("code, escaped hashes, headings, and link URLs do not become floor references", () => {
  for (const content of ["`##123`", "```text\n##123\n```", "\\##123", "## 123", "###123", "[地址](https://example.com/##123)", "https://example.com/##123", "##9007199254740992"]) {
    assert.equal(flatten(parseForumContent(content)).filter((node) => node.type === "reference").length, 0, content);
  }
  assert.equal(flatten(parseForumContent("##123\n##456")).filter((node) => node.type === "reference").length, 2);
  assert.equal(flatten(parseForumContent("[##123](https://example.com)")).filter((node) => node.type === "reference").length, 0);
});

test("Markdown formatting, line breaks, lists, and tables remain structured", () => {
  const nodes = flatten(parseForumContent("## 标题\n\n**粗体** *斜体* ~~删除~~\n下一行\n\n- 列表\n\n> 引用\n\n| 列 |\n| --- |\n| 值 |"));
  for (const type of ["heading", "strong", "em", "del", "br", "list", "blockquote", "table"]) {
    assert.ok(nodes.some((node) => node.type === type), type);
  }
});

test("HTML stays literal text and active URL schemes are rejected", () => {
  const nodes = flatten(parseForumContent('<img src=x onerror="alert(1)">\n\n<script>alert(1)</script>'));
  assert.ok(nodes.every((node) => ["text", "paragraph"].includes(node.type)));
  assert.ok(nodes.some((node) => node.type === "text" && node.text.includes("<script>")));
  for (const url of ["javascript:alert(1)", "java\nscript:alert(1)", "data:text/html,test", "file:///etc/passwd", "//example.com/image.png", "https://user:password@example.com/"]) {
    assert.equal(safeForumUrl(url), null, url);
  }
  assert.equal(safeForumUrl("http://example.com/image.png", true), null);
  assert.equal(safeForumUrl("http://example.com/"), "http://example.com/");
  const link = flatten(parseForumContent('[点我](javascript&#58;alert%281%29)')).find((node) => node.type === "link");
  assert.equal(link.url, null);
  const image = flatten(parseForumContent('![](data:image/svg+xml,test)')).find((node) => node.type === "image");
  assert.equal(image.url, null);
});

test("reference-style images and HTML entities are resolved without changing code", () => {
  const nodes = flatten(parseForumContent("![图片][photo]\n\n[photo]: https://example.com/a.png?x=1&amp;y=2\n\nA &amp; B `&amp;`"));
  assert.equal(nodes.find((node) => node.type === "image").url, "https://example.com/a.png?x=1&y=2");
  assert.ok(nodes.some((node) => node.type === "text" && node.text.includes("A & B")));
  assert.equal(nodes.find((node) => node.type === "codespan").text, "&amp;");
});

test("DanXi sticker identifiers and their legacy aliases resolve only to the official image host", () => {
  for (const source of ["dx_handsup", "danxi_handsup"]) {
    const sticker = flatten(parseForumContent(`![](${source})`)).find((node) => node.type === "sticker");
    assert.equal(sticker.url, "https://static.fduhole.com/stickers/dx_handsup.webp");
    assert.equal(forumContentExcerpt(`![](${source})`), "[表情]");
  }
  for (const source of ["dx_../secret", "dx_test?redirect=https://evil.example", "dx_test/../secret", "javascript:alert%281%29"]) {
    assert.equal(flatten(parseForumContent(`![](${source})`)).filter((node) => node.type === "sticker").length, 0);
  }
});

test("list and reference excerpts remove Markdown delimiters and image URLs", () => {
  assert.equal(forumContentExcerpt("##123 **文字**\n\n![](https://image.fduhole.com/photo.png)"), "[引用回复] 文字 [图片]");
  assert.equal(forumContentExcerpt("第一段\n\n第二段"), "第一段 第二段");
});

test("references use loaded floors first, then out-of-page mention metadata", () => {
  const mention = { floorId: 123, holeId: 10, anonyname: "Alice", content: "原正文", deleted: false };
  const loaded = { ...mention, content: "新正文", floorNumber: 2, deleted: true };
  assert.deepEqual(resolveForumReference(123, [mention], []), mention);
  assert.deepEqual(resolveForumReference(123, [mention], [loaded]), loaded);
  assert.equal(resolveForumReference(456, [mention], [loaded]), null);
});

test("the Svelte component renders safe markup and hides deleted reference content", async () => {
  const { createServer } = await import("vite");
  const server = await createServer({
    server: { middlewareMode: true, hmr: false },
    appType: "custom",
    logLevel: "error",
  });
  try {
    const { default: ForumContent } = await server.ssrLoadModule("/src/lib/components/forum/ForumContent.svelte");
    const { render } = await server.ssrLoadModule("svelte/server");
    const renderBody = (props) => render(ForumContent, { props }).body.replace(/<!--[\s\S]*?-->/g, "");
    const target = { floorId: 123, holeId: 10, anonyname: "Alice", content: "原回复内容", deleted: false, floorNumber: 2 };
    const body = renderBody({
      content: '##123 回复 **粗体**\n\n![图片](https://image.fduhole.com/test.png)\n\n<script>alert(1)</script>\n\n[危险](javascript:alert%281%29)',
      loadedFloors: [target],
    });
    assert.match(body, /回复 2F · Alice/);
    assert.match(body, /原回复内容/);
    assert.match(body, /<strong[^>]*>粗体<\/strong>/);
    assert.match(body, /<img[^>]*src="https:\/\/image.fduhole.com\/test.png"/);
    assert.match(body, /referrerpolicy="no-referrer"/);
    assert.doesNotMatch(body, /<script|href="javascript:|##123/);
    const deleted = renderBody({ content: "##123", mentions: [{ ...target, deleted: true, content: "应隐藏的正文" }] });
    assert.match(deleted, /该楼层已删除/);
    assert.doesNotMatch(deleted, /应隐藏的正文/);
    assert.match(renderBody({ content: "##456" }), /引用的楼层暂不可见/);
    const linkedImage = renderBody({ content: "[![图片](https://image.fduhole.com/test.png)](https://example.com/)" });
    assert.equal((linkedImage.match(/<a /g) ?? []).length, 1);
    assert.match(renderBody({ content: "![](dx_handsup)" }), /<img[^>]*src="https:\/\/static.fduhole.com\/stickers\/dx_handsup.webp"/);
    assert.match(renderBody({ content: "##123", onOpenReference: () => {} }), /<button[^>]*aria-label="查看对话上下文"/);
    assert.doesNotMatch(renderBody({ content: "##123 回复正文", hideReferences: true }), /引用的楼层|查看对话/);
    const reply = renderBody({ content: "##123\n##456\n回复正文", hideReferences: true });
    assert.match(reply, /回复正文/);
    assert.doesNotMatch(reply, /<br|引用的楼层|查看对话/);
    assert.doesNotMatch(renderBody({ content: "##123\n\n回复正文", hideReferences: true }), /<p>\s*<\/p>/);
  } finally {
    await server.close();
  }
});
