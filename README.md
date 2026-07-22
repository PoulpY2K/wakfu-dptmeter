<div id="top"></div>

<div style="text-align:center">

<a href="https://github.com/PoulpY2K/wakfu-dptmeter"><h1 style="text-align:center">Wakfu DPT Meter</h1></a>

[![MIT License](https://img.shields.io/badge/License-MIT-green.svg)](https://choosealicense.com/licenses/mit/)
![Build status](https://img.shields.io/github/actions/workflow/status/PoulpY2K/wakfu-dptmeter/tauri-build-analyze.yml)
![Gate](https://img.shields.io/sonar/quality_gate/PoulpY2K_wakfu-dptmeter?server=https%3A%2F%2Fsonarcloud.io
)
![Coverage](https://img.shields.io/sonar/coverage/PoulpY2K_wakfu-dptmeter?server=https%3A%2F%2Fsonarcloud.io
)
![Downloads](https://img.shields.io/github/downloads/PoulpY2K/wakfu-dptmeter/total
)
![Release](https://img.shields.io/github/v/release/PoulpY2K/wakfu-dptmeter?display_name=tag
)
![Stars](https://img.shields.io/github/stars/PoulpY2K/wakfu-dptmeter?style=flat
)


<p style="text-align:center">DPT Meter est un outil pour <a href="https://www.wakfu.com/fr/mmorpg">Wakfu</a>, jeu développé et détenu par Ankama Games. 
Le but de l'outil est d'afficher le nombre de dégâts par tour fait par les personnages au fur et à mesure de chaque combat, tout en apportant des statistiques et une interface moderne.</p></div>

## 📄 À propos du projet

L'outil a uniquement accès au fichier de log principal du jeu Wakfu. On ne fait que de la lecture des nouvelles lignes
insérées, absolument aucune écriture.

- `%APPDATA%\zaap\gamesLogs\wakfu\logs\wakfu.log` (Windows)
- `~/Library/Logs/zaap/wakfu/logs/wakfu.log` (MacOS)

## 🧠 Propriété intellectuelle

_**Wakfu et tout le contenu qui l'entoure est la propriété d'Ankama Games.**_

Ankama Games possède donc l'entièreté des droits sur ce projet.
Ce projet est susceptible d'être retiré de GitHub selon le bon vouloir d'Ankama Games.

Ce projet n'est pas affilié à Ankama Games de quelque manière que ce soit.

## ❓ F.A.Q

#### Est-ce que l'outil sera officialisé ?

Si je trouve la motivation de faire un post forum et de contacter le staff Ankama, je pourrais probablement publier
l'outil. Cependant, ce n'est pas mon but actuel, je le ferais dans le cas où l'application a du succès.

## 🛠️ Stack technique

- Outil construit avec [Tauri](https://www.tauri.app/)
    - Basée sur [Rust](https://rust-lang.org/)
    - Frontend en [Angular](https://angular.dev/)
        - Avec [TypeScript](https://www.typescriptlang.org/)

## 🚀 Développement local

### Prérequis

- [Rust](https://rust-lang.org/) + [Cargo](https://doc.rust-lang.org/cargo/)
- [Bun](https://bun.sh/)
- Tauri (voir le [guide officiel](https://v2.tauri.app/start/prerequisites/))

### Installation

```bash
bun install
```

### Lancer l'application localement

```bash
bun run tauri dev
```

### Tests et lint (Rust)

```bash
cd src-tauri
cargo test
cargo clippy --all-targets --all-features
```

## 🧪 Contribuer

- Crée une branche depuis `main` (`feature/...`, `fix/...`)
- Respecte le format [Conventional Commits](https://www.conventionalcommits.org/) pour les messages (`feat:`, `fix:`, `refactor:`, `chore:`, ...)
- Signer les commits avec une clé GPG
- `cargo test` et `cargo clippy --all-targets --all-features` doivent passer sans erreur ni warning avant d'ouvrir une Pull Request
- Ouvre une Pull Request vers `main`

## 📜 Licence

Distribué sous la **Licence MIT** - voir `LICENSE.md` pour plus d'informations.

## 👤 Contact

**Jérémy Laurent** ([@poulpy2k](https://twitter.com/PoulpY2K))

- Website: [jeremy-laurent.com](https://jeremy-laurent.com)
- Email: contact@jeremy-laurent.fr

<p style="text-align:right">[<a href="#top">Revenir au début</a>]</p>