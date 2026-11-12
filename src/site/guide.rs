//! Le guide : premiers pas, recettes complètes, déploiement, erreurs et questions
//! fréquentes. Chaque exemple est compilé à l'affichage : ce que montre la page est
//! exactement ce que produit le compilateur.

use orchestra::drc;
use orchestra::render;
use paquets::prelude::*;

use super::{code_block, layout, shell_block};

/// Une recette : un besoin, son fichier `.drc` et ce qu'il faut en savoir.
pub struct Recipe {
    /// Son ancre, et son adresse dans le bac à sable (`/bac-a-sable/<id>`).
    pub id: &'static str,
    pub title: &'static str,
    pub need: &'static str,
    pub file: &'static str,
    pub source: &'static str,
    /// Ce que le compilateur en fait, et ce qu'il faut savoir.
    pub notes: &'static [&'static str],
    /// Les commandes pour s'en servir.
    pub run: &'static str,
}

macro_rules! recipe {
    ($file:literal) => {
        concat!("examples/recettes/", $file)
    };
}

pub const RECIPES: &[Recipe] = &[
    Recipe {
        id: "site-statique",
        title: "Un site statique",
        need: "Servir des pages HTML déjà faites (un dossier public/), sans code.",
        file: recipe!("01-site-statique.drc"),
        source: include_str!("../../examples/recettes/01-site-statique.drc"),
        notes: &[
            "nginx écoute sur le port 80 : le compilateur le sait, inutile de l'écrire.",
            "<code>:ro</code> monte le dossier en lecture seule.",
            "Le volume ne sert qu'à Compose ; pour Kubernetes, construisez une image qui contient vos pages.",
        ],
        run: "$ orchestra build app.drc --out deploy\n$ docker compose -f deploy/docker-compose.yml up\n# http://localhost",
    },
    Recipe {
        id: "api-rust",
        title: "Une API en Rust",
        need: "Construire et lancer votre code Rust, seul.",
        file: recipe!("02-api-rust.drc"),
        source: include_str!("../../examples/recettes/02-api-rust.drc"),
        notes: &[
            "<code>rust</code> seul : le code est dans le dossier du fichier <code>.drc</code>, construit avec <code>rust:1.99-slim</code>, lancé sur <code>debian:trixie-slim</code>.",
            "Le binaire est trouvé quel que soit son nom (<code>cargo install</code>) ; le projet ne doit en produire qu'un.",
            "Le port est donné à votre programme dans la variable <code>PORT</code> : lisez-la pour écouter au bon endroit.",
        ],
        run: "$ orchestra build app.drc --out deploy\n$ docker compose -f deploy/docker-compose.yml up --build\n# http://localhost:3000",
    },
    Recipe {
        id: "api-postgres",
        title: "Une API et sa base PostgreSQL",
        need: "Votre code Rust, avec une base de données prête et reliée.",
        file: recipe!("03-api-postgres.drc"),
        source: include_str!("../../examples/recettes/03-api-postgres.drc"),
        notes: &[
            "L'API reçoit <code>DATABASE_URL=postgres://postgres:change-me@api-postgres:5432/api</code>.",
            "La base garde ses données : volume Compose <code>api-postgres</code>, StatefulSet et volume persistant de 1 Gio dans Kubernetes.",
            "Le mot de passe (<code>change-me</code> par défaut) va dans un Secret Kubernetes : changez-le avec <code>password</code> (voir « En production »).",
        ],
        run: "$ orchestra build app.drc --out deploy\n$ docker compose -f deploy/docker-compose.yml up --build",
    },
    Recipe {
        id: "site-complet",
        title: "Un site complet",
        need: "Votre code Rust derrière nginx, avec une base et un cache.",
        file: recipe!("04-site-complet.drc"),
        source: include_str!("../../examples/recettes/04-site-complet.drc"),
        notes: &[
            "nginx est le seul publié (port 80) ; il renvoie tout vers l'application (port 8080). La base et le cache restent internes.",
            "L'application reçoit <code>PORT</code>, <code>DATABASE_URL</code> et <code>REDIS_URL</code>.",
            "Les versions sont les dernières stables : nginx 1.30, postgres 18, redis 8. Elles sont écrites dans les fichiers générés.",
        ],
        run: "$ orchestra build app.drc --out deploy\n$ docker compose -f deploy/docker-compose.yml up --build\n# http://localhost",
    },
    Recipe {
        id: "variantes",
        title: "Choisir les variantes (alpine…)",
        need: "Les mêmes services, en images plus légères ou en versions précises.",
        file: recipe!("05-variantes.drc"),
        source: include_str!("../../examples/recettes/05-variantes.drc"),
        notes: &[
            "La valeur est le tag de l'image, écrit sans guillemets : <code>18-alpine</code> donne <code>postgres:18-alpine</code>.",
            "Pour Rust, l'image d'exécution suit : <code>1.99-alpine</code> tourne sur <code>alpine:3.24</code>, <code>1.99-slim-bookworm</code> sur <code>debian:bookworm-slim</code>.",
            "Une image Rust alpine produit un binaire très petit (quelques Mo).",
        ],
        run: "$ orchestra build app.drc --out deploy",
    },
    Recipe {
        id: "code-dans-un-dossier",
        title: "Le code dans un sous-dossier",
        need: "Votre projet Rust n'est pas à la racine, mais dans ./api.",
        file: recipe!("06-code-dans-un-dossier.drc"),
        source: include_str!("../../examples/recettes/06-code-dans-un-dossier.drc"),
        notes: &[
            "<code>src</code> donne le dossier du code quand <code>rust</code> donne une variante.",
            "Sans variante, écrivez directement le dossier : <code>rust = \"./api\"</code>.",
            "Un nom seul (<code>postgres</code>) peut se mêler aux réglages, sur sa propre ligne.",
        ],
        run: "$ orchestra build app.drc --out deploy",
    },
    Recipe {
        id: "image-publiee",
        title: "Une image déjà publiée",
        need: "Déployer une application construite ailleurs, avec sa base MariaDB.",
        file: recipe!("07-image-publiee.drc"),
        source: include_str!("../../examples/recettes/07-image-publiee.drc"),
        notes: &[
            "<code>image</code> et <code>version</code> donnent <code>ghcr.io/acme/shop:3.2</code>.",
            "Sans nginx, l'application est publiée elle-même, sur son port.",
            "MariaDB donne <code>DATABASE_URL=mysql://root:…@shop-mariadb:3306/shop</code> ; les caractères spéciaux du mot de passe y sont encodés.",
        ],
        run: "$ orchestra build app.drc --out deploy\n$ kubectl apply -f deploy/",
    },
    Recipe {
        id: "base-et-cache",
        title: "Juste une base et un cache",
        need: "Pour développer en local : PostgreSQL et Redis, sans application.",
        file: recipe!("08-base-et-cache.drc"),
        source: include_str!("../../examples/recettes/08-base-et-cache.drc"),
        notes: &[
            "Sans application, rien n'est publié sur la machine : la base et le cache restent dans le réseau Compose.",
            "Pour les joindre depuis votre machine, ajoutez une application simple avec un port, par exemple <code>db { image = \"postgres\", version = 18 }</code>, publiée sur 5432.",
        ],
        run: "$ orchestra build app.drc --out deploy\n$ docker compose -f deploy/docker-compose.yml up -d",
    },
    Recipe {
        id: "microservices",
        title: "Plusieurs services",
        need: "Un front, une API, un worker et une base, chacun avec ses réglages.",
        file: recipe!("09-microservices.drc"),
        source: include_str!("../../examples/recettes/09-microservices.drc"),
        notes: &[
            "Une variable et une fonction d'une ligne évitent de répéter le registre et la version.",
            "<code>depends_on</code> fixe l'ordre de démarrage dans Compose ; une dépendance inconnue est une erreur.",
            "Les volumes nommés (<code>pgdata</code>) sont déclarés pour vous en haut du fichier Compose.",
            "Deux applications ne peuvent pas publier le même port : le compilateur le refuse et en propose un libre.",
        ],
        run: "$ orchestra build app.drc --out deploy\n$ docker compose -f deploy/docker-compose.yml up -d\n$ kubectl apply -f deploy/",
    },
    Recipe {
        id: "production",
        title: "En production",
        need: "Un nom de domaine, plusieurs instances, un vrai mot de passe et une image publiée.",
        file: recipe!("10-production.drc"),
        source: include_str!("../../examples/recettes/10-production.drc"),
        notes: &[
            "<code>host</code> crée un Ingress Kubernetes vers nginx.",
            "<code>image</code> et <code>version</code> nomment l'image construite : poussez-la dans le registre avant <code>kubectl apply</code>.",
            "Le mot de passe vient d'une variable : passez le vrai avec <code>--set PASSWORD=…</code>, il ne s'écrit jamais dans le fichier.",
            "<code>replicas = 3</code> lance trois instances de l'application ; la base reste unique.",
        ],
        run: "$ orchestra build app.drc --out deploy --set PASSWORD=\"$DB_PASSWORD\"\n$ docker compose -f deploy/docker-compose.yml build web\n$ docker push ghcr.io/acme/web:1.0.0\n$ kubectl apply -f deploy/",
    },
    Recipe {
        id: "versions-centralisees",
        title: "Les versions au même endroit",
        need: "Garder toutes les versions en haut du fichier, et les choisir au déploiement.",
        file: recipe!("11-versions-centralisees.drc"),
        source: include_str!("../../examples/recettes/11-versions-centralisees.drc"),
        notes: &[
            "<code>--set API=2.2.0</code> remplace la variable sans modifier le fichier.",
            "La valeur garde le type de la variable : <code>--set REPLICAS=5</code> reste un nombre, <code>--set API=1.10</code> reste le texte « 1.10 ».",
            "Un nom inconnu est une erreur, avec la correction proposée.",
        ],
        run: "$ orchestra build app.drc --out deploy --set API=2.2.0",
    },
    Recipe {
        id: "plusieurs-conteneurs",
        title: "Plusieurs conteneurs ensemble",
        need: "Un proxy et un collecteur de journaux à côté de l'API, qui partagent son réseau.",
        file: recipe!("12-plusieurs-conteneurs.drc"),
        source: include_str!("../../examples/recettes/12-plusieurs-conteneurs.drc"),
        notes: &[
            "Dans Kubernetes : un seul pod, trois conteneurs, qui se parlent par <code>localhost</code>.",
            "Dans Compose : <code>api-proxy</code> et <code>api-logs</code> partagent le réseau de <code>api</code> (<code>network_mode: service:api</code>) ; leurs ports sont publiés par <code>api</code>.",
            "Deux conteneurs du même pod ne peuvent pas écouter sur le même port.",
        ],
        run: "$ orchestra build app.drc --out deploy",
    },
    Recipe {
        id: "mongo-valkey",
        title: "MongoDB et Valkey",
        need: "D'autres bases : un document store et un cache compatible Redis.",
        file: recipe!("13-mongo-valkey.drc"),
        source: include_str!("../../examples/recettes/13-mongo-valkey.drc"),
        notes: &[
            "L'application reçoit <code>MONGODB_URL</code> et <code>REDIS_URL</code>.",
            "Une seule base par rôle : <code>postgres</code> et <code>mysql</code> dans la même pile donneraient deux <code>DATABASE_URL</code> (voir les erreurs).",
        ],
        run: "$ orchestra build app.drc --out deploy",
    },
    Recipe {
        id: "completer",
        title: "Compléter à la main",
        need: "Ajouter un réglage que le langage ne prévoit pas.",
        file: recipe!("14-completer.drc"),
        source: include_str!("../../examples/recettes/14-completer.drc"),
        notes: &[
            "Une section <code>compose</code> fusionne avec ce que les applications génèrent : ici, <code>restart</code> s'ajoute au service <code>web</code>.",
            "Une section <code>k8s</code> ajoute des ressources : la ConfigMap s'écrit dans <code>configmap.yaml</code>.",
            "Toute la syntaxe de ces sections est dans la spécification.",
        ],
        run: "$ orchestra build app.drc --out deploy",
    },
];

