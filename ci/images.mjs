// Dessine les images de public/ : les icônes de l'application installée (PWA) et l'image
// de partage (WhatsApp, Messenger, Signal, Telegram, iMessage, Slack, Discord, LinkedIn…).
//
//   node ci/images.mjs
//
// Les PNG sont versionnés : ce script ne sert qu'à les refaire après un changement ici.
import { chromium } from 'playwright';

const colors = {
  bg: '#0b0d17',
  bg2: '#11142a',
  surface: '#161a33',
  line: '#2a3060',
  text: '#eef0ff',
  muted: '#a3a9d6',
  rust: '#ff7a3d',
  rust2: '#ffb36b',
  docker: '#38bdf8',
  compose: '#c084fc',
  k8s: '#4ade80',
};

// La note du logo (le favicon), dans un carré de 32 × 32.
const note = (fill = colors.rust, flag = colors.rust2) =>
  `<ellipse cx="13" cy="21" rx="5" ry="3.6" transform="rotate(-20 13 21)" fill="${fill}"/>` +
  `<rect x="16.6" y="6" width="2.2" height="15" rx="1" fill="${fill}"/>` +
  `<path d="M18.8 6q7 3 5 10q-1-5-5-6z" fill="${flag}"/>`;

// Une icône carrée. `zone` : la part du carré occupée par le dessin (les icônes
// « maskable » sont découpées en cercle par Android, le dessin reste au centre).
const icon = (zone) => {
  const offset = (32 - 32 * zone) / 2;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
  <defs><radialGradient id="g" cx=".3" cy=".2" r="1"><stop offset="0" stop-color="${colors.bg2}"/><stop offset="1" stop-color="${colors.bg}"/></radialGradient></defs>
  <rect width="32" height="32" fill="url(#g)"/>
  <g transform="translate(${offset} ${offset}) scale(${zone})">
    <path d="M3 11h26M3 16h26M3 21h26" stroke="${colors.line}" stroke-width="1.3" stroke-linecap="round"/>
    ${note()}
  </g>
</svg>`;
};

const share = `<!doctype html>
<html><head><meta charset="utf-8"><style>
  * { margin: 0; box-sizing: border-box; }
  body {
    width: 1200px; height: 630px; overflow: hidden; position: relative;
    font-family: Inter, system-ui, sans-serif; color: ${colors.text};
    background:
      radial-gradient(900px 520px at 85% 10%, rgb(192 132 252 / .16), transparent 60%),
      radial-gradient(700px 500px at 0% 100%, rgb(255 122 61 / .18), transparent 60%),
      ${colors.bg};
  }
  .staff { position: absolute; inset: 0; }
  .content { position: absolute; left: 80px; top: 72px; right: 80px; }
  .brand { display: flex; align-items: center; gap: 22px; }
  .brand svg { width: 92px; height: 92px; border-radius: 22px; box-shadow: 0 18px 40px -18px #000; }
  .brand span { font-size: 64px; font-weight: 850; letter-spacing: .06em; }
  h1 { margin-top: 42px; font-size: 58px; line-height: 1.12; font-weight: 800; letter-spacing: -.01em; max-width: 960px; }
  h1 em { font-style: normal; color: ${colors.rust}; }
  .code {
    margin-top: 40px; display: inline-block; padding: 22px 30px; border-radius: 18px;
    background: ${colors.surface}; border: 2px solid ${colors.line};
    font: 600 34px/1 "DejaVu Sans Mono", ui-monospace, monospace; color: ${colors.text};
  }
  .code .k { color: ${colors.rust2}; }
  .code .p { color: ${colors.muted}; }
  .targets { position: absolute; left: 80px; bottom: 64px; display: flex; gap: 18px; }
  .chip {
    font-size: 26px; font-weight: 700; padding: 11px 22px; border-radius: 999px;
    border: 2px solid currentColor; background: rgb(255 255 255 / .03);
  }
  .url { position: absolute; right: 80px; bottom: 76px; font-size: 24px; color: ${colors.muted}; font-weight: 600; }
</style></head><body>
  <svg class="staff" viewBox="0 0 1200 630" preserveAspectRatio="none" aria-hidden="true">
    <g fill="none" stroke="${colors.line}" stroke-width="2" opacity=".6">
      <path d="M680 86 Q840 56 960 86 T1260 86"/><path d="M680 114 Q840 84 960 114 T1260 114"/>
      <path d="M680 142 Q840 112 960 142 T1260 142"/><path d="M680 170 Q840 140 960 170 T1260 170"/>
      <path d="M680 198 Q840 168 960 198 T1260 198"/>
    </g>
    <g transform="translate(820 70) scale(3.2)">${note(colors.docker, colors.docker)}</g>
    <g transform="translate(920 88) scale(3.2)">${note(colors.compose, colors.compose)}</g>
    <g transform="translate(1020 106) scale(3.2)">${note(colors.k8s, colors.k8s)}</g>
  </svg>
  <div class="content">
    <div class="brand">${icon(1)}<span>ORCHESTRA</span></div>
    <h1>Un fichier <em>.drc</em>, et Docker, Compose et Kubernetes suivent.</h1>
    <div class="code"><span class="k">web</span> <span class="p">{</span> rust nginx postgres redis <span class="p">}</span></div>
  </div>
  <div class="targets">
    <span class="chip" style="color:${colors.docker}">Dockerfile</span>
    <span class="chip" style="color:${colors.compose}">Compose</span>
    <span class="chip" style="color:${colors.k8s}">Kubernetes</span>
  </div>
  <div class="url">maxlestage.github.io/ORCHESTRA</div>
</body></html>`;

const svgPage = (svg, size) =>
  `<!doctype html><html><head><style>*{margin:0}svg{display:block;width:${size}px;height:${size}px}</style></head><body>${svg}</body></html>`;

// La forme arrondie de l'icône « any » : les systèmes l'affichent telle quelle.
const rounded = (svg) => svg.replace('<rect width="32" height="32"', '<rect width="32" height="32" rx="7"');

const images = [
  { file: 'public/icon-192.png', html: svgPage(rounded(icon(0.86)), 192), width: 192, height: 192, transparent: true },
  { file: 'public/icon-512.png', html: svgPage(rounded(icon(0.86)), 512), width: 512, height: 512, transparent: true },
  // Android découpe en cercle : le dessin tient dans la zone sûre (80 % au centre).
  { file: 'public/icon-maskable-512.png', html: svgPage(icon(0.68), 512), width: 512, height: 512 },
  // iOS arrondit lui-même les coins et n'accepte pas la transparence.
  { file: 'public/apple-touch-icon.png', html: svgPage(icon(0.8), 180), width: 180, height: 180 },
  // En JPEG : WhatsApp n'affiche pas l'aperçu d'une image de plus de 300 Ko environ.
  { file: 'public/partage.jpg', html: share, width: 1200, height: 630, quality: 90 },
];

const browser = await chromium.launch({ channel: process.env.PW_CHANNEL || undefined });
for (const image of images) {
  const page = await browser.newPage({ viewport: { width: image.width, height: image.height } });
  await page.setContent(image.html);
  await page.screenshot({
    path: image.file,
    omitBackground: !!image.transparent,
    ...(image.quality ? { type: 'jpeg', quality: image.quality } : {}),
  });
  await page.close();
  console.log(`✓ ${image.file} (${image.width} × ${image.height})`);
}
await browser.close();
