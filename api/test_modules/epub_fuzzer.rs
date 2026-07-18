use crate::common::TestApp;
use std::io::Write;

fn make_epub(body: &str) -> Vec<u8> {
	let mut buf = std::io::Cursor::new(Vec::new());
	use zip::write::SimpleFileOptions;
	let mut zip = zip::ZipWriter::new(&mut buf);
	let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
	let normal = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
	zip.start_file("mimetype", stored).unwrap();
	zip.write_all(b"application/epub+zip").unwrap();
	zip.start_file("META-INF/container.xml", normal).unwrap();
	zip.write_all(b"<?xml version=\"1.0\"?><container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\"><rootfiles><rootfile full-path=\"content.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>").unwrap();
	zip.start_file("content.opf", normal).unwrap();
	zip.write_all(br#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="2.0"><metadata><dc:title xmlns:dc="http://purl.org/dc/elements/1.1/">Fuzz</dc:title></metadata><manifest><item id="s1" href="s1.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="s1"/></spine></package>"#).unwrap();
	zip.start_file("s1.xhtml", normal).unwrap();
	zip.write_all(
		format!(
			r#"<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml"><body>{}</body></html>"#,
			body
		)
		.as_bytes(),
	)
	.unwrap();
	zip.finish().unwrap();
	buf.into_inner()
}

fn make_raw(paths: &[(&str, &[u8])]) -> Vec<u8> {
	let mut buf = std::io::Cursor::new(Vec::new());
	use zip::write::SimpleFileOptions;
	let mut zip = zip::ZipWriter::new(&mut buf);
	let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
	let normal = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
	for (path, data) in paths {
		let opts = if *path == "mimetype" { &stored } else { &normal };
		zip.start_file(path, *opts).unwrap();
		zip.write_all(data).unwrap();
	}
	zip.finish().unwrap();
	buf.into_inner()
}

async fn upload_read(app: &TestApp, data: &[u8]) -> serde_json::Value {
	let r = app.upload_file("fuzz.epub", data).await;
	let book_id = r["book_id"].as_str().unwrap();
	let job_id = r["job_id"].as_str().unwrap();
	let job = app.wait_for_job(&job_id, std::time::Duration::from_secs(120)).await;
	assert_eq!(job["status"], "completed", "job failed: {:?}", job["error"]);
	let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(s, 200);
	d
}

fn spans(block: &serde_json::Value) -> Vec<(String, u16, Option<String>)> {
	block
		.get("Paragraph")
		.or_else(|| block.get("Heading").and_then(|h| h.get("spans")))
		.and_then(|s| s.as_array())
		.map(|a| {
			a.iter()
				.map(|s| {
					(
						s["text"].as_str().unwrap_or("").to_string(),
						s["marks"].as_u64().unwrap_or(0) as u16,
						s["href"].as_str().map(|h| h.to_string()),
					)
				})
				.collect()
		})
		.unwrap_or_default()
}

fn blocks(data: &serde_json::Value) -> Vec<&serde_json::Value> {
	data["book"]["spine"][0]["blocks"]
		.as_array()
		.map(|b| b.iter().collect())
		.unwrap_or_default()
}

fn all_text(data: &serde_json::Value) -> String {
	data["book"]["spine"]
		.as_array()
		.into_iter()
		.flatten()
		.flat_map(|s| s["blocks"].as_array().into_iter().flatten())
		.flat_map(|b| {
			b.get("Paragraph")
				.or_else(|| b.get("Heading").and_then(|h| h.get("spans")))
				.and_then(|s| s.as_array())
				.into_iter()
				.flatten()
		})
		.filter_map(|s| s["text"].as_str())
		.collect::<Vec<_>>()
		.join("")
}

#[tokio::test]
async fn fuzz_plain_paragraph() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_plain").await;
	let data = upload_read(&app, &make_epub("<p>Hello world</p>")).await;
	let s = spans(&blocks(&data)[0]);
	assert_eq!(s.len(), 1);
	assert_eq!(s[0].0, "Hello world");
}

