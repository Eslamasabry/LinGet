use crate::backend::SUGGEST_PREFIX;

pub fn parse_suggestion(message: &str) -> Option<(String, String)> {
    let idx = message.find(SUGGEST_PREFIX)?;
    let command = message[idx + SUGGEST_PREFIX.len()..].trim();
    if command.is_empty() {
        return None;
    }
    Some((message[..idx].trim().to_string(), command.to_string()))
}
