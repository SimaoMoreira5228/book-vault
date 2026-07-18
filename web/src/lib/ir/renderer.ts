import type { Block, Span } from "$lib/api/generated";

export type Annotation = {
	id: string;
	book_id: string;
	section_id: string;
	block_index: number;
	start_offset: number;
	end_offset: number;
	color: string | null;
	note: string | null;
};

export type Segment = {
	text: string;
	annotationId?: string;
	color?: string;
};

export type ReaderTheme = "light" | "dark" | "sepia";

export const COLORS: Record<string, string> = {
	yellow: "#fef08a",
	green: "#bbf7d0",
	blue: "#bfdbfe",
	pink: "#fbcfe8",
	orange: "#fed7aa"
};

export const COLOR_NAMES = ["yellow", "green", "blue", "pink", "orange"];

export const THEME_CLASSES: Record<ReaderTheme, string> = {
	light: "bg-surface text-on-surface",
	dark: "bg-neutral-900 text-neutral-100",
	sepia: "bg-amber-50 text-amber-950"
};

export const THEME_CONTENT_STYLES: Record<ReaderTheme, string> = {
	light: "text-on-surface/90",
	dark: "text-neutral-200",
	sepia: "text-amber-900"
};

export function cycleTheme(current: ReaderTheme): ReaderTheme {
	const themes: ReaderTheme[] = ["light", "sepia", "dark"];
	const idx = themes.indexOf(current);
	return themes[(idx + 1) % themes.length];
}

export function getBlockSpans(
	block: unknown
): Array<{ text: string; marks: number; href: string | null }> {
	if (typeof block !== "object" || block === null) return [];
	const b = block as Record<string, unknown>;
	if ("Paragraph" in b)
		return b.Paragraph as Array<{ text: string; marks: number; href: string | null }>;
	if ("Heading" in b)
		return (b.Heading as { spans: Array<{ text: string; marks: number; href: string | null }> })
			.spans;
	return [];
}

export function getBlockText(block: unknown): string {
	if (typeof block !== "object" || block === null) return "";
	const b = block as Record<string, unknown>;
	if ("Paragraph" in b) return (b.Paragraph as Span[]).map((s) => s.text).join("");
	if ("Heading" in b) return (b.Heading as { spans: Span[] }).spans.map((s) => s.text).join("");
	return "";
}

export function splitTextAtAnnotations(text: string, blockAnnotations: Annotation[]): Segment[] {
	if (!blockAnnotations.length) return [{ text }];
	const segments: Segment[] = [];
	let pos = 0;
	const sorted = [...blockAnnotations].sort((a, b) => a.start_offset - b.start_offset);
	for (const ann of sorted) {
		if (ann.start_offset > pos) segments.push({ text: text.slice(pos, ann.start_offset) });
		if (ann.start_offset < text.length && ann.end_offset > 0) {
			segments.push({
				text: text.slice(Math.max(0, ann.start_offset), Math.min(text.length, ann.end_offset)),
				annotationId: ann.id,
				color: ann.color ?? "yellow"
			});
		}
		pos = Math.max(pos, ann.end_offset);
	}
	if (pos < text.length) segments.push({ text: text.slice(pos) });
	return segments;
}

export function getAnnotatedSegments(
	spans: Array<{ text: string; marks: number; href: string | null }>,
	blockAnnotations: Annotation[]
): Array<{
	text: string;
	marks: number;
	href: string | null;
	annotationId?: string;
	color?: string;
}> {
	if (!blockAnnotations.length)
		return spans.map((s) => ({ text: s.text, marks: s.marks, href: s.href }));

	const flatText = spans.map((s) => s.text).join("");
	const charMarks: number[] = [];
	const charHref: (string | null)[] = [];
	for (const span of spans) {
		for (let i = 0; i < span.text.length; i++) {
			charMarks.push(span.marks);
			charHref.push(span.href ?? null);
		}
	}

	const segments: Array<{
		text: string;
		marks: number;
		href: string | null;
		annotationId?: string;
		color?: string;
	}> = [];
	let pos = 0;
	const sorted = [...blockAnnotations].sort((a, b) => a.start_offset - b.start_offset);
	for (const ann of sorted) {
		if (ann.start_offset > pos) {
			segments.push({
				text: flatText.slice(pos, ann.start_offset),
				marks: charMarks[pos] ?? 0,
				href: charHref[pos] ?? null
			});
		}
		if (ann.start_offset < flatText.length && ann.end_offset > 0) {
			const end = Math.min(flatText.length, ann.end_offset);
			const segText = flatText.slice(Math.max(0, ann.start_offset), end);
			segments.push({
				text: segText,
				marks: charMarks[Math.max(0, ann.start_offset)] ?? 0,
				href: charHref[Math.max(0, ann.start_offset)] ?? null,
				annotationId: ann.id,
				color: ann.color ?? "yellow"
			});
		}
		pos = Math.max(pos, ann.end_offset);
	}
	if (pos < flatText.length) {
		segments.push({
			text: flatText.slice(pos),
			marks: charMarks[pos] ?? 0,
			href: charHref[pos] ?? null
		});
	}
	return segments;
}

export function getBlockAnnotations(
	annotations: Annotation[],
	sectionIdx: number,
	blockIdx: number,
	sectionId: string
): Annotation[] {
	return annotations.filter((a) => a.section_id === sectionId && a.block_index === blockIdx);
}
