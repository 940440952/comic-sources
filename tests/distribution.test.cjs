const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');

const root = path.join(__dirname, '..');

test('catalogs refer to the implemented source with synchronized independent versions', () => {
    const venera = JSON.parse(fs.readFileSync(path.join(root, 'index.json'), 'utf8'));
    const aidoku = JSON.parse(fs.readFileSync(path.join(root, 'aidoku/index.json'), 'utf8'));
    const metadata = JSON.parse(fs.readFileSync(path.join(root, 'sources/manhuazhan/aidoku/res/source.json'), 'utf8')).info;
    const code = fs.readFileSync(path.join(root, 'sources/manhuazhan/venera.js'), 'utf8');
    assert.equal(venera.length, 1);
    assert.equal(venera[0].version, code.match(/\bversion\s*=\s*"([^"]+)"/)[1]);
    assert.equal(venera[0].url, code.match(/\burl\s*=\s*"([^"]+)"/)[1]);
    assert.deepEqual(aidoku.sources.map(source => source.id).sort(), ['zh.baozimh', 'zh.hipmh', 'zh.manhuazhan']);
    for (const entry of aidoku.sources) {
        const sourceName = entry.id.split('.')[1];
        const info = JSON.parse(fs.readFileSync(path.join(root, `sources/${sourceName}/aidoku/res/source.json`), 'utf8')).info;
        assert.equal(entry.version, info.version);
        assert.equal(entry.minAppVersion, info.minAppVersion);
        assert.equal(entry.downloadURL, `https://github.com/940440952/comic-sources/releases/download/aidoku-${info.id}-v${info.version}/${info.id}-v${info.version}.aix`);
    }
    assert.equal(aidoku.sources.find(source => source.id === metadata.id).version, metadata.version);
    assert.equal(venera[0].url, 'https://cdn.jsdelivr.net/gh/940440952/comic-sources@main/sources/manhuazhan/venera.js');
});
