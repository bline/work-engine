# Native Claude isolated-review authentication

## Status and scope

This is an operational recovery record and provisional runbook. It describes
the native Claude Code subscription-authentication path used by isolated
reviewer realizations. It does not authorize provider use, select a reviewer,
or change review, acceptance, publication, or spending authority.

No OAuth token, authorization code, access token, refresh token, or credential
content belongs in this document, Git, receipts, prompts, logs, or chatboard
messages.

## Incident observed on 2026-09-09

The reviewer adapter copied `~/.claude/.credentials.json` into every isolated
`CLAUDE_CONFIG_DIR`. A host `claude auth login` refreshed the host credential
file but did not update an existing isolated copy. Repeated isolated copies are
also unsafe as a steady-state design because OAuth refresh-token rotation can
leave the source or sibling copies stale.

A symlink was rejected as the durable solution. Claude may replace credential
files while refreshing them, and concurrent reviewers would share mutable
refresh state. A long-lived token produced by `claude setup-token` and supplied
through `CLAUDE_CODE_OAUTH_TOKEN` was investigated as the first replacement.

An old setup token produced `loggedIn: true`, `authMethod: oauth_token`, and
`apiProvider: firstParty` from `claude auth status`, but the inference path
returned `401 Invalid bearer token`. Therefore `auth status` establishes local
credential recognition only; it does not establish that the inference endpoint
will accept the token.

The replacement setup-token exchange also exposed a terminal-capture hazard:
Claude rendered the token across terminal lines. Authentication material must
not be reconstructed from wrapped presentation output. The exchange was
repeated under a wider terminal and the exact single token was captured without
logging it.

The retained retry then exposed two independent host recovery defects:

1. `retry_executing` recovery applied a pre-spawn predicate that excluded the
   `authentication_required` state it needed to reconcile.
2. Claude Code 2.1.261 reported an invalid setup token as
   `Failed to authenticate. API Error: 401 Invalid bearer token`, which was not
   among the adapter's exact pre-provider authentication signatures.

The same recovery sequence also demonstrated why newer failed-retry evidence
cannot be required to equal the recovery evidence that authorized the retry.
They are successive facts. The host must validate their shared obligation,
retained session, provider-entry boundary, and failure class, preserve the
prior recovery, and admit the newer evidence as its successor.

On Claude Code 2.1.267, a bounded follow-up experiment instead projected only
the `accessToken` from an ordinary `/login` credential as
`CLAUDE_CODE_OAUTH_TOKEN`. A fresh isolated inference returned `AUTH_OK`; a
second isolated runtime created and resumed one retained session successfully.
Neither isolated runtime materialized `.credentials.json`, so neither received
refresh authority. The non-secret receipts are retained under
`~/.local/state/work-engine/app-server-migration-934d50f/diagnostics/claude-login-access-projection-2.1.267/`.

## Current compatibility path

1. An operator runs `claude auth login --claudeai` under the intended Claude
   subscription with a durable, private canonical `CLAUDE_CONFIG_DIR`. Browser
   completion may return directly to the waiting CLI without a pasted code.
2. The canonical credential remains outside the repository. The current
   location is
   `~/.local/state/work-engine/credentials/claude-login-canonical/config/.credentials.json`;
   the file is mode `0600` and its containing directories are mode `0700`.
3. The proxy is started with `--claude-login-credentials PATH`. The path is host
   configuration and is not added to the executable-generation environment
   fingerprint. The 2026-09-09 restart retained the existing generation ID and
   reported `durable_active_current`.
4. The host opens the credential source at each review attempt, validates its
   ownership and permissions, parses only `.claudeAiOauth.accessToken` and
   `expiresAt`, removes any stale isolated `.credentials.json`, and supplies the
   access token as `CLAUDE_CODE_OAUTH_TOKEN`. It never projects the refresh
   token. Rereading permits a broker or operator to advance the canonical tip
   without recreating reviewer directories or restarting the proxy.
5. The direct-subscription environment removes `ANTHROPIC_API_KEY`,
   `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`, and OpenRouter credentials so
   authentication precedence cannot silently move an authorized subscription
   review onto API billing or a fallback gateway. It rejects inherited Claude
   cloud/gateway selectors and removes any ambient `CLAUDE_CODE_OAUTH_TOKEN`
   before adding the admitted access projection.
6. Receipts record `authenticationMechanism`, the access-token expiry, and
   `authenticationRefreshCapable: false`; they never record a token, refresh
   token, or token-derived fingerprint.

The native reviewer adapter no longer copies login credentials into isolated
reviewer configurations. An unavailable, unsafe, malformed, or expired source
fails before provider entry.

## Validation and recovery

- Check that the canonical credential source is an owner-owned, owner-only
  regular file before starting the App Server. The adapter separately requires
  one non-empty access token and a future `expiresAt` value before provider
  entry.
- Treat `claude auth status` as a preflight signal, not proof of inference
  validity. The first provider-entering validation must already be authorized,
  and its receipt supplies the stronger evidence.
- On a definite pre-provider authentication failure, retain the deterministic
  reviewer UUID and exact evidence. Do not create a fresh reviewer or switch
  providers merely to repair authentication.
- An authentication retry rereads the canonical tip. It does not itself
  refresh credentials. If the tip has not advanced or is expired, report
  authentication as unavailable rather than claiming broker recovery.
- If an admitted retry is interrupted after producing newer pre-provider
  evidence, reconcile it as a successor recovery fact. Do not erase it, restore
  an older session artifact, or force its digest to equal its predecessor.
- A quota failure is not an authentication failure. Preserve it under its own
  classification and resume according to the review authorization after reset.

## Remaining productization

The validated access projection is a compatibility mechanism, not the complete
fenced credential-tip broker. A later accepted design must own numbered
credential generations, admission horizons, live-reader drain, exclusive
refresh, compare-and-swap publication, and suspect-generation handling. Until
that exists, canonical refresh remains operator-mediated or externally owned;
reviewer runtimes must never receive the refresh token or independently run
`/login`.
