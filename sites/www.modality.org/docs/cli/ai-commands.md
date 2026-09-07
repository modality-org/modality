---
sidebar_position: 3
title: AI Commands
---

# AI Commands (`modal ai`)

Configure the LLM used by `modal ai suggest-rule`. Config is stored at
`$MODALITY_HOME/.modality/ai.json` (or `~/.modality/ai.json`).

## Set Provider

```bash
modal ai set --provider openai|anthropic|grok|bedrock|ollama|cursor-agent [--model] [--base-url] [--api-key] [--region] [--save-key]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--provider <PROVIDER>` | `openai`, `anthropic`, `grok`, `bedrock`, `ollama`, or `cursor-agent` |
| `--model <MODEL>` | Model id (provider default if omitted) |
| `--base-url <BASE_URL>` | API base URL (openai, anthropic, grok, ollama) |
| `--region <REGION>` | AWS region (bedrock) |
| `--api-key <API_KEY>` | API key (openai, anthropic, grok) |
| `--save-key` | Persist the API key in `~/.modality/ai.json` (mode `0600`) |

```bash
modal ai set --provider openai
modal ai set --provider anthropic
modal ai set --provider grok
modal ai set --provider bedrock --region us-east-1
modal ai set --provider ollama
modal ai set --provider cursor-agent
```

`--save-key` is only for openai, anthropic, and grok. Bedrock uses the AWS
credential chain (`AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` /
`AWS_SESSION_TOKEN`, a shared credentials file, or an instance role). Do not
persist AWS secrets in `ai.json`. Ollama needs no API key by default.
`cursor-agent` uses the Cursor CLI (`agent` or `cursor-agent` on `PATH`):
run `agent login`, or set `CURSOR_API_KEY`. Override the binary with
`MODAL_AI_CURSOR_AGENT`.

Defaults:

| Provider | Base URL | Model |
|----------|----------|-------|
| openai | `https://api.openai.com` | `gpt-5.6-luna` |
| anthropic | `https://api.anthropic.com` | `claude-sonnet-4-20250514` |
| grok | `https://api.x.ai` | `grok-3` |
| bedrock | region `us-east-1` | `anthropic.claude-sonnet-4-20250514-v1:0` |
| ollama | `http://127.0.0.1:11434` | `llama3.2` |
| cursor-agent | Cursor CLI in the contract directory | account default (`auto`) |

API key resolution, first match wins:

1. `--api-key` on `set` / `suggest-rule`
2. `MODAL_AI_API_KEY`
3. Provider env: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `XAI_API_KEY`, `CURSOR_API_KEY`
4. Optional API key saved in `ai.json` only with `--save-key`

## Show

```bash
modal ai show
```

Prints the configured provider, model, and a redacted API key. If nothing is
configured, run `modal ai set --provider openai|anthropic|grok|bedrock|ollama|cursor-agent`.

## Unset

```bash
modal ai unset
```

Removes `~/.modality/ai.json`.

## Suggest a Rule

```bash
modal ai suggest-rule <PROMPT>
```

Calls the configured provider and prints one Modality formula for
`modal add-rule`. If no provider is configured, the command fails with a
hint to run `modal ai set`.

With `--provider cursor-agent`, `suggest-rule` runs the Cursor CLI in the
contract directory (`--dir` or the current directory). The default is
`--print`: a one-shot formula on stdout. Pass `--interactive` for an Ask-mode
session so the agent can read `state/`, `rules/`, and `model/`.

```bash
modal ai suggest-rule "after this commit either alice or bob must sign"
modal ai suggest-rule --interactive "after this commit either alice or bob must sign"
```

```
[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
```

That printed formula is example output; yours may differ.
