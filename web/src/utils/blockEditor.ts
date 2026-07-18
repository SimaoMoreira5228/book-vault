import type { Block, Span } from "$lib/api/generated";

export function createEmptyBlock(type: string = "Paragraph"): Block {
	switch (type) {
		case "Heading":
			return { Heading: { level: 1, spans: [] } };
		case "BlockQuote":
			return { BlockQuote: [{ Paragraph: [{ text: "", marks: 0, href: null }] }] };
		case "CodeBlock":
			return { CodeBlock: { language: null, content: "" } };
		case "UnorderedList":
			return { UnorderedList: [[{ Paragraph: [{ text: "", marks: 0, href: null }] }]] };
		case "OrderedList":
			return { OrderedList: [[{ Paragraph: [{ text: "", marks: 0, href: null }] }]] };
		case "HorizontalRule":
			return "HorizontalRule";
		default:
			return { Paragraph: [{ text: "", marks: 0, href: null }] };
	}
}

export function getBlockType(block: Block): string {
	if (typeof block === "string") return block;
	const key = Object.keys(block)[0];
	return key;
}

export function blockToText(block: Block): string {
	if (typeof block === "string") return "";
	const b = block as Record<string, unknown>;
	const entry = Object.entries(b)[0];
	if (!entry) return "";
	const [type, value] = entry;

	switch (type) {
		case "Paragraph": {
			const spans = value as Span[];
			return spans.map((s) => s.text).join("");
		}
		case "Heading": {
			const h = value as { spans: Span[] };
			return h.spans.map((s) => s.text).join("");
		}
		case "CodeBlock": {
			const c = value as { content: string };
			return c.content;
		}
		case "BlockQuote": {
			const subs = value as Block[];
			return subs.map(blockToText).join(" ");
		}
		default:
			return "";
	}
}

const MARK_BOLD = 1;
const MARK_ITALIC = 2;
const MARK_UNDERLINE = 4;
const MARK_STRIKETHROUGH = 8;

function getMarkBits(node: Node): number {
	let bits = 0;
	let el = node instanceof Element ? node : node.parentElement;
	while (el && el !== document.body?.parentElement) {
		const tag = el.tagName?.toLowerCase();
		if (tag === "b" || tag === "strong") bits |= MARK_BOLD;
		if (tag === "i" || tag === "em") bits |= MARK_ITALIC;
		if (tag === "u" || tag === "ins") bits |= MARK_UNDERLINE;
		if (tag === "s" || tag === "strike" || tag === "del") bits |= MARK_STRIKETHROUGH;
		el = el.parentElement;
	}
	return bits;
}

export function htmlToSpans(el: HTMLElement): Span[] {
	const spans: Span[] = [];
	const walker = document.createTreeWalker(el, NodeFilter.SHOW_ALL, null);

	let currentText = "";
	let currentMarks = 0;
	let currentHref: string | null = null;

	function flush() {
		if (currentText) {
			spans.push({ text: currentText, marks: currentMarks, href: currentHref });
			currentText = "";
		}
	}

	while (walker.nextNode()) {
		const node = walker.currentNode;

		if (node.nodeType === Node.TEXT_NODE) {
			const text = node.textContent ?? "";
			if (text) {
				if (currentText === "") {
					currentMarks = getMarkBits(node);
					const a = node.parentElement?.closest("a");
					currentHref = a instanceof HTMLAnchorElement ? a.getAttribute("href") : null;
				}
				currentText += text;
			}
		} else if (node instanceof HTMLBRElement) {
			currentText += "\n";
		} else if (node instanceof HTMLAnchorElement) {
			flush();
		}
	}
	flush();
	return spans;
}

export function spansToHtml(spans: Span[]): string {
	return spans
		.map((s) => {
			const text = escapeHtml(s.text);
			let wrapped = text;
			if (s.marks & MARK_BOLD) wrapped = `<b>${wrapped}</b>`;
			if (s.marks & MARK_ITALIC) wrapped = `<i>${wrapped}</i>`;
			if (s.marks & MARK_UNDERLINE) wrapped = `<u>${wrapped}</u>`;
			if (s.marks & MARK_STRIKETHROUGH) wrapped = `<s>${wrapped}</s>`;
			if (s.href) wrapped = `<a href="${escapeAttr(s.href)}">${wrapped}</a>`;
			return wrapped;
		})
		.join("");
}

function escapeHtml(s: string): string {
	return s
		.replace(/&/g, "&amp;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;")
		.replace(/\n/g, "<br>");
}

function escapeAttr(s: string): string {
	return s.replace(/"/g, "&quot;").replace(/&/g, "&amp;");
}

export function renderBlockContent(block: Block): string {
	if (typeof block === "string") return "";
	const b = block as Record<string, unknown>;
	const entry = Object.entries(b)[0];
	if (!entry) return "";
	const [type, value] = entry;

	switch (type) {
		case "Paragraph": {
			return spansToHtml(value as Span[]);
		}
		case "Heading": {
			const h = value as { spans: Span[] };
			return spansToHtml(h.spans);
		}
		case "CodeBlock":
		case "BlockQuote":
		case "Image":
		default:
			return blockToText(block);
	}
}

export function extractSpansFromBlock(block: Block): Span[] {
	if (typeof block === "string") return [];
	const b = block as Record<string, unknown>;
	const entry = Object.entries(b)[0];
	if (!entry) return [];
	const [, value] = entry;

	if (
		Array.isArray(value) &&
		value.length > 0 &&
		typeof value[0] === "object" &&
		"text" in value[0]
	) {
		return value as Span[];
	}
	if (typeof value === "object" && value !== null && "spans" in value) {
		return (value as { spans: Span[] }).spans;
	}
	return [];
}
