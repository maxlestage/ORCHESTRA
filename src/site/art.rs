//! Les illustrations du site : du SVG en ligne, animé par le CSS (classes `a-*`)
//! et par SMIL (`animateMotion`) pour les notes qui suivent les partitions.
//! Tout s'arrête avec `prefers-reduced-motion`.

/// La scène d'accueil : le fichier `.drc` (la partition) envoie des notes à trois
/// instruments, Dockerfile, Compose et Kubernetes.
pub const HERO: &str = r##"<svg class="hero-art" viewBox="0 0 400 320" role="img" aria-labelledby="hero-art-title" xmlns="http://www.w3.org/2000/svg">
<title id="hero-art-title">Un fichier .drc envoie des notes vers un Dockerfile, un fichier Compose et des manifests Kubernetes</title>
<defs>
  <path id="p-docker" d="M140 148 C205 148 215 62 280 62"/>
  <path id="p-compose" d="M140 160 C205 160 215 160 280 160"/>
  <path id="p-k8s" d="M140 172 C205 172 215 262 280 262"/>
  <g id="note"><ellipse cx="0" cy="0" rx="5.2" ry="3.8" transform="rotate(-20)"/><rect x="3.6" y="-15" width="1.8" height="15" rx=".9"/><path d="M5.4 -15 q6 3 4 9 q-.5 -4 -4 -5z"/></g>
</defs>
<g class="staff" aria-hidden="true">
  <path d="M-10 112 Q100 92 200 112 T410 112"/><path d="M-10 136 Q100 116 200 136 T410 136"/><path d="M-10 160 Q100 140 200 160 T410 160"/><path d="M-10 184 Q100 164 200 184 T410 184"/><path d="M-10 208 Q100 188 200 208 T410 208"/>
</g>
<g class="wires" aria-hidden="true">
  <use href="#p-docker" class="wire w-docker"/><use href="#p-compose" class="wire w-compose"/><use href="#p-k8s" class="wire w-k8s"/>
  <use href="#p-docker" class="flow w-docker"/><use href="#p-compose" class="flow w-compose"/><use href="#p-k8s" class="flow w-k8s"/>
</g>
<g class="baton" aria-hidden="true"><line x1="64" y1="78" x2="118" y2="40"/><circle cx="64" cy="78" r="4"/></g>
<g class="doc">
  <path d="M28 88 h86 l22 22 v124 a10 10 0 0 1 -10 10 h-98 a10 10 0 0 1 -10 -10 v-136 a10 10 0 0 1 10 -10z" class="doc-sheet"/>
  <path d="M114 88 v14 a8 8 0 0 0 8 8 h14z" class="doc-fold"/>
  <text x="34" y="112" class="doc-title">app.drc</text>
  <rect class="bar" x="34" y="126" width="44" height="6" rx="3"/>
  <rect class="bar" x="42" y="140" width="70" height="6" rx="3"/>
  <rect class="bar" x="42" y="154" width="56" height="6" rx="3"/>
  <rect class="bar" x="34" y="168" width="38" height="6" rx="3"/>
  <rect class="bar" x="42" y="182" width="78" height="6" rx="3"/>
  <rect class="bar" x="42" y="196" width="50" height="6" rx="3"/>
  <rect class="bar" x="34" y="210" width="20" height="6" rx="3"/>
  <rect class="cursor" x="58" y="208" width="2.5" height="10" rx="1"/>
</g>
<g class="target t-docker">
  <g class="a-bob">
    <rect x="284" y="34" width="92" height="56" rx="8" class="box"/>
    <path d="M300 42 v40 M314 42 v40 M328 42 v40 M342 42 v40 M356 42 v40" class="ridges"/>
  </g>
  <text x="330" y="108" class="label">Dockerfile</text>
</g>
<g class="target t-compose">
  <rect x="300" y="168" width="60" height="18" rx="5" class="box a-stack s1"/>
  <rect x="300" y="148" width="60" height="18" rx="5" class="box a-stack s2"/>
  <rect x="300" y="128" width="60" height="18" rx="5" class="box a-stack s3"/>
  <text x="330" y="206" class="label">compose.yml</text>
</g>
<g class="target t-k8s">
  <g class="a-spin">
    <circle cx="330" cy="262" r="26" class="box"/>
    <circle cx="330" cy="262" r="7" class="hub"/>
    <path d="M330 262 L330 236 M330 262 L350.3 245.8 M330 262 L355.3 267.8 M330 262 L341.3 285.4 M330 262 L318.7 285.4 M330 262 L304.7 267.8 M330 262 L309.7 245.8" class="spokes"/>
  </g>
  <text x="330" y="308" class="label">k8s.yaml</text>
</g>
<g class="notes" aria-hidden="true">
  <use href="#note" class="n-docker"><animateMotion dur="3.2s" repeatCount="indefinite" begin="0s"><mpath href="#p-docker"/></animateMotion></use>
  <use href="#note" class="n-docker"><animateMotion dur="3.2s" repeatCount="indefinite" begin="-1.6s"><mpath href="#p-docker"/></animateMotion></use>
  <use href="#note" class="n-compose"><animateMotion dur="2.8s" repeatCount="indefinite" begin="-.5s"><mpath href="#p-compose"/></animateMotion></use>
  <use href="#note" class="n-compose"><animateMotion dur="2.8s" repeatCount="indefinite" begin="-1.9s"><mpath href="#p-compose"/></animateMotion></use>
  <use href="#note" class="n-k8s"><animateMotion dur="3.4s" repeatCount="indefinite" begin="-.9s"><mpath href="#p-k8s"/></animateMotion></use>
  <use href="#note" class="n-k8s"><animateMotion dur="3.4s" repeatCount="indefinite" begin="-2.6s"><mpath href="#p-k8s"/></animateMotion></use>
