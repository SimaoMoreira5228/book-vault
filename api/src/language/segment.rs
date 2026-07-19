use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Token {
	pub surface_form: String,
	pub lemma: String,
	pub char_start: u32,
	pub char_end: u32,
	pub reading: Option<String>,
	pub pos: Option<String>,
	pub frequency_rank: Option<u32>,
}

pub fn tokenize(text: &str, language: &str) -> Vec<Token> {
	let mut tokens = Vec::new();
	let mut pos = 0u32;
	let chars: Vec<char> = text.chars().collect();

	while pos < chars.len() as u32 {
		if chars[pos as usize].is_whitespace() {
			pos += 1;
			continue;
		}

		let start = pos;
		let mut end = pos;

		while (end as usize) < chars.len() {
			let c = chars[end as usize];
			if c.is_alphabetic() || c == '\'' || c == '-' || c == '\u{2019}' {
				end += 1;
			} else {
				break;
			}
		}

		if end > start {
			let word: String = chars[start as usize..end as usize].iter().collect();
			let lemma = lemmatize(&word, language);
			tokens.push(Token {
				surface_form: word,
				lemma,
				char_start: start,
				char_end: end,
				reading: None,
				pos: None,
				frequency_rank: None,
			});
			pos = end;
		} else {
			pos += 1;
		}
	}

	tokens
}

pub fn lemmatize(word: &str, language: &str) -> String {
	match language {
		l if l.starts_with("pt") => lemmatize_pt(word),
		l if l.starts_with("es") => lemmatize_es(word),
		l if l.starts_with("fr") => lemmatize_fr(word),
		l if l.starts_with("de") => lemmatize_de(word),
		l if l.starts_with("it") => lemmatize_it(word),
		_ => lemmatize_en(word),
	}
}

fn lemmatize_en(word: &str) -> String {
	let lower = word.to_lowercase();
	if lower.len() < 4 {
		return lower;
	}

	if lower.ends_with("sses") || lower.ends_with("shes") || lower.ends_with("ches") || lower.ends_with("xes") {
		return lower[..lower.len() - 2].to_string();
	}
	if lower.ends_with("ies") && lower.len() > 4 {
		return format!("{}y", &lower[..lower.len() - 3]);
	}
	if lower.ends_with('s') && !lower.ends_with("ss") {
		return lower[..lower.len() - 1].to_string();
	}
	if lower.ends_with("ied") {
		return format!("{}y", &lower[..lower.len() - 3]);
	}
	if lower.ends_with("ed") {
		let stem = &lower[..lower.len() - 2];
		if stem.ends_with('e') {
			return stem.to_string();
		}
		if stem.len() > 2 && stem.chars().last() == stem.chars().rev().nth(1) {
			return stem[..stem.len() - 1].to_string();
		}
		return stem.to_string();
	}
	if lower.ends_with("ing") {
		let stem = &lower[..lower.len() - 3];
		if stem.ends_with('e') {
			return stem.to_string();
		}
		if stem.len() > 2 && stem.chars().last() == stem.chars().rev().nth(1) {
			return stem[..stem.len() - 1].to_string();
		}
		return stem.to_string();
	}
	lower
}

fn lemmatize_pt(word: &str) -> String {
	let lower = word.to_lowercase();
	if lower.len() < 4 {
		return lower;
	}
	if lower.ends_with("ões") {
		return format!("{}ão", &lower[..lower.len() - 3]);
	}
	if lower.ends_with("ães") {
		return format!("{}ão", &lower[..lower.len() - 3]);
	}
	if lower.ends_with("ais") {
		return format!("{}al", &lower[..lower.len() - 3]);
	}
	if lower.ends_with("res") && lower.len() > 5 {
		return format!("{}r", &lower[..lower.len() - 2]);
	}
	if lower.ends_with("ndo") {
		return format!("{}r", &lower[..lower.len() - 2]);
	}
	if lower.ends_with("dos") || lower.ends_with("das") {
		let stem = &lower[..lower.len() - 2];
		if stem.ends_with('a') || stem.ends_with('o') {
			return stem.to_string();
		}
	}
	if lower.ends_with('s') {
		let stem = &lower[..lower.len() - 1];
		if stem.len() > 2 {
			return stem.to_string();
		}
	}
	lower
}

