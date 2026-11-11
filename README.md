# ORCHESTRA

**Rust‑Compose** : un langage déclaratif (`.drc`) compilé en `Dockerfile`, `docker-compose.yml` et
manifests Kubernetes, et le site qui le présente, écrit en Rust avec
[Paquets](https://github.com/maxlestage/Paquets).

```text
app web {
    image = "nginx:1.29"
    port = 80
}
```

Quatre lignes : le service Compose, le Deployment et le Service Kubernetes en sont déduits.
Ajoutez `build = "rust"` pour un Dockerfile, `host = "exemple.fr"` pour un Ingress,
`replicas`, `env`, `volumes`, `depends_on`… Une faute de frappe ? Le compilateur la situe et
propose la correction :

```text
app.drc:2:5 : `app web` : clé inconnue `imgae` (vouliez-vous dire `image` ?)
```

Les sections `docker`, `compose` et `k8s` de la spécification complète restent disponibles
pour tout le reste (voir `examples/complet.drc`).

```sh
$ orchestra build app.drc --out deploy
✓ deploy/docker-compose.yml
✓ deploy/deployment.yaml
✓ deploy/service.yaml
```

## Le site

- **Accueil** : une scène animée en 2D (SVG et CSS), la démarche en trois étapes, les cinq
  principes et un exemple compilé en direct à chaque affichage.
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
| `examples/` | `simple.drc` (quatre lignes), `boutique.drc` (trois applications), `complet.drc` (les trois sections) et `projet/` (`import`) |
| `ci/mobile.mjs` | Le contrôle « aucun défilement horizontal » |

## Intégration continue

Chaque push est testé (format, clippy, tests, contrôle mobile). Quand tout est vert, la branche
est **fusionnée automatiquement dans `master`** (sauf les branches nommées `wip/...`), puis le
site est déployé sur GitHub Pages.
