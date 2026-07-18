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

pub fn tokenize(text: &str) -> Vec<Token> {
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
			let lemma = simple_lemmatize(&word);
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

pub fn simple_lemmatize(word: &str) -> String {
	let lower = word.to_lowercase();

	if lower.len() > 5 {
		if lower.ends_with("ies") && lower.len() > 4 {
			return format!("{}y", &lower[..lower.len() - 3]);
		}
		if lower.ends_with("ves") && lower.len() > 4 {
			return format!("{}f", &lower[..lower.len() - 3]);
		}
		if lower.ends_with("ning") && lower.len() > 5 {
			let stem = &lower[..lower.len() - 3];
			if stem.ends_with('n') {
				return stem[..stem.len() - 1].to_string();
			}
			return stem.to_string();
		}
		if lower.ends_with("ming") && lower.len() > 5 {
			let stem = &lower[..lower.len() - 3];
			if stem.ends_with('m') {
				return stem[..stem.len() - 1].to_string();
			}
			return stem.to_string();
		}
		if lower.ends_with("sses") || lower.ends_with("shes") || lower.ends_with("ches") || lower.ends_with("xes") {
			return lower[..lower.len() - 2].to_string();
		}
		if lower.ends_with("ized") || lower.ends_with("ised") {
			return format!("{}e", &lower[..lower.len() - 1]);
		}
	}

	if lower.len() > 4 {
		if lower.ends_with("ss") {
			return lower.clone();
		}
		if lower.ends_with('s') && !lower.ends_with("ss") {
			return lower[..lower.len() - 1].to_string();
		}
	}

	if lower.len() > 4 {
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
		if lower.ends_with("er") || lower.ends_with("est") {
			let stem = if lower.ends_with("est") {
				&lower[..lower.len() - 3]
			} else {
				&lower[..lower.len() - 2]
			};
			if !stem.is_empty() {
				return stem.to_string();
			}
		}
		if lower.ends_with("ly") {
			return lower[..lower.len() - 2].to_string();
		}
	}

	lower
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_tokenize_basic() {
		let tokens = tokenize("Hello world");
		assert_eq!(tokens.len(), 2);
		assert_eq!(tokens[0].surface_form, "Hello");
		assert_eq!(tokens[1].surface_form, "world");
	}

	#[test]
	fn test_tokenize_punctuation() {
		let tokens = tokenize("Hello, world!");
		assert_eq!(tokens.len(), 2);
		assert_eq!(tokens[0].surface_form, "Hello");
	}

	#[test]
	fn test_lemmatize_plural() {
		assert_eq!(simple_lemmatize("books"), "book");
		assert_eq!(simple_lemmatize("boxes"), "box");
		assert_eq!(simple_lemmatize("parties"), "party");
	}

	#[test]
	fn test_lemmatize_past_tense() {
		assert_eq!(simple_lemmatize("walked"), "walk");
		assert_eq!(simple_lemmatize("studied"), "study");
		assert_eq!(simple_lemmatize("stopped"), "stop");
	}

	#[test]
	fn test_lemmatize_progressive() {
		assert_eq!(simple_lemmatize("reading"), "read");
		assert_eq!(simple_lemmatize("running"), "run");
	}
}
