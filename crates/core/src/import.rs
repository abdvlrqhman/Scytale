//! Importers. One CSV importer covers Chrome, Edge, Brave, Opera, Firefox, Safari, Bitwarden,
//! 1Password, LastPass and KeePassXC through a column-alias table; Bitwarden JSON adds cards and
//! identities.

use serde_json::Value;

use crate::{Card, Error, Identity, Item, ItemData, Login, Result};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Imported {
    pub items: Vec<Item>,
    /// Rows or entries that held nothing importable (or an unsupported type).
    pub skipped: usize,
}

const TITLE: &[&str] = &["name", "title"];
const URL: &[&str] = &["url", "uri", "login_uri", "website", "web site", "login url"];
const USERNAME: &[&str] = &["username", "user name", "login_username", "login", "user", "email"];
const PASSWORD: &[&str] = &["password", "login_password", "pass"];
const TOTP: &[&str] = &["totp", "otp", "otpauth", "login_totp"];
const NOTES: &[&str] = &["notes", "note", "extra", "comments"];
const FAVORITE: &[&str] = &["favorite", "fav"];
const TYPE: &[&str] = &["type"];

pub fn csv(data: &str) -> Result<Imported> {
    let mut reader =
        csv::ReaderBuilder::new().flexible(true).from_reader(data.trim_start_matches('\u{feff}').as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| Error::Invalid(format!("unreadable CSV: {e}")))?
        .iter()
        .map(|h| h.trim().to_lowercase())
        .collect();
    let col = |aliases: &[&str]| aliases.iter().find_map(|a| headers.iter().position(|h| h == a));
    let cols = [TITLE, URL, USERNAME, PASSWORD, TOTP, NOTES, FAVORITE, TYPE].map(col);
    let [title, url, username, password, totp, notes, favorite, kind] = cols;
    if password.is_none() && notes.is_none() {
        return Err(Error::Invalid("unrecognized CSV: no password or notes column".into()));
    }
    let url_is_bitwarden = url.is_some_and(|i| headers[i] == "login_uri");

    let mut out = Imported::default();
    for row in reader.records() {
        let row = row.map_err(|e| Error::Invalid(format!("unreadable CSV: {e}")))?;
        let get = |c: Option<usize>| c.and_then(|i| row.get(i)).unwrap_or("").trim().to_owned();
        let raw_url = get(url);
        let urls: Vec<String> = if url_is_bitwarden {
            raw_url.split(',').map(str::trim).map(str::to_owned).collect()
        } else {
            vec![raw_url]
        }
        .into_iter()
        .filter(|u| !u.is_empty())
        .collect();
        let (user, pw, note) = (get(username), get(password), get(notes));
        // LastPass marks secure notes with the pseudo-URL http://sn; Bitwarden with type=note.
        let is_note = get(kind).eq_ignore_ascii_case("note") || urls.first().is_some_and(|u| u == "http://sn");
        if urls.is_empty() && user.is_empty() && pw.is_empty() && note.is_empty() {
            out.skipped += 1;
            continue;
        }
        let data = if is_note {
            ItemData::Note
        } else {
            ItemData::Login(Login {
                username: user.clone(),
                password: pw,
                urls: urls.clone(),
                totp: get(totp),
                exact_host: false,
            })
        };
        let title = Some(get(title))
            .filter(|t| !t.is_empty())
            .or_else(|| urls.first().and_then(|u| host_of(u)))
            .or_else(|| Some(user).filter(|u| !u.is_empty()))
            .unwrap_or_else(|| "Imported item".into());
        let fav = get(favorite);
        out.items.push(Item { title, notes: note, favorite: fav == "1" || fav.eq_ignore_ascii_case("true"), data });
    }
    Ok(out)
}

fn host_of(url: &str) -> Option<String> {
    let parsed = if url.contains("://") { url::Url::parse(url) } else { url::Url::parse(&format!("https://{url}")) };
    parsed.ok()?.host_str().map(|h| h.trim_start_matches("www.").to_owned())
}

