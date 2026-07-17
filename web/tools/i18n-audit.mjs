import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const SRC_DIR = path.join(ROOT, "src");
const MESSAGES_DIR = path.join(ROOT, "messages");

function readJson(filePath) {
	return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function walk(dir) {
	const entries = fs.readdirSync(dir, { withFileTypes: true });
	const files = [];
	for (const entry of entries) {
		if (entry.name.startsWith(".")) continue;
		if (entry.name === "paraglide") continue;
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			files.push(...walk(full));
		} else {
			files.push(full);
		}
	}
	return files;
}

const filePaths = walk(SRC_DIR).filter(
	(p) => p.endsWith(".svelte") || p.endsWith(".ts") || p.endsWith(".js")
);

const keyRegex = /m\.([a-zA-Z_][a-zA-Z0-9_]*)\s*\(/g;

const usedKeys = new Set();

for (const filePath of filePaths) {
	const text = fs.readFileSync(filePath, "utf8");
	let match;
	while ((match = keyRegex.exec(text))) {
		const key = match[1];
		if (!key) continue;
		if (key === "then" || key === "catch" || key === "map") continue;
		usedKeys.add(key);
	}
}

const en = readJson(path.join(MESSAGES_DIR, "en.json"));
const pt = readJson(path.join(MESSAGES_DIR, "pt-PT.json"));

function isCodeKey(k) {
	return k === "$schema" || k.startsWith("$");
}

function getAllDefinedKeys(obj) {
	const keys = [];
	for (const key in obj) {
		if (isCodeKey(key)) continue;
		keys.push(key);
		if (typeof obj[key] === "object" && obj[key] !== null && !Array.isArray(obj[key])) {
			keys.push(...getAllDefinedKeys(obj[key]));
		}
	}
	return keys;
}

const enKeys = new Set(getAllDefinedKeys(en));
const ptKeys = new Set(getAllDefinedKeys(pt));

const missingInEn = [];
const missingInPt = [];
const definedButUnused = [];

for (const key of Array.from(usedKeys).sort()) {
	if (!enKeys.has(key)) missingInEn.push(key);
	if (!ptKeys.has(key)) missingInPt.push(key);
}

for (const key of enKeys) {
	if (!usedKeys.has(key) && key !== "$schema") {
		definedButUnused.push(key);
	}
}

function printList(title, list, limit = 200) {
	if (list.length === 0) return;
	console.log(`\n${title}: ${list.length}`);
	for (const k of list.slice(0, limit)) console.log(`  - ${k}`);
	if (list.length > limit) console.log(`  ... (${list.length - limit} more)`);
}

console.log(
	`Found ${usedKeys.size} m.*() keys used in src/** (${enKeys.size} defined in en.json, ${ptKeys.size} in pt-PT.json)`
);
printList("Missing in en.json", missingInEn);
printList("Missing in pt-PT.json", missingInPt);
printList("Defined in messages but never used in code", definedButUnused);

const hasErrors =
	missingInEn.length || missingInPt.length;
process.exitCode = hasErrors ? 2 : 0;
if (!hasErrors) {
	console.log("\nAll paraglide i18n keys look good!");
}
