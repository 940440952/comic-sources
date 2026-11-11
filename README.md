# 双生态漫画源仓库

同一个漫画网站同时维护两套原生扩展：**Venera / VeneraNext / VeneraPlus 的 JavaScript 源**，以及 **Aidoku 的 Rust / WASM 源**。当前支持 [漫画站 manhuazhan.com](https://m.manhuazhan.com)（双生态）、[嗨皮漫画 Hipmh](https://m.hipmh.com) 和 [包子漫画](https://www.twmanga.com)（后两者仅 Aidoku）。仓库名为 `comic-sources`，请使用下方新订阅地址。

## Venera 系列订阅与安装

在 Venera、VeneraNext 或 VeneraPlus 的「漫画源 / 源仓库」设置中添加此订阅地址，再安装「漫画站」（不同版本的菜单名称可能略有不同）：

```text
https://cdn.jsdelivr.net/gh/940440952/comic-sources@main/index.json
```

也可下载 [venera.js](https://cdn.jsdelivr.net/gh/940440952/comic-sources@main/sources/manhuazhan/venera.js)，使用应用的本地文件导入功能安装。源要求兼容 Venera 1.6.0 及以上的源 API。

## Aidoku 订阅与安装

在支持新版源 API 的 **Aidoku 0.7.0 及以上**版本中（包子漫画需要 **0.7.1 及以上**），进入「设置 → 源列表」，添加以下 JSON 地址，再前往浏览页的添加源页面安装「漫画站」「嗨皮漫画」或「包子漫画」；如未显示，请确认已启用中文语言筛选。

```text
https://cdn.jsdelivr.net/gh/940440952/comic-sources@main/aidoku/index.json
```

也可从 [Releases](https://github.com/940440952/comic-sources/releases) 下载 `zh.manhuazhan-v3.aix`、`zh.hipmh-v1.aix` 或 `zh.baozimh-v1.aix`，通过系统分享菜单用 Aidoku 打开，或在 Aidoku 添加源界面选择本地 `.aix` 文件。Rust 源码不能直接导入 Aidoku，须使用构建后的 `.aix` 包。此实现使用新版 Aidoku API，不适用于旧版 legacy 源运行时。

Aidoku 源 v3 将漫画站默认的「旧 → 新」DOM 章节列表整体反转一次后返回，保留上、中、下和特殊章节的相对次序，不再用章节编号或 key 推断方向。阅读器按原始列表切换章节，目录界面的排序设置不能用于验证阅读方向。更新已安装的源后，请退出阅读器，在漫画详情页刷新章节，再重新进入阅读。

首次推送后，GitHub Actions 完成检查和构建才会发布 `.aix`；如安装时提示文件不存在，请先查看 [构建状态](https://github.com/940440952/comic-sources/actions)。

## 当前支持与验证

| 功能 | Venera 系列 JS | Aidoku Rust / WASM |
| --- | --- | --- |
| 首页、全部与国产 / 日本 / 韩国 / 欧美分类 | 已实现 | 已实现 |
| 分类翻页、详情、作者、简介和章节 | 已实现 | 已实现 |
| 当前加密 DATA 图片解析、图片 Referer | 已实现 | 已实现 |
| HTML 懒加载图片与旧版 JSON newImgs | 已实现 | 已实现 |
| 关键词搜索 | 已实现；目前站点要求验证码 | 已实现；目前站点要求验证码 |
| 数字漫画 ID 搜索、漫画链接跳转 | 已实现 | 已实现 |
| 手机应用实机验证 | 待验证 | 待验证 |

2026-10-16 检查：JS 适配器通过当前站点的首页、分类、详情和章节图片请求；《斗罗大陆》（ID `219430`）返回 996 个章节条目、末章 33 张图片，首张图片下载成功。Rust 源已编译为 WASM，且两套解码实现均通过当前站点数据回归测试。这些检查不等同于所有应用、所有漫画及所有网络环境均已验证。

关键词搜索遇验证码时会明确报错，保留首页和分类入口；也可在搜索框输入漫画详情网址中的数字 ID，例如 `219430`。扩展不自动处理验证码。旧版 `newImgs` 兼容限于 JSON 数组，不执行站点任意脚本。

## 仓库结构

```text
index.json                          Venera 系列订阅目录
aidoku/index.json                   Aidoku 订阅目录
sources/
  baozimh/aidoku/                  包子漫画 Rust 源、元数据与上游说明
  hipmh/aidoku/                    Hipmh Rust 源、元数据与上游说明
  manhuazhan/
    venera.js                      JS 实现
    aidoku/
      Cargo.toml / Cargo.lock      Rust 项目与依赖锁定
      src/                         Rust 实现与图片解析
      res/source.json              Aidoku 元数据和分类入口
tests/fixtures/manhuazhan/          两套实现共用的解码回归数据
scripts/build.py                   构建 .aix 并生成两套订阅目录
scripts/smoke.cjs                  手动执行的真实站点检查
.github/workflows/check.yml        回归检查、WASM 构建及版本发布
```

按网站划分目录，而非按阅读器拆开站点；未来新增网站时，在 `sources/<站点>/` 下按需加入 `venera.js`、完整的 `aidoku/` 项目，或两者。构建脚本扫描所有站点并生成各生态目录，文件名与 Rust 包名使用同一站点标识。

## 开发与发布

需要 Node.js 22、Python 3 和支持 Rust 2024 的稳定版 Rust，并安装 `wasm32-unknown-unknown` 目标与 rustfmt。

```sh
npm ci --ignore-scripts
npm test
for source in sources/*/aidoku/Cargo.toml; do
  cargo fmt --manifest-path "$source" -- --check
  cargo test --locked --manifest-path "$source"
done
python3 scripts/build.py
```

`dist/` 中的 `.aix` 包含 `Payload/main.wasm`、`Payload/source.json` 及图标，可用于本地安装。仅刷新订阅目录时运行 `python3 scripts/build.py --catalogs-only`。修改 Rust 后执行 `cargo fmt --manifest-path sources/manhuazhan/aidoku/Cargo.toml`。

`npm run smoke` 访问真实站点并下载一张图片，可能受站点限流或网络影响；日常 CI 使用固定回归样例，避免把站点暂时不可达当作代码失败。

两套源独立版本：修改 JS 时递增 `venera.js` 的版本号；修改 Rust 时递增 `res/source.json` 的整数版本号。重新生成并提交两套目录。推送 `main` 后 CI 先检查 JS、Rust 测试、格式和 WASM 构建，再将 `.aix` 上传为构建产物，并按 `aidoku-<源 ID>-v<版本>` 创建 Release。已存在的版本不会覆盖，修复须升版。

jsDelivr 有缓存，更新源后可能需要等待。反馈请提交 Issues，注明阅读器名称、版本、漫画 ID、章节及报错信息。

接口依据：[Venera 源 API](https://github.com/venera-app/venera/blob/master/doc/comic_source.md)、[Aidoku Rust API](https://github.com/Aidoku/aidoku-rs)、[Aidoku 源目录读取实现](https://github.com/Aidoku/Aidoku/blob/main/Aidoku/Core/Sources/SourceList/ExternalSourceInfo.swift)。

## Hipmh Aidoku

`zh.hipmh` v1 使用当前 Hipmh API，支持搜索、最近更新、热门漫画、详情、多页章节、阅读页配置、图片 payload 解码、诱饵图删除和图片 Referer。章节请求显式使用 `order=desc`，直接保持新 → 旧顺序，不额外反转。支持 `/works/...` 和 reader 章节链接。

实现依据为 [Onedaytalk/tachimanga-hipmh-extension](https://github.com/Onedaytalk/tachimanga-hipmh-extension/tree/031bf1ab665a489593fc77d270e4c6f56bfd07f5)，已改写为 Aidoku Rust 接口；来源及许可见 `sources/hipmh/aidoku/res/NOTICE.txt` 和 `LICENSE.txt`。固定测试使用真实章节 payload，并覆盖格式损坏、URL 和诱饵处理。

此前聊天生成的 ZIP 未迁移到此工作环境，本实现从上述上游源码重新移植。手机 Aidoku 实机体验仍需在设备上验证。

## 包子漫画 Aidoku

`zh.baozimh` v1 支持首页、全部及国产 / 日本 / 韩国 / 欧美分类、分类分页、地区 / 连载状态 / 类型筛选、关键词搜索、漫画链接搜索与跳转、详情、完整章节目录及图片 Referer。关键词搜索接口只有一页结果，翻到下一页不会重复请求。源 ID 与社区的 `zh.baozimanhua` 区分，可以同时安装。

默认入口为 `https://www.twmanga.com`，这是包子漫画同站备用网址，并非 `baozimh.org`。2026-11-11 检查时 `www.baozimh.com` 等部分入口要求网页验证，twmanga 可正常访问；可在 Aidoku 源设置中切换备用网址，遇验证时先在源网页处理。不会自动绕过验证码。

完整目录包含可见的 `chapter-items` 和隐藏的 `chapters_other_list`，忽略重复的最新章节预览，按 DOM 顺序整体反转为新 → 旧；不按章节号重新排序。章节键同时保存 `section_slot` 和 `chapter_slot`，保留不同分区的同号章节。阅读请求跟随 `page_direct` 跳转，按 DOM 顺序读取所有 AMP 图片分组；只在图片主机名中将 `.bzcdn.net` / `.baozicdn.com` 换为 `.baozimh.com`，保留路径和查询参数。

2026-11-11 检查：首页、20 条分类列表及下一页、关键词搜索、漫画详情和阅读页可访问。《一人之下》完整目录 823 章（另有 24 条重复预览），最新章「783 新三賢」解析到 24 张图片，首图下载为有效 JPEG（435520 字节）。真实分类和目录样例用于 Rust 回归测试；WASM 编译与 `.aix` 打包已验证。尚未进行 iPhone / iPad Aidoku 实机验证。

运行 `npm run smoke:baozimh` 可重新检查真实站点，网络或站点临时不可用时会失败；普通 CI 仅使用固定样例。安装版本合入 `main` 后，现有 Actions 流程自动发布 `.aix`，订阅地址不变。

实现改编自 [Aidoku-Community/sources 包子源](https://github.com/Aidoku-Community/sources/tree/4ac84147993976d822f62632740d8870903279b9/sources/zh.baozimanhua)，采用 MIT 许可，来源及许可随 `.aix` 一并打包，见 `sources/baozimh/aidoku/res/NOTICE.txt` 和 `LICENSE.txt`。
