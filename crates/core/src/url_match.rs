//! Decides whether a saved login may be filled on a page. Anti-phishing rules: `docs/THREAT_MODEL.md`.

use url::{Host, Url};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Match {
    /// Same registrable domain (e.g. saved `example.com`, page `login.example.com`).
    SameSite,
    /// Same host.
    Exact,
}

/// `saved` may omit the scheme (`example.com` means `https://example.com`).
/// Returns `None` when filling is not allowed.
pub fn match_url(saved: &str, page: &str, exact_host: bool) -> Option<Match> {
    let page = Url::parse(page).ok()?;
    let saved = parse_saved(saved)?;
    if !matches!(page.scheme(), "http" | "https") || !matches!(saved.scheme(), "http" | "https") {
        return None;
    }
    // Never downgrade: a login saved for https is not filled over http.
    if saved.scheme() == "https" && page.scheme() == "http" {
        return None;
    }
    if saved.port().is_some() && saved.port_or_known_default() != page.port_or_known_default() {
        return None;
    }
    let (saved_host, page_host) = (saved.host()?, page.host()?);
    if saved_host == page_host {
        return Some(Match::Exact);
    }
    if exact_host {
        return None;
    }
    // IPs, localhost and other hosts without a registrable domain only ever match exactly.
    let (Host::Domain(s), Host::Domain(p)) = (saved_host, page_host) else {
        return None;
    };
    let site = |h: &str| psl::domain_str(h).map(str::to_owned);
    match (site(s), site(p)) {
        (Some(a), Some(b)) if a == b => Some(Match::SameSite),
        _ => None,
    }
}

fn parse_saved(saved: &str) -> Option<Url> {
    let saved = saved.trim();
    if saved.contains("://") { Url::parse(saved).ok() } else { Url::parse(&format!("https://{saved}")).ok() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Match::*;

    #[test]
    fn matching_rules() {
        let cases = [
            ("https://example.com", "https://example.com/login", false, Some(Exact)),
            ("example.com", "https://example.com", false, Some(Exact)),
            ("https://example.com", "https://login.example.com", false, Some(SameSite)),
            ("https://example.com", "https://login.example.com", true, None),
            ("https://example.com", "http://example.com", false, None), // downgrade
            ("http://example.com", "https://example.com", false, Some(Exact)), // upgrade ok
            ("https://example.com", "https://example.com.evil.io", false, None),
            ("https://example.com", "https://examp1e.com", false, None),
            ("https://example.com", "https://xn--exmple-cua.com", false, None), // homograph (punycode)
            ("https://a.github.io", "https://b.github.io", false, None),        // public suffix
            ("https://a.co.uk", "https://b.co.uk", false, None),
            ("https://login.bbc.co.uk", "https://www.bbc.co.uk", false, Some(SameSite)),
            ("https://example.com:8443", "https://example.com", false, None),
            ("https://example.com:8443", "https://example.com:8443/x", false, Some(Exact)),
            ("http://192.168.1.1", "http://192.168.1.1/admin", false, Some(Exact)),
            ("http://192.168.1.1", "http://192.168.1.2", false, None),
            ("http://localhost:3000", "http://localhost:3000", false, Some(Exact)),
            ("https://example.com", "file:///etc/passwd", false, None),
            ("https://example.com", "javascript:alert(1)", false, None),
            ("android://com.example", "https://example.com", false, None),
            ("", "https://example.com", false, None),
        ];
        for (saved, page, exact, want) in cases {
            assert_eq!(match_url(saved, page, exact), want, "{saved} on {page}");
        }
    }
}
