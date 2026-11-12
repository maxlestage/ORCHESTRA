# ORCHESTRA

**Rust‑Compose** : un langage déclaratif (`.drc`) compilé en `Dockerfile`, `docker-compose.yml` et
manifests Kubernetes, et le site qui le présente, écrit en Rust avec
[Paquets](https://github.com/maxlestage/Paquets).

```text
web { image = "nginx:1.30" }
```

Une ligne : le port (80 pour nginx), le service Compose, le Deployment et le Service
Kubernetes en sont déduits. Ports au choix : `port = 3000`, `port = "8080:80"` (publié sur 8080, l'application
écoute sur 80) ou `ports = [80, "8443:443"]` ; deux applications ne peuvent pas publier le
même port. Des versions à part (`image = "postgres"`, `version = "18-alpine"`), choisies au
besoin à la compilation (`orchestra build app.drc --set API=2.2.0`), et d'autres conteneurs
dans le même pod (`with = { proxy = { image = "envoyproxy/envoy" } }`). Ajoutez
`build = "rust"` pour un Dockerfile, `host = "exemple.fr"`
pour un Ingress,
`replicas`, `env`, `volumes`, `depends_on`… Une faute de frappe ? Le compilateur la situe et
propose la correction :

```text
app.drc:2:5 : `web` : clé inconnue `imgae` (vouliez-vous dire `image` ?)
```

Un site web complet, en nommant simplement les technologies :

```text
web { rust nginx postgres redis }
```

Votre code peut être en Rust, Go, Node.js, Python, Java (Maven) ou PHP : `web { node nginx postgres }`.
Un nom seul prend la dernière version stable (Node.js 26, Go 1.27, Python 3.15, nginx 1.30, postgres 18,
redis 8…). nginx renvoie
vers l'application, qui reçoit `DATABASE_URL` et `REDIS_URL` ; la base garde ses données (volume,
StatefulSet Kubernetes) et son mot de passe (`password`, Secret Kubernetes). Sans application,
`data { postgres redis }` donne juste une base et un cache.

Une version ou une variante s'écrit sans guillemets : `postgres = 18-alpine`,
`nginx = 1.30-alpine`. Pour le code Rust, `rust = 1.99-slim-bookworm` ou `rust = 1.99-alpine` :
l'image d'exécution de la même famille (`debian:bookworm-slim`, `alpine:3.24`) est choisie pour
vous.

Les sections `docker`, `compose` et `k8s` de la spécification complète restent disponibles
pour tout le reste (voir `examples/complet.drc`).

```sh
$ orchestra build app.drc --out deploy
✓ deploy/docker-compose.yml
✓ deploy/deployment.yaml
✓ deploy/service.yaml
```

## Le guide