/// Une erreur fréquente : le fichier fautif, l'explication, le fichier corrigé.
struct Mistake {
    title: &'static str,
    wrong: &'static str,
    why: &'static str,
    right: &'static str,
}

const MISTAKES: &[Mistake] = &[
    Mistake {
        title: "Une faute de frappe dans une clé",
        wrong: "web {\n    imgae = \"nginx:1.30\"\n}",
        why: "Le compilateur connaît toutes les clés : il propose la plus proche.",
        right: "web {\n    image = \"nginx:1.30\"\n}",
    },
    Mistake {
        title: "Un mot-clé mal écrit",
        wrong: "dokcer { base = \"rust:1.99-slim\" }",
        why: "À une lettre de <code>docker</code> : ce n'est pas pris pour une application nommée « dokcer ».",
        right: "docker { base = \"rust:1.99-slim\" }",
    },
    Mistake {
        title: "Deux applications sur le même port",
        wrong: "a { image = \"nginx:1.30\" }\nb { image = \"nginx:1.30\" }",
        why: "Les deux nginx veulent le port 80 de la machine. Le compilateur propose un port libre.",
        right: "a { image = \"nginx:1.30\" }\nb { image = \"nginx:1.30\", port = 81 }",
    },
    Mistake {
        title: "Une version écrite deux fois",
        wrong: "db { image = \"postgres:17\", version = 18 }",
        why: "L'image a déjà une version : écrivez-la à un seul endroit.",
        right: "db { image = \"postgres\", version = 18 }",
    },
    Mistake {
        title: "Deux bases pour la même adresse",
        wrong: "web { rust postgres mysql }",
        why: "Les deux donneraient <code>DATABASE_URL</code> à l'application. Gardez-en une, ou mettez l'autre dans un bloc à part.",
        right: "web { rust postgres }\nancien { mysql }",
    },
    Mistake {
        title: "Une variable mal écrite",
        wrong: "VERSION = \"2.1.0\"\napi { image = \"ghcr.io/acme/api\", version = VERISON }",
        why: "Une variable doit être déclarée ; la plus proche est proposée.",
        right: "VERSION = \"2.1.0\"\napi { image = \"ghcr.io/acme/api\", version = VERSION }",
    },
    Mistake {
        title: "Un réglage sans application",
        wrong: "web { postgres, replicas = 3 }",
        why: "<code>replicas</code> règle l'application ; ici, il n'y en a pas.",
        right: "web { rust postgres, replicas = 3 }",
    },
    Mistake {
        title: "Un domaine sans port",
        wrong: "web { image = \"ghcr.io/acme/web:1\", host = \"monsite.fr\" }",
        why: "Le port de cette image n'est pas connu : il faut le dire pour la publier.",
        right: "web { image = \"ghcr.io/acme/web:1\", port = 8080, host = \"monsite.fr\" }",
    },
    Mistake {
        title: "Une version incomplète",
        wrong: "web { nginx = 1.30- }",
        why: "Une version ne se termine pas par <code>-</code> ou <code>.</code>.",
        right: "web { nginx = 1.30-alpine }",
    },
];

