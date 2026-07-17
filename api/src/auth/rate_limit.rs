use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
	max_attempts: u32,
	window: Duration,
	ip_attempts: Mutex<HashMap<String, Vec<Instant>>>,
	email_attempts: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
	pub fn new(max_attempts: u32, window_secs: u64) -> Self {
		Self {
			max_attempts,
			window: Duration::from_secs(window_secs),
			ip_attempts: Mutex::new(HashMap::new()),
			email_attempts: Mutex::new(HashMap::new()),
		}
	}

	fn prune(attempts: &mut HashMap<String, Vec<Instant>>, window: Duration) {
		attempts.retain(|_, times| {
			times.retain(|t| t.elapsed() < window);
			!times.is_empty()
		});
	}

	pub fn check_ip(&self, ip: &str) -> Result<(), crate::AppError> {
		let mut map = self.ip_attempts.lock().unwrap();
		Self::prune(&mut map, self.window);
		if let Some(times) = map.get(ip) {
			if times.len() >= self.max_attempts as usize {
				return Err(crate::AppError::TooManyRequests(
					"Too many login attempts from this IP. Try again later.".into(),
				));
			}
		}
		Ok(())
	}

	pub fn check_email(&self, email: &str) -> Result<(), crate::AppError> {
		let mut map = self.email_attempts.lock().unwrap();
		Self::prune(&mut map, self.window);
		if let Some(times) = map.get(email) {
			if times.len() >= self.max_attempts as usize {
				return Err(crate::AppError::TooManyRequests(
					"Too many login attempts for this account. Try again later.".into(),
				));
			}
		}
		Ok(())
	}

	pub fn record_failure_ip(&self, ip: &str) {
		let mut map = self.ip_attempts.lock().unwrap();
		Self::prune(&mut map, self.window);
		map.entry(ip.to_string()).or_default().push(Instant::now());
	}

	pub fn record_failure_email(&self, email: &str) {
		let mut map = self.email_attempts.lock().unwrap();
		Self::prune(&mut map, self.window);
		map.entry(email.to_string()).or_default().push(Instant::now());
	}

	pub fn reset_ip(&self, ip: &str) {
		let mut map = self.ip_attempts.lock().unwrap();
		map.remove(ip);
	}

	pub fn reset_email(&self, email: &str) {
		let mut map = self.email_attempts.lock().unwrap();
		map.remove(email);
	}
}
