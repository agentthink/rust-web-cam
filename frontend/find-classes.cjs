const fs = require('fs');
const path = require('path');
const allowed = ['el-', 'status-', 'page-', 'toolbar-', 'body-content', 'header-', 'aside-', 'main-', 'logo-mark', 'logo-text', 'aside-logo', 'aside-footer', 'aside-footer-top', 'aside-footer-bottom', 'ws-row', 'user-row'];
const files = [];
function walk(d) {
  fs.readdirSync(d, { withFileTypes: true }).forEach(e => {
    const f = path.join(d, e.name);
    e.isDirectory() ? walk(f) : e.name.endsWith('.vue') && files.push(f);
  });
}
walk('src');
files.forEach(f => {
  const c = fs.readFileSync(f, 'utf8');
  const matches = c.match(/class="[^"]+"/g) || [];
  const custom = matches.filter(m => {
    const cls = m.replace('class="', '').replace('"', '');
    return !allowed.some(p => cls.startsWith(p));
  });
  if (custom.length) console.log(f.replace('src\\', ''), '|', custom.join(' '));
});