/// Les questions fréquentes : (question, réponse en HTML).
const FAQ: &[(&str, &str)] = &[
    (
        "Quelle version est prise quand je n'en écris pas ?",
        "La dernière stable, choisie par le compilateur : nginx 1.30, postgres 18, mysql 26, mariadb 13, mongo 9, redis 8, valkey 9, Rust 1.99. Elle est écrite dans les fichiers générés : vous savez toujours ce qui tourne. Pour la figer, écrivez-la : <code>postgres = 18</code>.",
    ),
    (
        "Comment choisir alpine, bookworm ou une autre variante ?",
        "Écrivez le tag complet, sans guillemets : <code>postgres = 18-alpine</code>, <code>rust = 1.99-slim-bookworm</code>. Pour Rust, l'image d'exécution de la même famille est choisie pour vous.",
    ),
    (
        "Où sont mes données ?",
        "Dans Compose, dans un volume nommé d'après le service (<code>web-postgres</code>) : <code>docker compose down</code> les garde, <code>docker compose down -v</code> les efface. Dans Kubernetes, dans un volume persistant de 1 Gio créé par le StatefulSet.",
    ),
    (
        "Comment changer le mot de passe de la base ?",
        "Avec <code>password</code> dans le bloc. Pour ne pas l'écrire dans le fichier, passez par une variable : <code>password = PASSWORD</code>, puis <code>orchestra build app.drc --set PASSWORD=…</code>. Dans Kubernetes, il va dans un Secret.",
    ),
    (
        "Mon programme doit-il lire des variables ?",
        "Oui : <code>PORT</code> (le port où écouter), et selon la pile <code>DATABASE_URL</code>, <code>REDIS_URL</code> ou <code>MONGODB_URL</code>. Ajoutez les vôtres avec <code>env = { CLE = \"valeur\" }</code>.",
    ),
    (
        "Puis-je modifier les fichiers générés ?",
        "Non : ils sont réécrits à chaque compilation. Pour un réglage absent du langage, ajoutez une section <code>compose</code> ou <code>k8s</code> dans le fichier <code>.drc</code> (recette « Compléter à la main »).",
    ),
    (
        "Où écrire les fichiers générés ?",
        "Où vous voulez avec <code>--out</code>. Le fichier Compose est ajusté pour que la construction parte toujours du dossier du fichier <code>.drc</code>, là où est votre code.",
    ),
    (
        "Et pour Kubernetes, d'où vient l'image de mon code ?",
        "D'un registre : nommez-la avec <code>image = \"ghcr.io/vous/web\"</code> et <code>version</code>, construisez-la (<code>docker compose build</code>), poussez-la (<code>docker push</code>), puis appliquez les manifests. Compose, lui, la construit sur place.",
    ),
    (
        "Puis-je avoir plusieurs piles dans le même fichier ?",
        "Oui, autant de blocs que vous voulez : chaque pile garde ses propres services (<code>web-postgres</code>, <code>admin-mariadb</code>…). Deux blocs ne peuvent pas publier le même port : un seul nginx par fichier, sur le port 80.",
    ),
    (
        "Comment découper un projet en plusieurs fichiers ?",
        "Avec <code>import \"config.drc\"</code> : variables, fonctions et blocs du fichier importé sont repris. Voir <code>examples/projet/</code> et la spécification.",
    ),
    (
        "Comment essayer sans rien installer ?",
        "Dans le bac à sable : chaque recette de ce guide s'y ouvre, et se compile dans votre navigateur pendant que vous tapez.",
    ),
];

