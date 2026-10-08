import { decodeHTML } from "entities";
import { Marked, type MarkedToken, type Token } from "marked";
import type { ForumFloorMention } from "../../types/app";
import { forumStickerUrl } from "../../api/forum-media.ts";

export type ForumContentNode =
  | { type: "text" | "code" | "codespan"; text: string }
  | { type: "br" | "hr" }
  | { type: "paragraph" | "blockquote" | "strong" | "em" | "del"; children: ForumContentNode[] }
  | { type: "heading"; depth: number; children: ForumContentNode[] }
  | { type: "link"; url: string | null; children: ForumContentNode[] }
  | { type: "image"; url: string | null; alt: string }
  | { type: "sticker"; url: string; alt: string }
  | { type: "reference"; floorId: number }
  | { type: "list"; ordered: boolean; start: number; children: ForumContentNode[] }
  | { type: "listItem"; checked?: boolean; children: ForumContentNode[] }
  | { type: "table"; header: ForumContentNode[][]; rows: ForumContentNode[][][] };

export type ForumReferenceTarget = ForumFloorMention & {
  floorNumber?: number;
  mentions?: ForumFloorMention[];
};

const markdown = new Marked({
  gfm: true,
  breaks: true,
  extensions: [{
    name: "forumReference",
    level: "inline",
    start(source) {
      return source.search(/(?<!#)##[1-9]\d*/);
    },
    tokenizer(source, tokens) {
      if (tokens.at(-1)?.raw.endsWith("#")) return;
      const match = /^##([1-9]\d*)/.exec(source);
      if (!match) return;
      const floorId = Number(match[1]);
      if (!Number.isSafeInteger(floorId)) return;
      return { type: "forumReference", raw: match[0], floorId };
    },
  }],
});

export function safeForumUrl(value: string, image = false): string | null {
  try {
    const url = new URL(value);
    if (url.username || url.password) return null;
    if (url.protocol !== "https:" && (image || url.protocol !== "http:")) return null;
    return url.href;
  } catch {
    return null;
  }
}

function contentNodes(tokens: Token[], inLink = false): ForumContentNode[] {
  return tokens.flatMap((input): ForumContentNode[] => {
    if (input.type === "forumReference") {
      return inLink
        ? [{ type: "text", text: input.raw }]
        : [{ type: "reference", floorId: Number(input.floorId) }];
    }
    const token = input as MarkedToken;
    switch (token.type) {
      case "space":
      case "def":
        return [];
      case "text":
        return token.tokens
          ? contentNodes(token.tokens, inLink)
          : [{ type: "text", text: token.escaped ? token.text : decodeHTML(token.text) }];
      case "escape":
      case "html":
        return [{ type: "text", text: token.text }];
      case "code":
      case "codespan":
        return [{ type: token.type, text: token.text }];
      case "br":
      case "hr":
        return [{ type: token.type }];
      case "paragraph":
      case "blockquote":
      case "strong":
      case "em":
      case "del":
        return [{ type: token.type, children: contentNodes(token.tokens, inLink) }];
      case "heading":
        return [{ type: "heading", depth: token.depth, children: contentNodes(token.tokens, inLink) }];
      case "link":
        return [{
          type: "link",
          url: safeForumUrl(token.autolink ? token.href : decodeHTML(token.href)),
          children: contentNodes(token.tokens, true),
        }];
      case "image": {
        const source = decodeHTML(token.href);
        const stickerUrl = forumStickerUrl(source);
        return stickerUrl
          ? [{ type: "sticker", url: stickerUrl, alt: decodeHTML(token.text) || "旦夕表情" }]
          : [{ type: "image", url: safeForumUrl(source, true), alt: decodeHTML(token.text) }];
      }
      case "list":
        return [{
          type: "list",
          ordered: token.ordered,
          start: token.start || 1,
          children: contentNodes(token.items, inLink),
        }];
      case "list_item":
        return [{ type: "listItem", checked: token.task ? token.checked : undefined, children: contentNodes(token.tokens, inLink) }];
      case "checkbox":
        return [{ type: "text", text: token.checked ? "☑ " : "☐ " }];
      case "table":
        return [{
          type: "table",
          header: token.header.map((cell) => contentNodes(cell.tokens, inLink)),
          rows: token.rows.map((row) => row.map((cell) => contentNodes(cell.tokens, inLink))),
        }];
      default:
        return [{ type: "text", text: input.raw }];
    }
  });
}

function withoutReferences(nodes: ForumContentNode[]): ForumContentNode[] {
  const result = nodes.flatMap((node, index): ForumContentNode[] => {
    if (node.type === "reference" || (node.type === "br" && nodes[index - 1]?.type === "reference")) return [];
    if ("children" in node) {
      const children = withoutReferences(node.children);
      return children.length ? [{ ...node, children }] : [];
    }
    if (node.type === "table") {
      return [{ ...node, header: node.header.map(withoutReferences), rows: node.rows.map((row) => row.map(withoutReferences)) }];
    }
    return [node];
  });
  const isEmpty = (node: ForumContentNode | undefined) => node?.type === "br" || (node?.type === "text" && !node.text.trim());
  while (result.length && isEmpty(result[0])) result.shift();
  while (result.length && isEmpty(result.at(-1))) result.pop();
  return result;
}

export function parseForumContent(content: string, options: { hideReferences?: boolean } = {}): ForumContentNode[] {
  const nodes = contentNodes(markdown.lexer(content));
  return options.hideReferences ? withoutReferences(nodes) : nodes;
}

function plainContent(nodes: ForumContentNode[]): string {
  return nodes.map((node) => {
    if ("text" in node) return node.text;
    if (node.type === "image") return "[图片]";
    if (node.type === "sticker") return "[表情]";
    if (node.type === "reference") return "[引用回复]";
    if (node.type === "br" || node.type === "hr") return " ";
    if (node.type === "table") {
      return [node.header, ...node.rows].map((row) => row.map(plainContent).join(" ")).join(" ");
    }
    if ("children" in node) {
      const separator = ["paragraph", "heading", "listItem", "blockquote"].includes(node.type) ? " " : "";
      return plainContent(node.children) + separator;
    }
    return "";
  }).join("");
}

export function forumContentExcerpt(content: string): string {
  return plainContent(parseForumContent(content)).replace(/\s+/g, " ").trim();
}

export function forumReferenceIds(content: string): number[] {
  const ids = new Set<number>();
  function visit(nodes: ForumContentNode[]) {
    for (const node of nodes) {
      if (node.type === "reference") ids.add(node.floorId);
      if ("children" in node) visit(node.children);
      if (node.type === "table") {
        for (const row of [node.header, ...node.rows]) row.forEach(visit);
      }
    }
  }
  visit(parseForumContent(content));
  return [...ids];
}

export function forumReplyExcerpt(content: string): string {
  return plainContent(parseForumContent(content, { hideReferences: true })).replace(/\s+/g, " ").trim();
}

export function resolveForumReference(
  floorId: number,
  mentions: ForumFloorMention[],
  loadedFloors: ForumReferenceTarget[],
): ForumReferenceTarget | null {
  return loadedFloors.find((floor) => floor.floorId === floorId)
    ?? mentions.find((mention) => mention.floorId === floorId)
    ?? null;
}