/// Unencrypted Bitwarden JSON export.
pub fn bitwarden_json(data: &str) -> Result<Imported> {
    let root: Value = serde_json::from_str(data).map_err(|e| Error::Invalid(format!("not valid JSON: {e}")))?;
    if root["encrypted"].as_bool() == Some(true) {
        return Err(Error::Invalid("this export is encrypted; export as unencrypted JSON from Bitwarden".into()));
    }
    let items = root["items"].as_array().ok_or_else(|| Error::Invalid("not a Bitwarden export: no items".into()))?;
    let s = |v: &Value| v.as_str().unwrap_or("").to_owned();

    let mut out = Imported::default();
    for it in items {
        let data = match it["type"].as_u64() {
            Some(1) => {
                let l = &it["login"];
                let urls =
                    l["uris"].as_array().map(|a| a.iter().map(|u| s(&u["uri"])).filter(|u| !u.is_empty()).collect());
                ItemData::Login(Login {
                    username: s(&l["username"]),
                    password: s(&l["password"]),
                    urls: urls.unwrap_or_default(),
                    totp: s(&l["totp"]),
                    exact_host: false,
                })
            }
            Some(2) => ItemData::Note,
            Some(3) => {
                let c = &it["card"];
                let (m, y) = (s(&c["expMonth"]), s(&c["expYear"]));
                let expiry = if m.is_empty() {
                    String::new()
                } else {
                    format!("{m:0>2}/{}", y.get(y.len().saturating_sub(2)..).unwrap_or(""))
                };
                ItemData::Card(Card {
                    holder: s(&c["cardholderName"]),
                    number: s(&c["number"]),
                    expiry,
                    cvv: s(&c["code"]),
                    pin: String::new(),
                })
            }
            Some(4) => {
                let i = &it["identity"];
                let name = ["firstName", "middleName", "lastName"]
                    .map(|k| s(&i[k]))
                    .into_iter()
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                ItemData::Identity(Identity {
                    full_name: name,
                    email: s(&i["email"]),
                    phone: s(&i["phone"]),
                    company: s(&i["company"]),
                    address1: s(&i["address1"]),
                    address2: [s(&i["address2"]), s(&i["address3"])]
                        .into_iter()
                        .filter(|p| !p.is_empty())
                        .collect::<Vec<_>>()
                        .join(", "),
                    city: s(&i["city"]),
                    region: s(&i["state"]),
                    postal_code: s(&i["postalCode"]),
                    country: s(&i["country"]),
                })
            }
            _ => {
                out.skipped += 1;
                continue;
            }
        };
        // Custom fields have no dedicated slot yet; keep them in the notes so nothing is lost.
        let mut notes = s(&it["notes"]);
        for f in it["fields"].as_array().into_iter().flatten() {
            notes.push_str(&format!(
                "{}{}: {}",
                if notes.is_empty() { "" } else { "\n" },
                s(&f["name"]),
                s(&f["value"])
            ));
        }
        out.items.push(Item {
            title: s(&it["name"]),
            notes,
            favorite: it["favorite"].as_bool().unwrap_or(false),
            data,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login(item: &Item) -> &Login {
        match &item.data {
            ItemData::Login(l) => l,
            other => panic!("expected login, got {other:?}"),
        }
    }

    #[test]
    fn chrome_csv() {
        let r =
            csv("\u{feff}name,url,username,password,note\nGitHub,https://github.com/login,me,pw1,hi\n,,,,\n").unwrap();
        assert_eq!(r.skipped, 1);
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.items[0].title, "GitHub");
        assert_eq!(r.items[0].notes, "hi");
        assert_eq!(login(&r.items[0]).urls, ["https://github.com/login"]);
    }

    #[test]
    fn firefox_csv_titles_from_host() {
        let data = "\"url\",\"username\",\"password\",\"httpRealm\",\"formActionOrigin\",\"guid\",\"timeCreated\",\"timeLastUsed\",\"timePasswordChanged\"\n\"https://www.example.com\",\"a\",\"b\",,\"https://www.example.com\",\"{x}\",\"1\",\"1\",\"1\"\n";
        let r = csv(data).unwrap();
        assert_eq!(r.items[0].title, "example.com");
        assert_eq!(login(&r.items[0]).password, "b");
    }

    #[test]
    fn bitwarden_and_lastpass_csv() {
        let bw = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n,1,login,Mail,,,0,\"https://a.com,https://b.com\",u,p,JBSWY3DPEHPK3PXP\n,,note,Secret,text,,0,,,,\n";
        let r = csv(bw).unwrap();
        assert!(r.items[0].favorite);
        assert_eq!(login(&r.items[0]).urls, ["https://a.com", "https://b.com"]);
        assert_eq!(login(&r.items[0]).totp, "JBSWY3DPEHPK3PXP");
        assert_eq!(r.items[1].data, ItemData::Note);

        let lp = "url,username,password,totp,extra,name,grouping,fav\nhttp://sn,,,,my note,Note,,0\n";
        assert_eq!(csv(lp).unwrap().items[0].data, ItemData::Note);
    }

    #[test]
    fn rejects_unrelated_csv() {
        assert!(csv("a,b,c\n1,2,3\n").is_err());
    }

    #[test]
    fn bitwarden_json_all_types() {
        let data = r#"{"encrypted":false,"items":[
            {"type":1,"name":"Mail","notes":null,"favorite":true,"fields":[{"name":"pin","value":"42"}],
             "login":{"username":"u","password":"p","totp":null,"uris":[{"uri":"https://mail.com"}]}},
            {"type":2,"name":"Note","notes":"txt"},
            {"type":3,"name":"Visa","card":{"cardholderName":"A B","number":"4111","expMonth":"3","expYear":"2031","code":"123"}},
            {"type":4,"name":"Me","identity":{"firstName":"Ada","lastName":"Lovelace","email":"a@b.c","address3":"x"}},
            {"type":5,"name":"SSH key"}
        ]}"#;
        let r = bitwarden_json(data).unwrap();
        assert_eq!((r.items.len(), r.skipped), (4, 1));
        assert_eq!(r.items[0].notes, "pin: 42");
        assert_eq!(login(&r.items[0]).urls, ["https://mail.com"]);
        let ItemData::Card(c) = &r.items[2].data else { panic!() };
        assert_eq!(c.expiry, "03/31");
        let ItemData::Identity(i) = &r.items[3].data else { panic!() };
        assert_eq!((i.full_name.as_str(), i.address2.as_str()), ("Ada Lovelace", "x"));
        assert!(bitwarden_json(r#"{"encrypted":true,"items":[]}"#).is_err());
    }
}