pub async fn page(_req: Request) -> Page {
    let toc = details()
        .class("toc")
        .attr("open", "")
        .child(summary().text("Sommaire"))
        .child(
            ol().child(li().child(a().href("#premiers-pas").text("Premiers pas")))
                .child(
                    li().child(a().href("#recettes").text("Recettes")).child(
                        ol().children(
                            RECIPES
                                .iter()
                                .map(|r| li().child(a().href(format!("#{}", r.id)).text(r.title))),
                        ),
                    ),
                )
                .child(li().child(a().href("#deployer").text("Utiliser les fichiers")))
                .child(li().child(a().href("#erreurs").text("Lire une erreur")))
                .child(li().child(a().href("#faq").text("Questions fréquentes"))),
        );
    let article = article()
        .class("prose")
        .child(span().class("eyebrow").text("Guide"))
        .child(h1().text("Rust‑Compose par l'exemple"))
        .child(p().class("lead").html(
            "Des besoins concrets, le fichier <code>.drc</code> qui y répond, et les fichiers qu'il produit, générés à l'instant par le compilateur. Chaque exemple s'ouvre dans le bac à sable.",
        ))
        .children(first_steps())
        .child(h2().id("recettes").text("Recettes"))
        .children(RECIPES.iter().map(recipe))
        .children(deploy())
        .children(mistakes())
        .children(faq());
    layout(
        "Guide",
        "Le guide de Rust‑Compose : premiers pas, quatorze recettes complètes (site, API, bases, production, versions…), déploiement, erreurs fréquentes et questions.",
        "/guide",
        div()
            .class("wrap doc-layout")
            .child(div().child(toc))
            .child(article),
    )
}

