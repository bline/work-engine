# devwits-auth

The public pages at https://auth.devwits.ai, plus the laptop side of owner
approvals. See [HANDOFF.md](HANDOFF.md) for deployment details and the ownership
boundary with the separately maintained MCP server and bridge.

Browse the [UI screenshot gallery](docs/ui-gallery/README.md) for full-page
desktop and phone views of every route and the main UI states. An interactive
viewer and read-only sample-data preview are included.

The phase 1 [typed task review and history](docs/task-ui.md) candidate includes
six additional full-page desktop/phone captures and reproducible browser checks.

## Pages

| path | what |
|---|---|
| `/` | sign-in state, sign-out, links |
| `/oauth/authorize` | Stytch Connected Apps consent page (ChatGPT connector sign-in) |
| `/passkeys` | add a Stytch sign-in passkey |
| `/approve/setup` | register an approval passkey (separate from sign-in) |
| `/approve` | answer open owner requests from the board, signed per decision |

Sign-in is passkey-only; sign-ups are disabled in the Stytch test project.

## API (Vercel functions, dedicated Upstash Redis `devwits-mailbox`)

| route | caller | purpose |
|---|---|---|
| `PUT /api/requests` | laptop token | replace the open-request snapshot |
| `GET /api/requests` | owner session | read it |
| `POST /api/answers` | owner session | store a signed answer (checked and verified first) |
| `GET /api/answers` | laptop token | collect answers |
| `DELETE /api/answers?answer_id=` | laptop token | remove one after posting |
| `GET /api/approval-keys` | either | list approval passkeys (public data) |
| `POST /api/approval-keys?step=options\|verify` | owner session | register one |

The owner is a Stytch session JWT for user
`user-test-8b1b7b52-0fa4-4b70-8594-b2b043592a1c`, verified against the
project's public JWKS. The laptop token is `MAILBOX_TOKEN` (Vercel, sensitive;
locally `~/.config/devwits/mailbox.env`, mode 600).

## Laptop

```sh
python3 laptop/approval_sync.py sync          # push requests, pull and post answers
python3 laptop/approval_sync.py sync --force  # republish even if the open set is unchanged
python3 laptop/approval_sync.py keys          # registered approval passkeys
python3 laptop/approval_sync.py pin <prefix>  # trust one, by fingerprint
python3 -m unittest discover -s laptop        # verifier tests
```

Format and verification rules: [docs/owner-answer-v1.md](docs/owner-answer-v1.md).

The sync caches the last successfully uploaded open-request set under
`~/.local/state/devwits-approval-sync/last-push.json`. Unrelated board activity
does not cause another upload. Changed, added, and closed requests do. A failed
upload is retried on the next run. Use `--force` after changing mailbox storage
or restoring its data. Timer and manual sync runs share a process lock.

The units in `laptop/systemd/` are installed and enabled as the systemd user
timer `devwits-approval-sync.timer`, with a 60-second interval after each run
finishes. With user lingering disabled, the timer runs only while the owner is
logged in, like the existing bridge services. Inspect it with
`systemctl --user status devwits-approval-sync.timer` and read execution logs
with `journalctl --user -u devwits-approval-sync.service`.

Redis holds the latest snapshot, pending answers, registered approval keys and
short-lived registration challenges. The old private Blob store is retained
for rollback and is no longer used by the API. The migration utility in
`scripts/migrate-blob-mailbox.mjs` verifies public-key pins and keeps a private
backup; a completion marker prevents a later rerun from restoring stale data.

```sh
npm test                         # API authorization and validation checks
npm run test:redis               # real Redis checks in an isolated namespace
```

For the Redis checks, first pull the development environment to
`.env.redis.local` (gitignored; keep mode 600). Production-only secrets are
not needed for these tests. Test records are removed afterward.

## Develop and deploy

```sh
npm install
npm run dev                                   # http://localhost:3000 (sign-in only;
                                              # approvals need auth.devwits.ai)
vercel deploy --prod --yes --scope blines-projects
```

No Stytch secret is used anywhere. The public token is in `src/config.js`.
