# Website

This website is built using [Docusaurus](https://docusaurus.io/), a modern static website generator.

Doc **content** lives in repo-root [`docs/`](../../docs/). This directory is the site chrome (sidebar, theme, blog). CI copies `docs/` into `docs/` here before every production build.

## Local preview (recommended)

From the repository root:

```bash
./scripts/run-site.sh
```

That copies `docs/` into this site, installs npm dependencies if needed, and starts Docusaurus at http://localhost:3000. Edit markdown in `docs/`, then re-run the script so the preview picks up the copy. Full workflow: [DEVELOPMENT.md](../../DEVELOPMENT.md#documentation-site).

## Installation

```bash
npm ci
```

## Local Development

Prefer `./scripts/run-site.sh` from the repo root so `docs/` is synced. To start Docusaurus only (no copy):

```bash
npm start
```

This command starts a local development server and opens up a browser window. Most changes under this directory are reflected live without having to restart the server.

## Build

```bash
npm run build
```

This command generates static content into the `build` directory and can be served using any static contents hosting service.

## Deployment

Using SSH:

```bash
USE_SSH=true yarn deploy
```

Not using SSH:

```bash
GIT_USER=<Your GitHub username> yarn deploy
```

If you are using GitHub pages for hosting, this command is a convenient way to build the website and push to the `gh-pages` branch.