fn drc_block(name: &str, source: &str) -> Node {
    div()
        .child(div().class("file-tag").html(format!(
            "<span class=\"dot\"></span>{}",
            render::highlight::escape(name)
        )))
        .child(code_block(name, source))
        .into()
}

fn first_steps() -> Vec<Node> {
    let step = |title: &str, text: &str, code: Node| -> Node {
        li().child(h3().text(title.to_owned()))
            .child(p().html(text.to_owned()))
            .child(code)
            .into()
    };
    let files = drc::compile_str(include_str!("../../examples/recettes/04-site-complet.drc"))
        .unwrap_or_default();
    let written: String = files
        .iter()
        .map(|f| format!("✓ deploy/{}\n", f.name))
        .collect();
    vec![
        h2().id("premiers-pas").text("Premiers pas").into(),
        p().html("Six étapes, de rien à un site qui tourne. Il faut Rust 1.99 ou plus récent, et Docker pour lancer en local.").into(),
        ol().class("tutorial")
            .child(step(
                "Installer l'outil",
                "Une commande, puis vérifiez la version.",
                shell_block("$ cargo install --git https://github.com/maxlestage/ORCHESTRA --locked\n$ orchestra --version").into(),
            ))
            .child(step(
                "Écrire app.drc",
                "À la racine de votre projet Rust, à côté de <code>Cargo.toml</code> :",
                drc_block("app.drc", "web { rust nginx postgres redis }"),
            ))
            .child(step(
                "Vérifier",
                "Sans rien écrire : les fautes sont signalées avec leur ligne, leur colonne et leur correction.",
                shell_block(&format!("$ orchestra check app.drc\n✓ app.drc : {} fichier(s) à générer", files.len())).into(),
            ))
            .child(step(
                "Générer",
                "Les fichiers vont dans <code>deploy/</code> (ou où vous voulez).",
                shell_block(&format!("$ orchestra build app.drc --out deploy\n{written}")).into(),
            ))
            .child(step(
                "Lancer en local",
                "Compose construit votre code et démarre tout ; le site répond sur <a href=\"http://localhost\">http://localhost</a>.",
                shell_block("$ docker compose -f deploy/docker-compose.yml up --build").into(),
            ))
            .child(step(
                "Déployer sur Kubernetes",
                "Nommez l'image de votre code dans un registre (recette « En production »), poussez-la, puis :",
                shell_block("$ kubectl apply -f deploy/\n$ kubectl get pods").into(),
            ))
            .into(),
    ]
}

