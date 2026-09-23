use rand::RngExt;

// Short public ids like AC-7K2QX9PL (orders) or FB-M3TZ0A1B (feedback).
pub fn short_id(prefix: &str) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    let suffix: String = (0..8)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect();
    format!("{prefix}-{suffix}")
}
