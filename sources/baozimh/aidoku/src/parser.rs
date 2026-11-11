use alloc::{format, string::String, vec::Vec};
use serde_json::Value;

pub const BASE_URL: &str = "https://www.twmanga.com";
pub const HOSTS: &[&str] = &[
    "www.twmanga.com",
    "cn.twmanga.com",
    "tw.twmanga.com",
    "www.baozimh.com",
    "cn.baozimh.com",
    "tw.baozimh.com",
    "www.webmota.com",
    "cn.webmota.com",
    "tw.webmota.com",
    "www.dinnerku.com",
    "cn.dinnerku.com",
    "tw.dinnerku.com",
    "www.kukuc.co",
    "cn.kukuc.co",
    "tw.kukuc.co",
];

pub fn urlencode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            output.push(byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

pub fn valid_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
}

pub fn base_url(value: &str) -> Option<String> {
    let host = value.trim_end_matches('/').strip_prefix("https://")?;
    HOSTS.contains(&host).then(|| format!("https://{host}"))
}

fn site_path(url: &str) -> Option<&str> {
    if url.starts_with('/') && !url.starts_with("//") {
        return Some(url);
    }
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/')?;
    HOSTS
        .contains(&host)
        .then_some(&url[url.len() - path.len() - 1..])
}

pub fn manga_key(url: &str) -> Option<String> {
    let path = site_path(url)?.split(['?', '#']).next()?;
    let key = path.strip_prefix("/comic/")?.trim_end_matches('/');
    valid_key(key).then(|| key.into())
}

fn query_value<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query
        .split('&')
        .filter_map(|part| part.split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn slot(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

/// Retain BOTH slots: section 1, chapter 0 is different from section 0, chapter 0.
pub fn chapter_key(url: &str, manga: &str) -> Option<String> {
    let path = site_path(url)?;
    let query = path.strip_prefix("/user/page_direct?")?.split('#').next()?;
    if query_value(query, "comic_id")? != manga {
        return None;
    }
    let section = query_value(query, "section_slot")?;
    let chapter = query_value(query, "chapter_slot")?;
    (slot(section) && slot(chapter)).then(|| format!("{section}_{chapter}"))
}

pub fn chapter_path(manga: &str, chapter: &str) -> Option<String> {
    let (section, chapter) = chapter.split_once('_')?;
    if !valid_key(manga) || !slot(section) || !slot(chapter) {
        return None;
    }
    Some(format!(
        "/user/page_direct?comic_id={manga}&section_slot={section}&chapter_slot={chapter}"
    ))
}

pub fn absolute_image(value: &str, base: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        None
    } else if value.starts_with("https://") || value.starts_with("http://") {
        Some(value.into())
    } else if value.starts_with("//") {
        Some(format!("https:{value}"))
    } else if value.starts_with('/') && !value.contains(':') {
        Some(format!("{}{value}", base.trim_end_matches('/')))
    } else {
        None
    }
}

/// Rewrite only the image host, never occurrences in the path or query.
pub fn image_request_url(url: &str) -> Option<String> {
    let url = absolute_image(url, BASE_URL)?;
    let (scheme, rest) = url.split_once("://")?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    for suffix in [".baozicdn.com", ".bzcdn.net"] {
        if let Some(node) = host.strip_suffix(suffix) {
            return Some(format!("{scheme}://{node}.baozimh.com/{path}"));
        }
    }
    Some(url)
}

pub fn list_path(region: &str, state: &str, genre: &str, page: i32) -> String {
    format!(
        "/api/bzmhq/amp_comic_list?type={}&region={}&state={}&filter=*&page={}",
        urlencode(genre),
        urlencode(region),
        urlencode(state),
        page.max(1)
    )
}

#[derive(Debug)]
pub struct ListEntry {
    pub key: String,
    pub title: String,
    pub cover: Option<String>,
}

pub fn api_list(json: &str) -> Result<(Vec<ListEntry>, bool), &'static str> {
    let value: Value = serde_json::from_str(json).map_err(|_| "包子漫画分类接口返回无效 JSON")?;
    let items = value["items"].as_array().ok_or("包子漫画分类数据缺失")?;
    let mut entries: Vec<ListEntry> = Vec::new();
    for item in items {
        let Some(key) = item["comic_id"].as_str().filter(|key| valid_key(key)) else {
            continue;
        };
        let Some(title) = item["name"]
            .as_str()
            .filter(|title| !title.trim().is_empty())
        else {
            continue;
        };
        if entries.iter().any(|entry| entry.key == key) {
            continue;
        }
        let cover = item["topic_img"]
            .as_str()
            .filter(|path| !path.is_empty())
            .and_then(|path| {
                if path.starts_with('/')
                    || path.starts_with("https://")
                    || path.starts_with("http://")
                {
                    absolute_image(path, "https://static-tw.baozimh.com")
                } else if !path.contains(':') {
                    Some(format!("https://static-tw.baozimh.com/cover/{path}"))
                } else {
                    None
                }
            });
        entries.push(ListEntry {
            key: key.into(),
            title: title.into(),
            cover,
        });
    }
    // Do not follow the server's next URL; generate the next page locally.
    let has_next = !entries.is_empty()
        && value["next"]
            .as_str()
            .is_some_and(|next| !next.trim().is_empty());
    Ok((entries, has_next))
}