fn recipe(r: &Recipe) -> Node {
    let files = drc::compile_str(r.source).unwrap_or_default();
    let notes = ul().children(r.notes.iter().map(|n| li().html(*n)));
    article()
        .class("recipe")
        .id(r.id)
        .child(h3().text(r.title))
        .child(
            p().class("need")
                .html(format!("<strong>Le besoin.</strong> {}", r.need)),
        )
        .child(drc_block(r.file, r.source))
        .child(notes)
        .child(p().class("muted").html(format!(
            "Les {} fichiers générés, à l'instant :",
            files.len()
        )))
        .child(div().html(render::outputs_with(&files, false)))
        .child(shell_block(r.run))
        .child(
            div().class("actions").child(
                a().class("btn")
                    .href(format!("/bac-a-sable/{}", r.id))
                    .text("Ouvrir dans le bac à sable"),
            ),
        )
        .into()
}

fn deploy() -> Vec<Node> {
    vec![
        h2().id("deployer").text("Utiliser les fichiers générés").into(),
        h3().text("En local, avec Docker Compose").into(),
        shell_block("$ orchestra build app.drc --out deploy\n$ docker compose -f deploy/docker-compose.yml up --build   # construit et démarre\n$ docker compose -f deploy/docker-compose.yml logs -f     # suivre les journaux\n$ docker compose -f deploy/docker-compose.yml down        # arrêter (les données restent)\n$ docker compose -f deploy/docker-compose.yml down -v     # arrêter et effacer les données").into(),
        h3().text("Sur Kubernetes").into(),
        shell_block("$ orchestra build app.drc --out deploy --set PASSWORD=\"$DB_PASSWORD\"\n$ docker compose -f deploy/docker-compose.yml build      # l'image de votre code\n$ docker push ghcr.io/vous/web:1.0.0                       # dans votre registre\n$ kubectl apply -f deploy/\n$ kubectl get pods,services,ingress").into(),
        h3().text("En intégration continue").into(),
        p().html("Compilez dans le pipeline, avec la version qui vient d'être construite :").into(),
        shell_block("$ orchestra check app.drc\n$ orchestra build app.drc --out deploy --set VERSION=\"$GITHUB_SHA\"").into(),
    ]
}

