// Adapted from Aidoku-Community/sources zh.baozimanhua; see res/NOTICE.txt.
use crate::parser::{self, BASE_URL, ChapterEntry};
use aidoku::{
    AidokuError, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
    Listing, ListingProvider, Manga, MangaPageResult, MangaStatus, Page, PageContent, PageContext,
    Result, Source, Viewer,
    alloc::{String, Vec, format, vec},
    imports::{
        defaults::defaults_get,
        html::{Document, Element, Html},
        net::Request,
    },
    prelude::*,
};

struct Baozimh;

fn base_url() -> String {
    defaults_get::<String>("url")
        .and_then(|url| parser::base_url(&url))
        .unwrap_or_else(|| BASE_URL.into())
}

impl Baozimh {
    fn get(path: &str) -> Result<String> {
        let base = base_url();
        let response = Request::get(format!("{base}{path}"))?
            .header("Origin", &base)
            .header("Referer", &format!("{base}/"))
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .send()?;
        let status = response.status_code();
        let body = response.get_string()?;
        if [
            "challenge_required",
            "cf-chl-",
            "Just a moment",
            "人机验证",
            "人機驗證",
        ]
        .iter()
        .any(|marker| body.contains(marker))
        {
            bail!("包子漫画要求网页验证，请在源网页完成验证，或在源设置中切换备用网址后重试。");
        }
        if status != 200 {
            bail!("包子漫画请求失败：HTTP {status}，可在源设置中切换备用网址。");
        }
        Ok(body)
    }

    fn document(path: &str) -> Result<Document> {
        Ok(Html::parse_with_url(
            Self::get(path)?,
            format!("{}{path}", base_url()),
        )?)
    }

    fn image(element: Element) -> Option<String> {
        ["data-src", "data-original", "src"]
            .iter()
            .filter_map(|attribute| element.attr(attribute))
            .find_map(|value| parser::absolute_image(&value, &base_url()))
    }

    fn meta(document: &Document, name: &str) -> Option<String> {
        document
            .select_first(format!("meta[name='{name}'], meta[property='{name}']"))?
            .attr("content")
            .filter(|value| !value.trim().is_empty())
    }

    fn html_list(path: &str) -> Result<MangaPageResult> {
        let document = Self::document(path)?;
        let mut entries: Vec<Manga> = Vec::new();
        for card in document.select("div.comics-card").into_iter().flatten() {
            let Some(key) = card
                .select_first("a.comics-card__poster")
                .and_then(|anchor| anchor.attr("href"))
                .and_then(|url| parser::manga_key(&url))
            else {
                continue;
            };
            let title = card
                .select_first("h3")
                .and_then(|element| element.text())
                .unwrap_or_default();
            if title.trim().is_empty() || entries.iter().any(|entry| entry.key == key) {
                continue;
            }
            entries.push(Manga {
                url: Some(format!("{}/comic/{key}", base_url())),
                key,
                title,
                cover: card.select_first("amp-img, img").and_then(Self::image),
                ..Default::default()
            });
        }
        // The site's keyword endpoint returns a single result page (no page parameter).
        Ok(MangaPageResult {
            entries,
            has_next_page: false,
        })
    }

    fn list(region: &str, state: &str, genre: &str, page: i32) -> Result<MangaPageResult> {
        let json = Self::get(&parser::list_path(region, state, genre, page))?;
        let (items, has_next_page) = parser::api_list(&json).map_err(AidokuError::message)?;
        let base = base_url();
        Ok(MangaPageResult {
            entries: items
                .into_iter()
                .map(|item| Manga {
                    url: Some(format!("{base}/comic/{}", item.key)),
                    key: item.key,
                    title: item.title,
                    cover: item.cover,
                    ..Default::default()
                })
                .collect(),
            has_next_page,
        })
    }
}

impl Source for Baozimh {
    fn new() -> Self {
        Self
    }