</g>
</svg>"##;

/// Étape 1 : écrire le fichier (un curseur qui clignote, des lignes qui s'écrivent).
pub const STEP_WRITE: &str = r##"<svg viewBox="0 0 64 64" class="step-art" aria-hidden="true"><rect x="10" y="6" width="44" height="52" rx="7" class="s-sheet"/><rect x="18" y="18" width="22" height="4" rx="2" class="s-line a-type" style="--d:0s"/><rect x="18" y="28" width="28" height="4" rx="2" class="s-line a-type" style="--d:.4s"/><rect x="18" y="38" width="16" height="4" rx="2" class="s-line a-type" style="--d:.8s"/><rect x="37" y="36" width="2" height="8" class="s-cursor"/></svg>"##;

/// Étape 2 : compiler (deux engrenages qui tournent en sens inverse).
pub const STEP_COMPILE: &str = r##"<svg viewBox="0 0 64 64" class="step-art" aria-hidden="true"><g class="a-gear" style="transform-origin:24px 26px"><circle cx="24" cy="26" r="11" class="s-gear"/><path d="M24 9v6M24 37v6M7 26h6M35 26h6M12 14l4 4M32 34l4 4M12 38l4-4M32 18l4-4" class="s-teeth"/></g><g class="a-gear rev" style="transform-origin:44px 44px"><circle cx="44" cy="44" r="7" class="s-gear alt"/><path d="M44 33v4M44 51v4M33 44h4M51 44h4" class="s-teeth alt"/></g></svg>"##;

/// Étape 3 : déployer (trois conteneurs qui montent vers le nuage).
pub const STEP_DEPLOY: &str = r##"<svg viewBox="0 0 64 64" class="step-art" aria-hidden="true"><path d="M18 22a10 10 0 0 1 19-4a8 8 0 0 1 11 8h-30a6 6 0 0 1 0-4z" class="s-cloud"/><rect x="14" y="46" width="10" height="10" rx="2" class="s-box a-rise" style="--d:0s"/><rect x="27" y="46" width="10" height="10" rx="2" class="s-box a-rise alt" style="--d:.35s"/><rect x="40" y="46" width="10" height="10" rx="2" class="s-box a-rise alt2" style="--d:.7s"/></svg>"##;

/// Les icônes des cinq principes.
pub const ICON_DECLARATIVE: &str = r##"<svg viewBox="0 0 32 32" class="icon" aria-hidden="true"><path d="M6 9l3 3 5-6M6 20l3 3 5-6M18 9h9M18 20h9"/></svg>"##;
pub const ICON_MODULAR: &str = r##"<svg viewBox="0 0 32 32" class="icon" aria-hidden="true"><rect x="5" y="5" width="9" height="9" rx="2"/><rect x="18" y="5" width="9" height="9" rx="2" class="i-pop"/><rect x="5" y="18" width="9" height="9" rx="2" class="i-pop d2"/><rect x="18" y="18" width="9" height="9" rx="2"/></svg>"##;
pub const ICON_FUNCTIONS: &str = r##"<svg viewBox="0 0 32 32" class="icon" aria-hidden="true"><path d="M14 6c-4 0-4 3-4 6v8c0 3 0 6-4 6M5 14h9M18 12l8 10M26 12l-8 10"/></svg>"##;
pub const ICON_TYPED: &str = r##"<svg viewBox="0 0 32 32" class="icon" aria-hidden="true"><path d="M16 4l10 4v7c0 6-4 11-10 13C10 26 6 21 6 15V8z"/><path d="M11 16l4 4 6-8" class="i-check"/></svg>"##;
pub const ICON_READABLE: &str = r##"<svg viewBox="0 0 32 32" class="icon" aria-hidden="true"><path d="M3 16s5-9 13-9 13 9 13 9-5 9-13 9S3 16 3 16z"/><circle cx="16" cy="16" r="4" class="i-blink"/></svg>"##;

/// Le logo : une note posée sur une portée, en dégradé.
pub const LOGO: &str = r##"<svg viewBox="0 0 32 32" class="logo" aria-hidden="true"><path d="M2 10h28M2 16h28M2 22h28" class="logo-staff"/><g class="logo-note"><ellipse cx="13" cy="22" rx="5" ry="3.6" transform="rotate(-20 13 22)"/><rect x="16.6" y="6" width="2.2" height="16" rx="1"/><path d="M18.8 6q7 3 5 10q-1-5-5-6z"/></g></svg>"##;

/// La page introuvable : une note qui tombe de la portée.
pub const LOST: &str = r##"<svg viewBox="0 0 240 160" class="lost-art" role="img" aria-label="Une note qui tombe de la portée"><g class="staff"><path d="M10 40H230M10 56H230M10 72H230M10 88H230M10 104H230"/></g><use href="#lost-note" x="70" y="72" class="n-docker"/><use href="#lost-note" x="120" y="56" class="n-compose"/><g class="a-fall"><use href="#lost-note" x="170" y="88" class="n-k8s"/></g><defs><g id="lost-note"><ellipse cx="0" cy="0" rx="7" ry="5" transform="rotate(-20)"/><rect x="5" y="-22" width="2.4" height="22" rx="1"/></g></defs></svg>"##;
