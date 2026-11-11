// Public-site contract check; this does not run the iOS Aidoku runtime.
const assert = require('node:assert/strict');
const {execFileSync} = require('node:child_process');
const {load} = require('cheerio');
const metadata = require('../sources/baozimh/aidoku/res/source.json');
const base = metadata.info.url;
const get = address => execFileSync('curl', [
    '-fsSL', '--max-time', '30', '-H', `Referer: ${base}/`,
    '-H', `Origin: ${base}`, address,
], {maxBuffer: 12 * 1024 * 1024});
const html = path => load(get(new URL(path, base).href).toString());

const home = html('/')('div.comics-card').length;
assert(home > 0, 'Homepage returned no manga');
const list = page => JSON.parse(get(`${base}/api/bzmhq/amp_comic_list?type=all&region=all&state=all&filter=*&page=${page}`));
const first = list(1);
const second = list(2);
assert(first.items.length && second.items.length, 'Classification returned no manga');
assert.notEqual(first.items[0].comic_id, second.items[0].comic_id, 'Repeated category page');
const search = html(`/search?q=${encodeURIComponent('斗罗')}`)('div.comics-card').length;
assert(search > 0, 'Keyword search returned no manga');
const manga = first.items[0].comic_id;
const $ = html(`/comic/${manga}`);
const title = $("meta[name='og:novel:book_name']").attr('content');
assert(title, 'Missing manga title');
const chapters = $('#chapter-items a.comics-chapters__item, #chapters_other_list a.comics-chapters__item').toArray();
assert(chapters.length > 24, 'Full hidden directory missing');
const newest = $(chapters.at(-1)).attr('href');
const reader = html(newest);
const images = reader('amp-img.comic-contain__item, img.comic-contain__item').toArray()
    .map(el => reader(el).attr('data-src') || reader(el).attr('src')).filter(Boolean);
assert(images.length, 'No chapter images');
const image = new URL(images[0]);
image.hostname = image.hostname.replace(/\.(bzcdn\.net|baozicdn\.com)$/, '.baozimh.com');
const bytes = get(image.href);
assert(bytes[0] === 0xff && bytes[1] === 0xd8 || bytes.subarray(1, 4).toString() === 'PNG' || bytes.subarray(0, 4).toString() === 'RIFF', 'Image request returned no image');
console.log(JSON.stringify({base, home, category: first.items.length, nextCategory: second.items.length, search, title, chapters: chapters.length, pages: images.length, imageBytes: bytes.length}, null, 2));
