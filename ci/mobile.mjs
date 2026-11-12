// Vérifie qu'aucune page ne défile horizontalement, du plus petit téléphone au bureau,
// en clair et en sombre, avec tous les blocs dépliés.
//
//   node ci/mobile.mjs http://localhost:3000
//   node ci/mobile.mjs https://maxlestage.github.io/ORCHESTRA   (le site publié)
import { chromium } from 'playwright';

const base = process.argv[2] ?? 'http://localhost:3000';
const pages = ['/', '/guide', '/langage', '/bac-a-sable', '/bac-a-sable/erreur', '/bac-a-sable/k8s', '/introuvable'];
const widths = [320, 360, 375, 390, 414, 768, 1024, 1280, 1440];

// PW_CHANNEL=chrome : le Chrome installé sur la machine (celui des runners GitHub).
const browser = await chromium.launch({ channel: process.env.PW_CHANNEL || undefined });
let problems = 0;
for (const colorScheme of ['dark', 'light']) {
  const page = await browser.newPage({ colorScheme });
  for (const width of widths) {
    await page.setViewportSize({ width, height: 800 });
    for (const path of pages) {
      await page.goto(base + path);
      await page.evaluate(() => document.querySelectorAll('details').forEach((d) => (d.open = true)));
      const result = await page.evaluate(() => {
        const limit = document.documentElement.clientWidth + 0.5;
        const outside = [...document.querySelectorAll('body *')]
          .filter((el) => !el.closest('svg') || el.tagName === 'svg')
          .filter((el) => {
            const box = el.getBoundingClientRect();
            return box.width > 0 && (box.right > limit || box.left < -0.5);
          })
          .map((el) => el.tagName.toLowerCase() + (el.id ? '#' + el.id : ''));
        return {
          scroll: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth),
          width: document.documentElement.clientWidth,
          outside: outside.slice(0, 5),
        };
      });
      if (result.scroll > result.width || result.outside.length) {
        problems++;
        console.error(`✗ ${colorScheme} ${width}px ${path}`, JSON.stringify(result));
      }
    }
  }
  await page.close();
}
await browser.close();
if (problems) {
  console.error(`${problems} page(s) dépassent en largeur`);
  process.exit(1);
}
console.log(`✓ aucun défilement horizontal (${pages.length} pages × ${widths.length} largeurs × 2 thèmes)`);
