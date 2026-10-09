use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanResult {
    pub decision: String,
    pub entities: Vec<String>,
    pub legal_score: f64,
}

static CREDIT_CARD_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(?:\b\d[ -]?){13,19}\b").unwrap());
static IBAN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b[A-Z]{2}\d{2}[A-Z0-9]{11,30}\b").unwrap());
static CRYPTO_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(?:bc1[a-zA-HJ-NP-Z0-9]{20,80}|[13][a-km-zA-HJ-NP-Z1-9]{25,34}|0x[a-fA-F0-9]{40})\b").unwrap());
static CLAUSE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b\d+\.\d+\b").unwrap());

const STRUCTURE: &[&str] = &["article i", "article ii", "section 1.1", "section 2.3", "clause", "hereinafter", "whereas", "hereto", "hereby", "thereof", "therein"];
const OBLIGATION: &[&str] = &["shall", "shall not", "agrees to", "is obligated to", "undertakes to", "covenant", "warrants", "represents"];
const BOILERPLATE: &[&str] = &["termination", "force majeure", "entire agreement", "amendment", "severability", "waiver", "assignment"];
const LEGAL: &[&str] = &["indemnify", "liability", "damages", "governing law", "jurisdiction", "applicable law", "compliance with laws"];
const STRONG: &[&str] = &["non-disclosure agreement", "nda", "confidential information", "proprietary information", "disclosing party", "receiving party", "mutual agreement", "unilateral agreement", "terms and conditions of this agreement"];

pub fn score_legal_text(text: &str) -> f64 {
    let lower = text.to_lowercase();
    let mut score = 0.0;
    for term in STRONG { if lower.contains(term) { score += 0.6; } }
    for term in STRUCTURE { if lower.contains(term) { score += 0.3; } }
    for term in OBLIGATION { if lower.contains(term) { score += 0.2; } }
    for term in LEGAL { if lower.contains(term) { score += 0.2; } }
    for term in BOILERPLATE { if lower.contains(term) { score += 0.25; } }
    if CLAUSE_RE.is_match(text) { score += 0.3; }
    if lower.matches("shall").count() > 3 { score += 0.3; }
    score
}

fn luhn_valid(s: &str) -> bool {
    let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if !(13..=19).contains(&digits.len()) { return false; }
    let mut sum = 0;
    let parity = digits.len() % 2;
    for (i, d) in digits.iter().enumerate() {
        let mut n = *d;
        if i % 2 == parity { n *= 2; if n > 9 { n -= 9; } }
        sum += n;
    }
    sum % 10 == 0
}

fn iban_valid(value: &str) -> bool {
    let compact: String = value.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() < 15 || compact.len() > 34 { return false; }
    let rotated = format!("{}{}", &compact[4..], &compact[..4]);
    let mut remainder: u32 = 0;
    for c in rotated.chars() {
        if c.is_ascii_digit() {
            remainder = (remainder * 10 + c.to_digit(10).unwrap()) % 97;
        } else if c.is_ascii_alphabetic() {
            let n = (c.to_ascii_uppercase() as u32) - ('A' as u32) + 10;
            remainder = (remainder * 100 + n) % 97;
        } else { return false; }
    }
    remainder == 1
}

pub fn scan_text(text: &str) -> ScanResult {
    let mut entities = Vec::new();
    if CREDIT_CARD_RE.find_iter(text).any(|m| luhn_valid(m.as_str())) { entities.push("CREDIT_CARD".to_string()); }
    if IBAN_RE.find_iter(text).any(|m| iban_valid(m.as_str())) { entities.push("IBAN_CODE".to_string()); }
    if CRYPTO_RE.is_match(text) { entities.push("CRYPTO".to_string()); }

    let legal_score = score_legal_text(text);
    let decision = if legal_score >= 1.2 || !entities.is_empty() { "block" } else { "allow" };
    ScanResult { decision: decision.into(), entities, legal_score }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_terms_score() {
        assert!(score_legal_text("This non-disclosure agreement contains confidential information and shall apply.") >= 1.2);
    }

    #[test]
    fn normal_text_allows() {
        assert_eq!(scan_text("Hello, how is the weather?").decision, "allow");
    }

    #[test]
    fn valid_card_is_detected() {
        assert!(scan_text("Test card 4111 1111 1111 1111").entities.contains(&"CREDIT_CARD".to_string()));
    }
}