#[tokio::test]
async fn fuzz_bold_italic() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_bi").await;
	let data = upload_read(&app, &make_epub("<p>normal <b>bold</b> <i>italic</i> normal</p>")).await;
	let s = spans(&blocks(&data)[0]);
	assert_eq!(s.len(), 5);
	assert_eq!(s[1].1, 1);
	assert_eq!(s[3].1, 2);
}

#[tokio::test]
async fn fuzz_spaces_between_spans() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_spc").await;
	let data = upload_read(&app, &make_epub("<p>words <i>again</i> and <i>Anastasia</i> together</p>")).await;
	let s = spans(&blocks(&data)[0]);
	let full: String = s.iter().map(|(t, _, _)| t.as_str()).collect();
	assert_eq!(full, "words again and Anastasia together", "got: '{full}'");
}

#[tokio::test]
async fn fuzz_unicode_cjk() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_cjk").await;
	let data = upload_read(&app, &make_epub("<p>日本語 中文 한국어 Русский العربية עברית हिन्दी</p>")).await;
	let t = all_text(&data);
	assert!(t.contains("日本語"));
	assert!(t.contains("中文"));
	assert!(t.contains("Русский"));
}

#[tokio::test]
async fn fuzz_unicode_emoji() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_emoji").await;
	let data = upload_read(&app, &make_epub("<p>😀😂🥲🤯👩‍💻🧑🏽‍🚀🔥💀🎉</p>")).await;
	let t = all_text(&data);
	assert!(t.contains("😀"));
}

#[tokio::test]
async fn fuzz_unicode_mathematical() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_math").await;
	let html = "<p>𝐁𝐨𝐥𝐝 𝘐𝘵𝘢𝘭𝘪𝘤 𝔉𝔯𝔞𝔨𝔱𝔲𝔯 𝓯𝓪𝓷𝓬𝔂 𝒮𝒸𝓇𝒾𝓅𝓉</p>";
	let data = upload_read(&app, &make_epub(html)).await;
	let t = all_text(&data);
	assert!(t.contains("𝐁𝐨𝐥𝐝"));
}

#[tokio::test]
async fn fuzz_unicode_zalgo() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_zalgo").await;
	let zalgo = "ä̴̢̨̛͎̺͖͚̫̮̩̺͕̬̘̪̟̮́̔͋͐̈́̒͌̓̎̈́̽̑͑̔͆̚͝͝";
	let data = upload_read(&app, &make_epub(&format!("<p>{}</p>", zalgo))).await;
	let t = all_text(&data);
	assert!(!t.is_empty(), "zalgo text should survive ingestion");
}

#[tokio::test]
async fn fuzz_unicode_rtl() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_rtl").await;
	let data = upload_read(&app, &make_epub("<p dir=\"rtl\">אבג דהו זחט יכל</p>")).await;
	let t = all_text(&data);
	assert!(t.contains("אבג"), "RTL Hebrew should survive");
}

#[tokio::test]
async fn fuzz_unicode_combining() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_combo").await;
	let data = upload_read(&app, &make_epub("<p>e\u{0301} cafe\u{0301} na\u{00EF}ve</p>")).await;
	let t = all_text(&data);
	assert!(t.contains("e\u{0301}"));
}

#[tokio::test]
async fn fuzz_unicode_mixed() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_mix").await;
	let html = "<p>English 日本語 العربية 😀 𝐁𝐨𝐥𝐝 𝘪𝘵𝘢𝘭𝘪𝘤 ä̴́̚ together</p>";
	let data = upload_read(&app, &make_epub(html)).await;
	let t = all_text(&data);
	assert!(t.contains("English"));
	assert!(t.contains("日本語"));
	assert!(t.contains("together"));
}