    fn get_search_manga_list(
        &self,
        query: Option<String>,
        page: i32,
        filters: Vec<FilterValue>,
    ) -> Result<MangaPageResult> {
        let query = query.unwrap_or_default();
        let query = query.trim();
        if !query.is_empty() {
            if page > 1 {
                return Ok(MangaPageResult::default());
            }
            if let Some(key) = parser::manga_key(query) {
                return Ok(MangaPageResult {
                    entries: vec![self.get_manga_update(
                        Manga {
                            key,
                            ..Default::default()
                        },
                        true,
                        false,
                    )?],
                    has_next_page: false,
                });
            }
            return Self::html_list(&format!("/search?q={}", parser::urlencode(query)));
        }
        let mut region = String::from("all");
        let mut state = String::from("all");
        let mut genre = String::from("all");
        for filter in filters {
            if let FilterValue::Select { id, value } = filter {
                match id.as_str() {
                    "地區" => region = value,
                    "狀態" => state = value,
                    "類型" => genre = value,
                    _ => {}
                }
            }
        }
        Self::list(&region, &state, &genre, page)
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        if !parser::valid_key(&manga.key) {
            bail!("无效漫画 ID");
        }
        if !needs_details && !needs_chapters {
            return Ok(manga);
        }
        let document = Self::document(&format!("/comic/{}", manga.key))?;
        if needs_details {
            manga.title = Self::meta(&document, "og:novel:book_name")
                .or_else(|| {
                    document
                        .select_first("h1")
                        .and_then(|element| element.text())
                })
                .filter(|title| !title.trim().is_empty())
                .ok_or_else(|| AidokuError::message("包子漫画标题缺失，站点格式可能已变化"))?;
            manga.cover = Self::meta(&document, "og:image")
                .and_then(|url| parser::absolute_image(&url, &base_url()))
                .or_else(|| {
                    document
                        .select_first("amp-img.comic-cover")
                        .and_then(Self::image)
                });
            manga.description = Self::meta(&document, "og:description");
            manga.authors = Self::meta(&document, "og:novel:author").map(|author| {
                let mut authors = Vec::new();
                for name in author
                    .split([',', '，', '、'])
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                {
                    if !authors.iter().any(|author| author == name) {
                        authors.push(String::from(name));
                    }
                }
                authors
            });
            let tags: Vec<String> = document
                .select("span.tag")
                .into_iter()
                .flatten()
                .filter_map(|element| element.text())
                .filter(|tag| {
                    !tag.trim().is_empty()
                        && !["連載中", "连载中", "已完結", "已完结"].contains(&tag.trim())
                })
                .collect();
            manga.viewer = if tags.iter().any(|tag| {
                tag.contains("國漫")
                    || tag.contains("国漫")
                    || tag.contains("韓國")
                    || tag.contains("韩国")
            }) {
                Viewer::Webtoon
            } else if tags.iter().any(|tag| tag.contains("日本")) {
                Viewer::RightToLeft
            } else {
                Viewer::LeftToRight
            };
            manga.tags = Some(tags);
            manga.status = match Self::meta(&document, "og:novel:status").as_deref() {
                Some("連載中" | "连载中") => MangaStatus::Ongoing,
                Some("已完結" | "已完结") => MangaStatus::Completed,
                _ => MangaStatus::Unknown,
            };
            manga.url = Some(format!("{}/comic/{}", base_url(), manga.key));
        }
        if needs_chapters {
            // Both visible and hidden directory containers are required. Ignore the
            // latest-update preview, which duplicates only the newest 24 chapters.
            let full = document
                .select_first("#chapter-items, #chapters_other_list")
                .is_some();
            let selector = if full {
                "#chapter-items a.comics-chapters__item, #chapters_other_list a.comics-chapters__item"
            } else {
                "a.comics-chapters__item"
            };
            let items = document
                .select(selector)
                .into_iter()
                .flatten()
                .filter_map(|anchor| {
                    let key = anchor
                        .attr("href")
                        .and_then(|url| parser::chapter_key(&url, &manga.key))?;
                    Some(ChapterEntry {
                        key,
                        title: anchor.text().unwrap_or_default(),
                    })
                })
                .collect();
            let entries = parser::ordered_chapters(items, full);
            if entries.is_empty() {
                bail!("包子漫画章节目录为空，站点格式可能已变化");
            }
            manga.chapters = Some(
                entries
                    .into_iter()
                    .map(|entry| {
                        let volume = entry.title.trim().ends_with('卷');
                        let number = parser::chapter_number(&entry.title);
                        Chapter {
                            url: parser::chapter_path(&manga.key, &entry.key)
                                .map(|path| format!("{}{path}", base_url())),
                            key: entry.key,
                            chapter_number: if volume { None } else { number },
                            volume_number: if volume { number } else { None },
                            title: Some(entry.title),
                            language: Some("zh".into()),
                            ..Default::default()
                        }
                    })
                    .collect(),
            );
        }
        Ok(manga)
    }

    fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        let path = parser::chapter_path(&manga.key, &chapter.key)
            .ok_or_else(|| AidokuError::message("无效章节 ID"))?;
        // page_direct follows the site's redirect to its current reader host.
        let document = Self::document(&path)?;
        let mut urls: Vec<String> = Vec::new();
        for image in document
            .select("amp-img.comic-contain__item, img.comic-contain__item")
            .into_iter()
            .flatten()
        {
            if let Some(url) = Self::image(image) {
                // Keep DOM order, including every AMP image group.
                if !urls.contains(&url) {
                    urls.push(url);
                }
            }
        }
        if urls.is_empty() {
            bail!("包子漫画章节图片为空，请先在源网页完成验证或切换备用网址");
        }
        Ok(urls
            .into_iter()
            .map(|url| Page {
                content: PageContent::url(url),
                ..Default::default()
            })
            .collect())
    }
}

impl ListingProvider for Baozimh {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        match listing.id.as_str() {
            "home" if page > 1 => Ok(MangaPageResult::default()),
            "home" => Self::html_list("/"),
            "all" | "cn" | "jp" | "kr" | "en" => Self::list(&listing.id, "all", "all", page),
            _ => bail!("未知分类"),
        }
    }
}

impl ImageRequestProvider for Baozimh {
    fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
        let url =
            parser::image_request_url(&url).ok_or_else(|| AidokuError::message("无效图片地址"))?;
        Ok(Request::get(url)?.header("Referer", &format!("{}/", base_url())))
    }
}

impl DeepLinkHandler for Baozimh {
    fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
        Ok(parser::manga_key(&url).map(|key| DeepLinkResult::Manga { key }))
    }
}

register_source!(
    Baozimh,
    ListingProvider,
    ImageRequestProvider,
    DeepLinkHandler
);
