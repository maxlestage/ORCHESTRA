# ORCHESTRA

**Rust‑Compose** : un langage déclaratif (`.drc`) compilé en `Dockerfile`, `docker-compose.yml` et
manifests Kubernetes, et le site qui le présente, écrit en Rust avec
[Paquets](https://github.com/maxlestage/Paquets).

```text
var IMAGE_NAME = "myapp"

fn image_tag(tag) {
    return format!("{}:{}", IMAGE_NAME, tag)
}

docker  { base = "rust:1.99-slim", workdir = "/app", build = ["cargo build --release"], run = ["./myapp"] }
compose { services = { web = { image = image_tag("latest"), ports = ["8080:80"] } } }
k8s     { service = { name = "myapp", selector = { app = "myapp" }, ports = [{ port = 80 }] } }
```

```sh
$ orchestra build app.drc --out deploy
✓ deploy/Dockerfile
✓ deploy/docker-compose.yml
✓ deploy/service.yaml
```

## Le site

- **Accueil** : une scène animée en 2D (SVG et CSS), la démarche en trois étapes, les cinq
  principes et un exemple compilé en direct à chaque affichage.
- **Langage** (`/langage`) : la spécification complète.
- **Bac à sable** (`/bac-a-sable`) : un éditeur dont le fichier est compilé par le serveur, avec
  l'erreur surlignée à la bonne ligne et à la bonne colonne.
- **API** (`POST /api/compile`) : `{"source": "…"}` → `{"files": [{"name", "content"}]}`, ou une
  erreur 422 avec sa position.

Conçu mobile d'abord : aucune page ne défile horizontalement, de 320 à 1440 px, en thème clair
comme en thème sombre (vérifié à chaque push par `ci/mobile.mjs`). Les animations s'arrêtent
avec `prefers-reduced-motion`. Aucune ligne de JavaScript : les pages sont rendues par le
serveur et leur Content-Security-Policy n'autorise que leur propre code.

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

## Organisation

| Chemin | Rôle |
|---|---|
| `src/drc/` | Le compilateur : lexeur, analyseur, évaluation (variables, fonctions, imports), vérifications, génération Docker / Compose / Kubernetes, YAML |
| `src/site/` | Le site Paquets : pages, illustrations SVG, coloration syntaxique, feuille de style |
| `src/main.rs` | Le binaire `orchestra` : le site, `orchestra build` et `orchestra check` |
| `examples/` | `complet.drc` (les trois sections) et `projet/` (fichiers séparés et `import`) |
| `ci/mobile.mjs` | Le contrôle « aucun défilement horizontal » |

## Intégration continue

Chaque push est testé (format, clippy, tests, contrôle mobile). Quand tout est vert, la branche
est **fusionnée automatiquement dans `master`** (sauf les branches nommées `wip/...`).