#[tokio::test]
async fn fuzz_long_paragraph() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_long").await;
	let text = "word ".repeat(5000);
	let html = format!("<p>{}</p>", text);
	let data = upload_read(&app, &make_epub(&html)).await;
	let t = all_text(&data);
	assert!(t.len() > 20000, "long text preserved: {} chars", t.len());
}

#[tokio::test]
async fn fuzz_many_paragraphs() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_many").await;
	let html = (0..100)
		.map(|i| format!("<p>Paragraph {}</p>", i))
		.collect::<Vec<_>>()
		.join("\n");
	let data = upload_read(&app, &make_epub(&html)).await;
	let b = blocks(&data);
	assert_eq!(b.len(), 100, "should have 100 paragraphs");
}

#[tokio::test]
async fn fuzz_deeply_nested_spans() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_deep").await;
	let inner = (0..20).fold("deep".to_string(), |acc, _| format!("<b>{}</b>", acc));
	let html = format!("<p>{}</p>", inner);
	let data = upload_read(&app, &make_epub(&html)).await;
	let s = spans(&blocks(&data)[0]);
	assert!(s.len() >= 1, "deeply nested should produce at least one span");
}

#[tokio::test]
async fn fuzz_many_spans() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_mspans").await;
	let inner = (0..200)
		.map(|i| format!("<b>B{}</b> <i>I{}</i> ", i, i))
		.collect::<Vec<_>>()
		.join("");
	let html = format!("<p>{}</p>", inner);
	let data = upload_read(&app, &make_epub(&html)).await;
	let s = spans(&blocks(&data)[0]);
	assert!(s.len() > 200, "should have 200+ spans, got {}", s.len());
}

#[tokio::test]
async fn fuzz_large_text_node() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_large").await;
	let text = "A".repeat(100_000);
	let html = format!("<p>{}</p>", text);
	let data = upload_read(&app, &make_epub(&html)).await;
	let t = all_text(&data);
	assert_eq!(t.len(), 100_000, "100KB text should be preserved");
}

#[tokio::test]
async fn fuzz_inline_svg() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_svg").await;
	let html = r#"<p>Text</p><svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><circle cx="50" cy="50" r="40" fill="red"/></svg><p>After</p>"#;
	let data = upload_read(&app, &make_epub(html)).await;
	let b = blocks(&data);
	assert!(b.len() >= 2, "should parse around SVG, got {} blocks", b.len());
}

#[tokio::test]
async fn fuzz_images_with_various_mime() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_img").await;
	let html = r#"<p>Image: <img src="data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxyZWN0IHdpZHRoPSIxMDAiIGhlaWdodD0iMTAwIiBmaWxsPSJyZWQiLz48L3N2Zz4=" alt="test"/></p>"#;
	let data = upload_read(&app, &make_epub(html)).await;
	let t = all_text(&data);
	assert!(t.contains("test") || t.contains("Image"));
}

#[tokio::test]
async fn fuzz_missing_mimetype() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_nomime").await;
	let data = make_raw(&[
		("content.opf", b"<package/>"),
		("s1.xhtml", b"<html><body><p>hi</p></body></html>"),
	]);
	let r = app.upload_file("broken.epub", &data).await;
	let _book_id = r["book_id"].as_str().unwrap();
	let job_id = r["job_id"].as_str().unwrap();
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;
	let job = app.get_job(&job_id).await;
	println!("missing_mimetype status: {:?}", job["status"]);
}

#[tokio::test]
async fn fuzz_missing_container_xml() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_nocont").await;
	let data = make_raw(&[
		("mimetype", b"application/epub+zip"),
		("content.opf", b"<package/>"),
		("s1.xhtml", b"<html><body><p>hi</p></body></html>"),
	]);
	let r = app.upload_file("nocont.epub", &data).await;
	let _book_id = r["book_id"].as_str().unwrap();
	let job_id = r["job_id"].as_str().unwrap();
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;
	let job = app.get_job(&job_id).await;
	println!("missing_container status: {:?}", job["status"]);
}

