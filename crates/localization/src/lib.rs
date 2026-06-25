use parking_lot::RwLock;
use serde_json::Value;
use std::{collections::BTreeMap, env, sync::OnceLock};

const DEFAULT_LOCALE: &str = "en-US";

struct LocaleResource {
    tag: &'static str,
    json: &'static str,
}

const RESOURCES: &[LocaleResource] = &[
    LocaleResource {
        tag: "en-US",
        json: include_str!("../resources/en-US.json"),
    },
    LocaleResource {
        tag: "zh-Hans",
        json: include_str!("../resources/zh-Hans.json"),
    },
    LocaleResource {
        tag: "zh-Hant-HK",
        json: include_str!("../resources/zh-Hant-HK.json"),
    },
    LocaleResource {
        tag: "zh-Hant-TW",
        json: include_str!("../resources/zh-Hant-TW.json"),
    },
    LocaleResource {
        tag: "ja-JP",
        json: include_str!("../resources/ja-JP.json"),
    },
];

static ACTIVE_LOCALE: OnceLock<RwLock<String>> = OnceLock::new();
static CATALOGS: OnceLock<BTreeMap<&'static str, BTreeMap<String, String>>> = OnceLock::new();

pub fn set_locale(locale: impl AsRef<str>) {
    *active_locale().write() = canonical_locale(locale.as_ref());
}

pub fn locale() -> String {
    active_locale().read().clone()
}

pub fn available_locales() -> &'static [&'static str] {
    &["en-US", "zh-Hans", "zh-Hant-HK", "zh-Hant-TW", "ja-JP"]
}

pub fn text(key: &str) -> String {
    text_for_locale(&locale(), key)
}

pub fn text_args(key: &str, args: &[(&str, &str)]) -> String {
    text_args_for_locale(&locale(), key, args)
}

fn text_for_locale(locale: &str, key: &str) -> String {
    for candidate in locale_candidates(locale) {
        if let Some(value) = catalogs().get(candidate.as_str()).and_then(|catalog| catalog.get(key)) {
            return value.clone();
        }
    }

    catalogs()
        .get(DEFAULT_LOCALE)
        .and_then(|catalog| catalog.get(key))
        .unwrap_or_else(|| panic!("missing localization key `{key}`"))
        .clone()
}

fn text_args_for_locale(locale: &str, key: &str, args: &[(&str, &str)]) -> String {
    let mut value = text_for_locale(locale, key);

    for (name, replacement) in args {
        value = value.replace(&format!("{{{name}}}"), replacement);
    }

    value
}

fn active_locale() -> &'static RwLock<String> {
    ACTIVE_LOCALE.get_or_init(|| RwLock::new(initial_locale()))
}

fn initial_locale() -> String {
    if let Ok(locale) = env::var("ZED_LOCALE") {
        return canonical_locale(&locale);
    }

    sys_locale::get_locale()
        .map(|locale| canonical_locale(&locale))
        .unwrap_or_else(|| DEFAULT_LOCALE.to_string())
}

fn catalogs() -> &'static BTreeMap<&'static str, BTreeMap<String, String>> {
    CATALOGS.get_or_init(|| {
        RESOURCES
            .iter()
            .map(|resource| {
                let parsed = serde_json::from_str::<BTreeMap<String, Value>>(resource.json)
                    .unwrap_or_else(|err| panic!("invalid localization resource {}: {err}", resource.tag));
                let catalog = parsed
                    .into_iter()
                    .map(|(key, value)| {
                        let value = value
                            .as_str()
                            .unwrap_or_else(|| panic!("localization value `{key}` is not a string"));
                        (key, value.to_string())
                    })
                    .collect();

                (resource.tag, catalog)
            })
            .collect()
    })
}

fn canonical_locale(locale: &str) -> String {
    let normalized = locale.replace('_', "-");
    let mut parts = normalized.split('-').map(str::to_string);
    let language = parts
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or_else(|| DEFAULT_LOCALE.to_string())
        .to_ascii_lowercase();
    let mut canonical = vec![language];

    for part in parts {
        if part.len() == 4 {
            let mut chars = part.chars();
            let first = chars.next().unwrap().to_uppercase().to_string();
            canonical.push(format!("{}{}", first, chars.as_str().to_ascii_lowercase()));
        } else {
            canonical.push(part.to_ascii_uppercase());
        }
    }

    canonical.join("-")
}

fn locale_candidates(locale: &str) -> Vec<String> {
    let canonical = canonical_locale(locale);
    let parts = canonical.split('-').collect::<Vec<_>>();
    let mut candidates = vec![canonical.clone()];

    if parts.len() >= 2 && parts[1].len() == 4 {
        candidates.push(format!("{}-{}", parts[0], parts[1]));
    }

    if let Some(language_locale) = first_locale_for_language(parts[0]) {
        candidates.push(language_locale.to_string());
    }

    candidates.push(DEFAULT_LOCALE.to_string());
    candidates.dedup();
    candidates
}

fn first_locale_for_language(language: &str) -> Option<&'static str> {
    RESOURCES
        .iter()
        .map(|resource| resource.tag)
        .find(|tag| tag.split('-').next() == Some(language))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn canonicalizes_locale_tags() {
        assert_eq!(canonical_locale("zh_hans_cn"), "zh-Hans-CN");
        assert_eq!(canonical_locale("EN-us"), "en-US");
    }

    #[test]
    fn falls_back_to_language_match_then_english() {
        assert_eq!(text_for_locale("zh-Hans-CN", "common.close"), "关闭");

        assert_eq!(text_for_locale("fr-FR", "common.close"), "Close");
    }

    #[test]
    fn interpolates_placeholders() {
        assert_eq!(
            text_args_for_locale("en-US", "agent.thread.copied_context", &[("name", "main.rs")]),
            "Copied main.rs"
        );
    }

    #[test]
    fn resources_have_the_same_keys_and_placeholders() {
        let en = catalogs().get("en-US").unwrap();
        let en_keys = en.keys().collect::<BTreeSet<_>>();

        for locale in available_locales().iter().copied().filter(|locale| *locale != "en-US") {
            let catalog = catalogs().get(locale).unwrap();
            assert_eq!(en_keys, catalog.keys().collect::<BTreeSet<_>>(), "{locale}");

            for key in &en_keys {
                assert_eq!(
                    placeholders(en.get(*key).unwrap()),
                    placeholders(catalog.get(*key).unwrap()),
                    "{locale}:{key}"
                );
            }
        }
    }

    fn placeholders(value: &str) -> BTreeSet<String> {
        let mut placeholders = BTreeSet::new();
        let mut rest = value;

        while let Some(start) = rest.find('{') {
            rest = &rest[start + 1..];
            let Some(end) = rest.find('}') else {
                break;
            };
            placeholders.insert(rest[..end].to_string());
            rest = &rest[end + 1..];
        }

        placeholders
    }
}