Le [guide](https://maxlestage.github.io/ORCHESTRA/guide) explique tout par l'exemple :
premiers pas (de l'installation au déploiement), vingt recettes (Rust, Node.js, Python, Go, Java, PHP…) avec les fichiers qu'elles
produisent, l'usage des fichiers générés (Compose, Kubernetes, intégration continue), les erreurs
fréquentes et leur correction, et une FAQ. Les recettes sont aussi dans `examples/recettes/` :

| Recette | Fichier |
|---|---|
| Un site statique | `01-site-statique.drc` |
| Une API en Rust | `02-api-rust.drc` |
| Une API et sa base PostgreSQL | `03-api-postgres.drc` |
| Un site complet (Rust, nginx, PostgreSQL, Redis) | `04-site-complet.drc` |
| Choisir les variantes (alpine…) | `05-variantes.drc` |
| Le code dans un sous-dossier | `06-code-dans-un-dossier.drc` |
| Une image déjà publiée, avec MariaDB | `07-image-publiee.drc` |
| Juste une base et un cache | `08-base-et-cache.drc` |
| Plusieurs services | `09-microservices.drc` |
| En production (domaine, instances, mot de passe) | `10-production.drc` |
| Les versions au même endroit | `11-versions-centralisees.drc` |
| Plusieurs conteneurs ensemble | `12-plusieurs-conteneurs.drc` |
| MongoDB et Valkey | `13-mongo-valkey.drc` |
| Compléter à la main | `14-completer.drc` |
| Node.js | `15-node.drc` |
| Python | `16-python.drc` |
| Go | `17-go.drc` |
| Java (Maven) | `18-java.drc` |
| PHP (Apache) | `19-php.drc` |
| Plusieurs langages dans un projet | `20-plusieurs-langages.drc` |

## Le site

- **Accueil** : une scène animée en 2D (SVG et CSS), la démarche en trois étapes, les cinq
  principes et un exemple compilé en direct à chaque affichage.
- **Guide** (`/guide`) : premiers pas, recettes, déploiement, erreurs, FAQ.
- **Langage** (`/langage`) : la spécification complète.
- **Bac à sable** (`/bac-a-sable`, `/bac-a-sable/k8s`…) : un éditeur dont le fichier est compilé
  par le serveur, ou dans le navigateur pendant la frappe quand `orchestra.wasm` est publié, avec
  l'erreur surlignée à la bonne ligne et à la bonne colonne.
- **API** (`POST /api/compile`) : `{"source": "…"}` → `{"files": [{"name", "content"}]}`, ou une
  erreur 422 avec sa position.

Conçu mobile d'abord : aucune page ne défile horizontalement, de 320 à 1440 px, en thème clair
comme en thème sombre (vérifié à chaque push par `ci/mobile.mjs`). Les animations s'arrêtent
avec `prefers-reduced-motion`. Les pages sont rendues par le serveur et leur
Content-Security-Policy n'autorise que leur propre code ; le seul script est celui du bac à sable,
qui charge le compilateur WebAssembly s'il est là.

## Démarrer

Rust 1.99 ou plus récent.

```sh
cargo run                                          # le site : http://localhost:3000
cargo run -- check examples/complet.drc            # vérifie un fichier
cargo run -- build examples/projet/main.drc --out deploy
cargo test                                         # le compilateur et le site
```

Installer l'outil seul :

```sh
cargo install --git https://github.com/maxlestage/ORCHESTRA --locked
```

En production : `docker build -t orchestra . && docker run -p 8080:8080 orchestra`
(`heroku.yml` déploie la même image sur Heroku).

## GitHub Pages

Poussez sur `master` : le site est publié sur **https://maxlestage.github.io/ORCHESTRA/**.
C'est tout.

Le même site, en local :

```sh
cargo wasm && cargo run -- export dist
```

## Organisation

| Chemin | Rôle |
|---|---|
| `src/drc/` | Le compilateur : lexeur, analyseur, évaluation (variables, fonctions, `+`, imports), applications (`app`), vérifications et suggestions, génération Docker / Compose / Kubernetes, YAML |
| `src/render/` | Le HTML des résultats (coloration, fichiers, erreurs), partagé par le serveur et le WebAssembly |
| `src/wasm.rs` | `orchestra.wasm` : le compilateur dans le navigateur |
| `src/site/` | Le site Paquets : pages, illustrations SVG, feuille de style, export statique |
| `src/main.rs` | Le binaire `orchestra` : le site, `orchestra build` et `orchestra check` |
| `examples/` | `simple.drc` (une ligne), `boutique.drc` (trois applications), `ports.drc` (ports au choix), `versions.drc` (versions et `with`), `web.drc` (une pile web), `complet.drc` (les trois sections) et `projet/` (`import`) |
| `ci/mobile.mjs` | Le contrôle « aucun défilement horizontal » |

## Intégration continue

Chaque push est testé (format, clippy, tests, contrôle mobile). Quand tout est vert, la branche
est **fusionnée automatiquement dans `master`** (sauf les branches nommées `wip/...`), puis le
site est déployé sur GitHub Pages.
