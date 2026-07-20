# Design — Moteur de détection des combats et identification des combattants

**Date** : 2026-07-20
**Statut** : Validé (en attente de plan d'implémentation)

## Contexte

Wakfu DPT Meter lit en continu le fichier de log du client Wakfu
(`%APPDATA%\zaap\gamesLogs\wakfu\logs\wakfu.log`) pour, à terme, calculer les
dégâts par tour (DPT) de chaque personnage lors d'un combat. Avant de pouvoir
calculer quoi que ce soit, il faut être capable de :

1. Détecter le début d'un combat.
2. Identifier les combattants qui y participent (joueurs/alliés vs ennemis).
3. Détecter la fin du combat.
4. Rattacher correctement chaque action de dégâts/soin à son véritable
   auteur, y compris quand cet auteur agit via une invocation.

## Périmètre

**Dans le périmètre** : le moteur de détection/identification côté backend
Rust — production d'un flux d'événements métier structurés (`FightEvent`)
représentant le cycle de vie d'un combat et l'attribution des actions de
dégâts/soin à leur source réelle.

**Hors périmètre** (fera l'objet d'une spec séparée) :
- Le calcul et l'agrégation du DPT à partir des `FightEvent`.
- L'affichage / l'UI Angular (aucune modification du frontend dans cette spec).
- La gestion d'un démarrage de l'application en cours de combat (limitation
  acceptée, voir Cas limites).
- La gestion de combats simultanés (hypothèse : un seul combat actif à la
  fois).

## Architecture

```
wakfu.log ──▶ [log_watcher] ──raw lines──▶ [log_parser] ──LogEvent──▶ [fight_tracker] ──FightEvent (JSON)──▶ Tauri emit ──▶ (frontend, hors périmètre)
```

Trois modules Rust aux responsabilités strictement séparées :

### `log_watcher` (I/O)
- Surveille `wakfu.log` via `notify` + `notify-debouncer-mini` (scaffolding
  déjà présent dans `src-tauri/src/lib.rs`).
- Lit uniquement les **nouvelles** lignes ajoutées (tail depuis la position
  courante, jamais de re-lecture depuis le début du fichier).
- Transmet chaque nouvelle ligne brute, une par une, au `log_parser`.
- Aucune connaissance du contenu métier des lignes.

### `log_parser` (pur, sans dépendance Tauri)
- Fonction centrale : `parse_line(&str) -> LogEvent`.
- Un matcher/regex statique par type de ligne reconnue. Toute ligne non
  reconnue produit `LogEvent::Unrecognized`.
- Aucun état conservé entre deux appels — testable unitairement avec de
  simples chaînes de caractères, y compris en rejouant des extraits de
  `resources/wakfu-one-fight.log`.

### `fight_tracker` (state machine)
- Consomme le flux de `LogEvent` et maintient l'état interne du combat en
  cours :
  - `fight_id: Option<u64>`
  - `participants: HashMap<EntityId, Combatant>` — uniquement joueurs et
    ennemis réels, **jamais** les invocations.
  - `summon_owner: HashMap<SummonName, EntityId>` — mapping invocation →
    invocateur.
  - `current_caster: Option<EntityId>` — dernier lanceur de sort connu, utilisé
    pour attribuer les variations de PV qui suivent.
- Produit des `FightEvent` de haut niveau, émis via `app_handle.emit(...)`.
- Se réinitialise entièrement à chaque `FightEnded`.

## Modèle de données

### `LogEvent` (interne à `log_parser`, non exposé au frontend)

```rust
enum LogEvent {
    FightCreationDetected,
    FighterJoined {
        fight_id: u64,
        name: String,
        entity_id: i64,
        is_controlled_by_ai: bool,
    },
    SummonInvoked {
        owner_name: String,
        summon_name: String,
    },
    SpellCast {
        actor_name: String,
    },
    HpChange {
        name: String,
        amount: i32,           // signé, négatif = dégâts, positif = soin
        element: Option<String>,
        is_parried: bool,
    },
    FightEnded {
        fight_id: u64,
    },
    Unrecognized,
}
```

### `FightEvent` (sortie de `fight_tracker`, émis au frontend via Tauri)

```rust
enum FightEvent {
    FightStarted { fight_id: u64 },
    CombatantIdentified {
        fight_id: u64,
        name: String,
        entity_id: i64,
        side: Side, // Player | Enemy
    },
    ActionRecorded {
        fight_id: u64,
        source: String,
        target: String,
        amount: i32,
        kind: ActionKind, // Damage | Heal
        element: Option<String>,
    },
    FightEnded { fight_id: u64 },
}
```

Les champs exacts (types Rust précis, dérivés `serde`) seront affinés lors de
l'implémentation, en s'appuyant sur les lignes réelles du fichier de
référence.

## Règles de détection

### Identification joueur vs ennemi

Basée uniquement sur la ligne `[_FL_] fightId=... <name> breed : ... [<entity_id>] isControlledByAI=<bool> ...` :
- `is_controlled_by_ai == false` → personnage joueur/allié.
- `is_controlled_by_ai == true` → ennemi **ou** invocation (voir ci-dessous).

Aucune autre source de vérité (ex: messages "a rejoint le groupe") n'est
utilisée : cette ligne `[_FL_]` est la seule source de vérité pour la liste
des combattants.

### Distinction ennemi / invocation

Un combattant qui rejoint via `FighterJoined` et dont le nom correspond à un
`summon_name` précédemment vu dans un `SummonInvoked` est :
- **exclu** de la liste des combattants (`participants`),
- ajouté à `summon_owner` à la place, avec pour valeur l'`entity_id` de son
  invocateur.

### Algorithme d'attribution des dégâts/soins

Le log n'indique jamais explicitement la relation source→cible sur la ligne
de variation de PV — la causalité est implicite et séquentielle :

1. Une ligne `X lance le sort Y` (`SpellCast { actor_name: X }`) définit
   `current_caster = résoudre_proprietaire(X)` (résout vers le propriétaire si
   `X` est une invocation connue de `summon_owner`, sinon `X` lui-même).
2. Toute `HpChange` qui suit est attribuée avec `source = current_caster` et
   `target = name` (le nom présent sur la ligne de PV elle-même), jusqu'à la
   prochaine ligne `SpellCast`.
3. Ce mécanisme couvre aussi les dégâts sur soi-même / de zone : la cible est
   simplement égale à la source dans ce cas.

## Cas limites

| Cas | Décision |
|---|---|
| `The fight with the id X has not been found` (avant création) | Ignorée — bruit |
| `Starting join procedure for <id>` sans `[_FL_] ... join the fight` correspondant | Ignorée — seule une ligne `[_FL_]` confirmée fait foi |
| Lignes de statut sans `PV` (`Fuyard (+1 Niv.)`, `3 PM`, `50 Tacle`...) | `Unrecognized` / ignorées dans cette spec |
| Invocation qui lance elle-même un sort | `current_caster` devient le propriétaire de l'invocation, pas l'invocation |
| Application lancée en cours de combat | Non géré — le combat en cours ne sera détecté qu'à partir de la prochaine `CREATION DU COMBAT` |
| Deux combats simultanés | Non géré (hypothèse : un seul combat actif à la fois) |
| Nombres avec séparateur de milliers français (`-1 757 PV`) | Les espaces internes sont retirés avant conversion en entier |

## Stratégie de tests

- **`log_parser`** : tests unitaires ligne-par-ligne, un cas par type de ligne
  reconnue + cas de non-reconnaissance, en réutilisant des extraits exacts de
  `resources/wakfu-one-fight.log`.
- **`fight_tracker`** : test d'intégration rejouant l'intégralité de
  `resources/wakfu-one-fight.log` et vérifiant la séquence de `FightEvent`
  produite : 1 combat détecté (fightId `1568151141`), 4 combattants
  identifiés (Soeur Zerker, Blampy, Distipy, Marylpy), dégâts des invocations
  de Blampy correctement rattachés à Blampy, fin de combat détectée.
- **`log_watcher`** : conserver et étendre le test existant (écriture d'un
  fichier temporaire + vérification de la détection de nouvelles lignes).

## Suite

Un plan d'implémentation détaillé sera produit via la skill `writing-plans` à
partir de ce document.

