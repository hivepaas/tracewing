# Capturing an app

A program's traffic comes through tracewing only when that program is told to
send it there, and programs listen in different places: a Mac or Windows app to
the system's proxy setting, a command-line tool to its environment, a browser
to either or to its own flags. Each place below takes a different set of
programs, and none of them takes all.

| The way in | What comes through | What does not |
|---|---|---|
| **The system proxy** — Set up, step 2 | Safari, Chrome and Edge as they are, Mac and Windows apps, desktop apps built on Electron (Claude, ChatGPT, Cursor, Slack, Discord) | Command-line tools; on macOS, `localhost` and `127.0.0.1` |
| **A browser on its own** — Set up › Browsers › Open on its own | Chrome, Edge, Brave, Opera, Chromium and Firefox, in a profile of tracewing's, servers on this computer included | Safari, which has no such profile |
| **A terminal** — Set up › Open a terminal, or `tracewing env` | What is started in it: curl, git, Python, Go, Deno, Bun, and AI agents such as Claude Code, Codex and Gemini CLI | Node.js before 22.21, and 23 to 24.4; Java |

The two last leave the system proxy and every other app alone, which is what
to use when only one program is being looked at.

## Browsers

**Open on its own** starts the browser in a profile kept for tracewing, pointed
at the proxy by its own flags, beside the profile the user works in. Nothing
else on the computer changes, and a server on this computer comes through too.

Safari has no profile to start that way: it comes through only by the system
proxy. Like every program on macOS's own networking, it then still sends
`localhost` and `127.0.0.1` straight to the server, whatever the proxy settings
say. A name under `.localhost` goes to this computer as well and does go
through the proxy: `http://playground.localhost:10000` instead of
`http://localhost:10000`.

## Command-line tools

**Open a terminal**, on Set up, opens one whose programs go through tracewing
and trust its certificate authority. For a terminal tracewing did not open — an
editor's, iTerm's, a script's — `tracewing env` says the same, for the shell to
run:

```sh
eval "$(tracewing env)"                    # sh, bash, zsh
tracewing env --shell fish | source        # fish
tracewing env | Invoke-Expression          # PowerShell
```

Set up shows that line with the full path of the program, for a terminal that
does not have `tracewing` on its `PATH`. It reads the port of the window that is
open, or of a `tracewing capture` running on the same data folder; `--port`
names another.

What it sets, and who reads it:

| Variable | Read by |
|---|---|
| `HTTP_PROXY`, `HTTPS_PROXY` (and in lower case) | curl, wget, git, Python, Go, Rust's reqwest, Deno, Bun, Node from 22.21 and 24.5 |
| `NO_PROXY` empty | the same: a server on this computer goes through as well |
| `SSL_CERT_FILE`, `CURL_CA_BUNDLE`, `REQUESTS_CA_BUNDLE`, `GIT_SSL_CAINFO`, `AWS_CA_BUNDLE` | OpenSSL and what is built on it, curl, Python's `requests`, git, the AWS CLI: a file of the system's roots with tracewing's authority added, so that hosts tracewing does not decrypt still check out |
| `NODE_EXTRA_CA_CERTS`, `DENO_CERT` | Node and Bun, Deno: tracewing's authority, added to the roots they have |
| `NODE_USE_ENV_PROXY=1` | Node, which reads `HTTPS_PROXY` only when asked to |

What does not come through, although started in such a terminal:

- **Node.js before 22.21, and 23 to 24.4.** These send every request straight
  out whatever `HTTPS_PROXY` says (24.0 to 24.4 send `fetch` through, but not
  `http` and `https`, which most libraries use). Set up says so when the Node a
  terminal runs is one of them, and so does `tracewing env`. `nvm install 22`
  or a newer Node puts it right.
- **Python from Apple, `/usr/bin/python3`.** It comes through, but its LibreSSL
  ignores `SSL_CERT_FILE`, so `urllib` turns down tracewing's certificates.
  `requests` and `httpx` load the file themselves and work; so does Python
  from python.org, Homebrew or uv.
- **Java**, which reads neither: start it with
  `-Dhttps.proxyHost=127.0.0.1 -Dhttps.proxyPort=9090`, and add tracewing's
  `ca.pem` to the keystore it uses.
- A program that pins a server's certificate, which no proxy can decrypt.

## AI agents

What an agent asks of a model is laid out on its own in a flow: the system
prompt, the messages, the tools offered, what came back, from Anthropic's,
OpenAI's, Gemini's and Ollama's APIs and those that copy OpenAI's.

- **An agent on the command line** — Claude Code, Codex, Gemini CLI, aider —
  comes through when started in a terminal like the ones above, as they read
  `HTTPS_PROXY`. Claude Code reads `NODE_EXTRA_CA_CERTS` too, and its calls
  show decrypted. One that runs on the Node.js installed on the computer, and
  does not read the proxy itself, needs a Node that does, as above.
- **An agent in an editor** — VS Code's extensions, Cursor — runs with the
  editor's environment, which is the one the editor started with. Quit the
  editor, then start it from such a terminal: `code .`, `cursor .`. An editor
  already running hands the new window to the old one, which has no proxy.
- **An agent's desktop app** — Claude, ChatGPT — comes through by the system
  proxy.
- **An agent of your own**, on an SDK: run it in such a terminal. Python's
  `anthropic` and `openai` packages, on `httpx`, and Go's take it as they are.

tracewing's MCP server goes the other way: an agent reads what tracewing has
recorded. Set up › AI tools says how to add it.

## Phones and other computers

A phone, a tablet or another computer sets its Wi-Fi proxy to this computer;
an Android app or a Flutter app needs a line or two of its own. Set up › Other
devices has a guide for each.