#[tokio::test]
async fn fuzz_invalid_xhtml() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_badxml").await;
	let data = make_epub("<p>valid</p><p>also valid<p>unclosed");
	let r = app.upload_file("badxml.epub", &data).await;
	println!("invalid_xhtml upload: {:?}", r);
}

struct Rng(u64);
impl Rng {
	fn new(seed: u64) -> Self {
		Self(seed)
	}
	fn next(&mut self) -> u64 {
		self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
		self.0
	}
	fn range(&mut self, lo: usize, hi: usize) -> usize {
		lo + (self.next() as usize % (hi - lo))
	}
	fn pick<'a, T: Copy>(&mut self, items: &'a [T]) -> T {
		items[self.range(0, items.len())]
	}
	fn coin(&mut self) -> bool {
		self.next() & 1 == 0
	}
}

fn random_para(rng: &mut Rng) -> String {
	let mut html = String::from("<p>");
	let count = rng.range(1, 30);
	for _ in 0..count {
		if rng.coin() {
			let words = ["hello", "world", "foo", "bar", "test", "lorem", "ipsum", "dolor"];
			html.push_str(rng.pick(&words[..]));
		} else {
			let scripts = [
				"\u{65E5}\u{672C}\u{8A9E}",
				"\u{4E2D}\u{6587}",
				"\u{CE74}\u{D0C0}\u{B9D0}",
				"\u{0440}\u{0443}\u{0441}",
				"\u{1D400}\u{1D41D}\u{1D41E}\u{1D41B}",
				"\u{1F600}",
				"\u{1F680}",
			];
			html.push_str(rng.pick(&scripts[..]));
		}
		html.push(' ');
		if rng.coin() {
			let tags = ["b", "i", "u", "s", "code", "em", "strong"];
			let inner = ["X", "Y", "Z", "marked"];
			html.push_str(&format!(
				"<{}>{}</{}> ",
				rng.pick(&tags[..]),
				rng.pick(&inner[..]),
				rng.pick(&tags[..])
			));
		}
	}
	html.push_str("</p>");
	html
}

#[tokio::test]
async fn fuzz_property_random() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_prop").await;
	for seed in 0..15 {
		let mut rng = Rng::new(seed);
		let mut html = String::new();
		let p_count = rng.range(1, 10);
		for _ in 0..p_count {
			html.push_str(&random_para(&mut rng));
		}
		let data = make_epub(&html);
		let r = app.upload_file("prop.epub", &data).await;
		let bid = r["book_id"].as_str().unwrap();
		let jid = r["job_id"].as_str().unwrap();
		let job = app.wait_for_job(&jid, std::time::Duration::from_secs(60)).await;
		assert_eq!(job["status"], "completed", "seed {}: {:?}", seed, job["error"]);
		let (s, _rd) = app.raw_get(&format!("/api/v1/books/{bid}/read")).await;
		assert_eq!(s, 200, "seed {} read failed", seed);
		println!("prop seed {}: {} paras OK", seed, p_count);
	}
}

#[tokio::test]
async fn fuzz_property_mixed() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fz_mixp").await;
	for seed in 100..112 {
		let mut rng = Rng::new(seed);
		let mut html = String::new();
		for _ in 0..rng.range(1, 8) {
			let tag = rng.pick(&["b", "i", "u", "s", "code", "em", "strong"]);
			html.push_str(&format!("<{}>{}</{}>", tag, random_para(&mut rng), tag));
		}
		let data = make_epub(&format!("<div>{}</div>", html));
		let r = app.upload_file("mix.epub", &data).await;
		let bid = r["book_id"].as_str().unwrap();
		let jid = r["job_id"].as_str().unwrap();
		let job = app.wait_for_job(&jid, std::time::Duration::from_secs(60)).await;
		assert_eq!(job["status"], "completed", "seed {}: {:?}", seed, job["error"]);
		println!("mix seed {} OK", seed);
	}
}
