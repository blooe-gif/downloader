use std::collections::HashMap;

pub fn embedding(text: &str) -> Vec<f32> {
    // Lightweight local embedding approximation to avoid heavyweight model dependencies.
    let mut buckets = vec![0.0f32; 64];
    for tok in text.split(|c: char| !c.is_alphanumeric()) {
        if tok.is_empty() {
            continue;
        }
        let idx = fxhash(tok) % buckets.len() as u64;
        buckets[idx as usize] += 1.0;
    }
    let norm = buckets.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut buckets {
            *x /= norm;
        }
    }
    buckets
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

pub fn priority_score(url: &str, filename: &str, rules: &HashMap<String, f64>) -> f64 {
    let lower = format!("{url} {filename}").to_lowercase();
    let mut score = 0.1;
    for (needle, boost) in rules {
        if lower.contains(needle) {
            score += boost;
        }
    }
    score
}

fn fxhash(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