fn mistakes() -> Vec<Node> {
    let mut nodes: Vec<Node> = vec![
        h2().id("erreurs").text("Lire une erreur").into(),
        p().html("Le compilateur refuse un fichier incorrect avant d'écrire quoi que ce soit, en donnant la ligne, la colonne et, quand il le peut, la correction. Les messages ci-dessous sont produits à l'instant par le compilateur.").into(),
    ];
    for m in MISTAKES {
        let message = match drc::compile_str(m.wrong) {
            Err(e) => format!(
                "Ligne {}, colonne {} : {}",
                e.pos.line,
                e.pos.col,
                render::highlight::escape(&e.message)
            ),
            Ok(_) => "(aucune erreur)".to_owned(),
        };
        nodes.push(
            article()
                .class("mistake")
                .child(h3().text(m.title))
                .child(code_block("erreur.drc", m.wrong))
                .child(
                    div()
                        .class("status error")
                        .html(format!("<span>{message}</span>")),
                )
                .child(p().html(m.why))
                .child(p().class("fix").html("<strong>Corrigé :</strong>"))
                .child(code_block("corrige.drc", m.right))
                .into(),
        );
    }
    nodes
}

fn faq() -> Vec<Node> {
    let mut nodes: Vec<Node> = vec![h2().id("faq").text("Questions fréquentes").into()];
    for (question, answer) in FAQ {
        nodes.push(
            details()
                .class("faq")
                .child(summary().text(*question))
                .child(p().html(*answer))
                .into(),
        );
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_recipe_compiles_and_every_mistake_is_caught() {
        for r in RECIPES {
            if let Err(e) = drc::compile_str(r.source) {
                panic!("{} : {e}", r.id);
            }
        }
        for m in MISTAKES {
            assert!(
                drc::compile_str(m.wrong).is_err(),
                "{} : pas d'erreur",
                m.title
            );
            if let Err(e) = drc::compile_str(m.right) {
                panic!("{} (corrigé) : {e}", m.title);
            }
        }
    }

    #[test]
    fn recipe_ids_do_not_clash_with_sandbox_examples() {
        for r in RECIPES {
            assert!(
                !super::super::examples::ALL.iter().any(|e| e.id == r.id),
                "{}",
                r.id
            );
        }
    }
}