fn lemmatize_es(word: &str) -> String {
	let lower = word.to_lowercase();
	if lower.len() < 4 {
		return lower;
	}
	if lower.ends_with("ción") || lower.ends_with("sión") {
		return lower.clone();
	}
	if lower.ends_with("ciones") || lower.ends_with("siones") {
		return format!("{}ción", &lower[..lower.len() - 5]);
	}
	if lower.ends_with("ando") || lower.ends_with("endo") {
		return format!("{}r", &lower[..lower.len() - 2]);
	}
	if lower.ends_with("ados") || lower.ends_with("idas") || lower.ends_with("idos") {
		return lower[..lower.len() - 1].to_string();
	}
	if lower.ends_with('s') && !lower.ends_with("as") {
		return lower[..lower.len() - 1].to_string();
	}
	lower
}

fn lemmatize_fr(word: &str) -> String {
	let lower = word.to_lowercase();
	if lower.len() < 4 {
		return lower;
	}
	if lower.ends_with("ment") && lower.len() > 5 {
		return lower[..lower.len() - 4].to_string();
	}
	if lower.ends_with("sses") {
		return lower[..lower.len() - 2].to_string();
	}
	if lower.ends_with("ents") {
		return format!("{}ent", &lower[..lower.len() - 1]);
	}
	if lower.ends_with('s') && !lower.ends_with("ss") {
		return lower[..lower.len() - 1].to_string();
	}
	lower
}

fn lemmatize_de(word: &str) -> String {
	let lower = word.to_lowercase();
	if lower.len() < 4 {
		return lower;
	}
	if lower.ends_with("ung") || lower.ends_with("heit") || lower.ends_with("keit") || lower.ends_with("schaft") {
		return lower.clone();
	}
	if lower.ends_with("en") || lower.ends_with("nen") {
		return lower[..lower.len() - 1].to_string();
	}
	if lower.ends_with("er") || lower.ends_with("es") {
		return lower[..lower.len() - 1].to_string();
	}
	lower
}

fn lemmatize_it(word: &str) -> String {
	let lower = word.to_lowercase();
	if lower.len() < 4 {
		return lower;
	}
	if lower.ends_with("zione") || lower.ends_with("mento") {
		return lower.clone();
	}
	if lower.ends_with("zioni") {
		return format!("{}zione", &lower[..lower.len() - 4]);
	}
	if lower.ends_with("menti") {
		return format!("{}mento", &lower[..lower.len() - 4]);
	}
	if lower.ends_with("ando") || lower.ends_with("endo") {
		return format!("{}re", &lower[..lower.len() - 2]);
	}
	if lower.ends_with('i') {
		return lower[..lower.len() - 1].to_string();
	}
	lower
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_tokenize_basic() {
		let tokens = tokenize("Hello world", "en");
		assert_eq!(tokens.len(), 2);
		assert_eq!(tokens[0].surface_form, "Hello");
		assert_eq!(tokens[1].surface_form, "world");
	}

	#[test]
	fn test_en_lemmatize() {
		assert_eq!(lemmatize("books", "en"), "book");
		assert_eq!(lemmatize("walked", "en"), "walk");
		assert_eq!(lemmatize("reading", "en"), "read");
		assert_eq!(lemmatize("parties", "en"), "party");
	}

	#[test]
	fn test_pt_lemmatize() {
		assert_eq!(lemmatize("livros", "pt"), "livro");
		assert_eq!(lemmatize("coração", "pt"), "coração");
		assert_eq!(lemmatize("cantando", "pt"), "cantar");
	}

	#[test]
	fn test_es_lemmatize() {
		assert_eq!(lemmatize("libros", "es"), "libro");
		assert_eq!(lemmatize("cantando", "es"), "cantar");
	}

	#[test]
	fn test_fr_lemmatize() {
		assert_eq!(lemmatize("livres", "fr"), "livre");
	}

	#[test]
	fn test_de_lemmatize() {
		assert_eq!(lemmatize("bücher", "de"), "bücher");
	}

	#[test]
	fn test_it_lemmatize() {
		assert_eq!(lemmatize("libri", "it"), "libro");
	}
}
