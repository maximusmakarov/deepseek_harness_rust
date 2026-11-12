//! Best-effort redaction of secrets before they reach logs, errors, or event
//! payloads.
//!
//! This is a safety net, not a guarantee: it recognises the credential shapes
//! `dshr` itself handles (API keys, account tokens, bearer headers) and
//! sensitive YAML/JSON keys. Anything unrecognised must not be logged in the
//! first place.

/// Redacts known secret shapes in a multi-line string.
#[must_use]
pub fn redact(input: &str) -> String {
    input
        .split('\n')
        .map(redact_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_line(line: &str) -> String {
    if let Some(prefix) = sensitive_key_prefix(line) {
        return format!("{prefix} <redacted>");
    }
    let without_keys = redact_prefixed_tokens(line);
    redact_bearer(&without_keys)
}

/// Returns the `key:` prefix (including the colon) when the line assigns a
/// value to a key that by convention holds a credential.
fn sensitive_key_prefix(line: &str) -> Option<&str> {
    let colon = line.find(':')?;
    let key = line[..colon]
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_uppercase();
    let sensitive = matches!(
        key.as_str(),
        "AUTHORIZATION" | "X-DSH-AUTH-TOKEN" | "X-API-KEY" | "API-KEY"
    ) || key.ends_with("_KEY")
        || key.ends_with("_TOKEN")
        || key.ends_with("_SECRET")
        || key.ends_with("PASSWORD");
    sensitive.then_some(&line[..=colon])
}

fn redact_prefixed_tokens(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut index = 0;

    while index < chars.len() {
        let sk = chars[index..].starts_with(&['s', 'k', '-'])
            || chars[index..].starts_with(&['s', 'k', '_']);
        if sk {
            let mut end = index + 3;
            while end < chars.len()
                && (chars[end].is_ascii_alphanumeric() || chars[end] == '-' || chars[end] == '_')
            {
                end += 1;
            }
            if end - (index + 3) >= 8 {
                out.push_str("<redacted>");
                index = end;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }

    out
}

fn redact_bearer(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let Some(position) = lower.find("bearer ") else {
        return line.to_owned();
    };
    let token_start = position + "bearer ".len();
    let rest = &line[token_start..];
    let token_len = rest
        .chars()
        .take_while(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '+' | '/' | '=')
        })
        .count();
    if token_len < 8 {
        return line.to_owned();
    }
    let token_end = rest
        .char_indices()
        .nth(token_len)
        .map_or(rest.len(), |(offset, _)| offset);
    format!(
        "{}Bearer <redacted>{}",
        &line[..position],
        &rest[token_end..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_header_values() {
        assert_eq!(
            redact("x-dsh-auth-token: abcdef123456"),
            "x-dsh-auth-token: <redacted>"
        );
        assert_eq!(
            redact("Authorization: Bearer abcdef123456"),
            "Authorization: <redacted>"
        );
    }

    #[test]
    fn redacts_sensitive_yaml_keys() {
        assert_eq!(
            redact("ROUTERAI_API_KEY: sk-abcdefgh12345678"),
            "ROUTERAI_API_KEY: <redacted>"
        );
        assert_eq!(
            redact("db_password: hunter2hunter2"),
            "db_password: <redacted>"
        );
    }

    #[test]
    fn redacts_inline_keys() {
        assert_eq!(
            redact("using sk-abcdefgh12345678 for the call"),
            "using <redacted> for the call"
        );
        assert_eq!(redact("token=sk_abcdefgh12345678"), "token=<redacted>");
    }

    #[test]
    fn leaves_benign_text_alone() {
        let text = "task-run-123 completed in 42ms; see /tmp/out.log";
        assert_eq!(redact(text), text);
    }

    #[test]
    fn keeps_multiline_shape() {
        let input = "a\nx-api-key: secretvalue123\nb";
        assert_eq!(redact(input), "a\nx-api-key: <redacted>\nb");
    }
}