#[derive(Debug, PartialEq)]
pub struct ChapterEntry {
    pub key: String,
    pub title: String,
}

/// Full directory is oldest first; latest-update preview is newest first.
/// Never sort by slot or chapter number: preserve special and split chapters.
pub fn ordered_chapters(items: Vec<ChapterEntry>, full_directory: bool) -> Vec<ChapterEntry> {
    let mut result: Vec<ChapterEntry> = Vec::new();
    for item in items {
        if !result.iter().any(|entry| entry.key == item.key) {
            result.push(item);
        }
    }
    if full_directory {
        result.reverse();
    }
    result
}

pub fn chapter_number(title: &str) -> Option<f32> {
    let title = title
        .trim()
        .strip_prefix('第')
        .unwrap_or(title.trim())
        .trim();
    let length = title
        .bytes()
        .take_while(|byte| byte.is_ascii_digit() || *byte == b'.')
        .count();
    title[..length].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn validates_mirrors_and_manga_links() {
        for host in HOSTS {
            assert_eq!(
                manga_key(&format!(
                    "https://{host}/comic/id_with-suffix?from=search#top"
                ))
                .as_deref(),
                Some("id_with-suffix")
            );
        }
        assert_eq!(manga_key("/comic/id/").as_deref(), Some("id"));
        for url in [
            "//evil.test/comic/id",
            "https://www.baozimh.com.evil.test/comic/id",
            "https://evil.test/comic/id",
            "/comic/../secret",
            "/comic/chapter/id_0.html",
        ] {
            assert!(manga_key(url).is_none(), "{url}");
        }
        assert!(base_url("https://www.twmanga.com/path").is_none());
        assert_eq!(urlencode("斗罗 &?"), "%E6%96%97%E7%BD%97%20%26%3F");
    }

    #[test]
    fn chapter_slots_survive_reordered_query_parameters() {
        assert_eq!(
            chapter_key(
                "/user/page_direct?chapter_slot=12&comic_id=manga_id&section_slot=1&foo=bar",
                "manga_id"
            )
            .as_deref(),
            Some("1_12")
        );
        assert_ne!(
            chapter_key(
                "/user/page_direct?comic_id=id&section_slot=0&chapter_slot=0",
                "id"
            ),
            chapter_key(
                "/user/page_direct?comic_id=id&section_slot=1&chapter_slot=0",
                "id"
            )
        );
        assert!(
            chapter_key(
                "/user/page_direct?comic_id=other&section_slot=0&chapter_slot=1",
                "id"
            )
            .is_none()
        );
        assert!(chapter_path("id", "0_1&evil=1").is_none());
        assert!(chapter_path("id", "-1_0").is_none());
        let path = chapter_path("manga_id", "1_12").unwrap();
        assert_eq!(chapter_key(&path, "manga_id").as_deref(), Some("1_12"));
    }

    #[test]
    fn preserves_split_chapters_and_deduplicates_preview() {
        let entries = || {
            vec![
                ChapterEntry {
                    key: "0_0".into(),
                    title: "第1话 上".into(),
                },
                ChapterEntry {
                    key: "0_1".into(),
                    title: "第1话 下".into(),
                },
                ChapterEntry {
                    key: "1_0".into(),
                    title: "番外".into(),
                },
                ChapterEntry {
                    key: "0_1".into(),
                    title: "重复预览".into(),
                },
            ]
        };
        let result = ordered_chapters(entries(), true);
        assert_eq!(
            result
                .iter()
                .map(|item| item.key.as_str())
                .collect::<Vec<_>>(),
            ["1_0", "0_1", "0_0"]
        );
        assert_eq!(ordered_chapters(entries(), false)[0].key, "0_0");
        assert_eq!(chapter_number("第 12.5 話"), Some(12.5));
        assert_eq!(chapter_number("番外"), None);
    }

    #[test]
    fn parses_real_category_response_and_pagination() {
        let (entries, next) =
            api_list(include_str!("../../../../tests/fixtures/baozimh/list.json")).unwrap();
        assert_eq!(entries.len(), 20);
        assert_eq!(entries[0].key, "yirenzhixia-dongmantang");
        assert!(
            entries[0]
                .cover
                .as_ref()
                .unwrap()
                .starts_with("https://static-tw.baozimh.com/cover/")
        );
        assert!(next);
        for next in ["null", "\"\""] {
            assert!(
                !api_list(&format!("{{\"items\":[],\"next\":{next}}}"))
                    .unwrap()
                    .1
            );
        }
        assert!(api_list("{\"error\":\"challenge_required\"}").is_err());
        assert!(api_list("<html>challenge</html>").is_err());
        assert!(list_path("all&bad=1", "all", "all", 0).contains("region=all%26bad%3D1"));
    }

    #[test]
    fn real_directory_retains_all_hidden_chapters_and_correct_direction() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/baozimh/chapters.json"
        ))
        .unwrap();
        let manga = fixture["manga"].as_str().unwrap();
        let parse = |field: &str| -> Vec<ChapterEntry> {
            fixture[field]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| ChapterEntry {
                    key: chapter_key(item["href"].as_str().unwrap(), manga).unwrap(),
                    title: item["title"].as_str().unwrap().into(),
                })
                .collect()
        };
        let directory = ordered_chapters(parse("directory"), true);
        let preview = ordered_chapters(parse("preview"), false);
        assert_eq!(directory.len(), 823);
        assert_eq!(preview.len(), 24);
        assert_eq!(directory[0].key, "0_822");
        assert_eq!(directory.last().unwrap().key, "0_0");
        assert_eq!(&directory[..24], preview.as_slice());
        let mut items = parse("directory");
        items.extend(parse("preview"));
        assert_eq!(ordered_chapters(items, true), directory);
    }

    #[test]
    fn rewrites_cdn_host_without_corrupting_signed_paths() {
        assert_eq!(
            image_request_url("https://s1.bzcdn.net/a.webp?token=.bzcdn.net").as_deref(),
            Some("https://s1.baozimh.com/a.webp?token=.bzcdn.net")
        );
        assert_eq!(
            image_request_url("//s2.baozicdn.com/a.jpg").as_deref(),
            Some("https://s2.baozimh.com/a.jpg")
        );
        assert_eq!(
            image_request_url("https://cdn.test/a.bzcdn.net.jpg").as_deref(),
            Some("https://cdn.test/a.bzcdn.net.jpg")
        );
        assert!(image_request_url("javascript:alert(1)").is_none());
    }
}
