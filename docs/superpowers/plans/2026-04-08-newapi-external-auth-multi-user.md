# New API External Auth Multi-User Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace local username/password authentication with New API login and registration while keeping llumen's own database as the source of truth for per-user chat history, files, preferences, and ownership isolation.

**Architecture:** The backend remains the only app the SPA talks to for llumen data. Backend auth routes become an adapter: they call New API's management auth endpoints, resolve the external user identity via `GET /api/user/self`, upsert a local user row keyed by the external user id, then mint llumen's existing local PASETO so all existing owner-scoped chat and file routes continue to work. The frontend keeps the current login-first SPA shape, adds registration, removes local password/admin-user management UX, and continues sending only llumen's local token to llumen routes.

**Tech Stack:** Rust, Axum, SeaORM, reqwest, PASETO, SQLite, Svelte 5, TypeScript, pnpm, typeshare

---

## Scope Notes

- This plan assumes New API is the system of record for user credentials.
- This plan does **not** add tenant/workspace sharing. Each New API user maps to exactly one local llumen user row.
- This plan keeps the existing owner-based history isolation already present in chat/message/file routes. The implementation work is to make auth map the right external identity to the right local owner id every time.
- This plan intentionally removes or hides local user-creation/password-management features in llumen because they conflict with “user auth uses the API”.

## Cross-Domain Rule

- Browser traffic must stay same-origin to llumen: SPA -> `llumen /api/...` only.
- llumen backend performs all New API auth calls server-to-server with `reqwest`.
- Do **not** call New API directly from `frontend/src/...`; that would make auth depend on cross-domain browser CORS and expose upstream auth details to the client.
- llumen CORS only needs to support browser -> llumen development access. It does not need to solve browser -> New API.

## External Contract To Implement

- New API login: `POST /api/user/login`
- New API register: `POST /api/user/register`
- New API current user lookup: `GET /api/user/self`
- New API auth modes documented at `https://www.newapi.ai/en/docs/api/management/auth`

Because the public docs are sparse about response bodies and cookies, implementation must capture the real wire contract in tests before hard-coding assumptions. The safest adapter is:

1. Call New API login/register.
2. Reuse the returned cookies and/or bearer token when calling `GET /api/user/self`.
3. Upsert local user by external id.
4. Mint local llumen token from the local user id.

The browser must never see or call the upstream New API base URL during this flow.

## File Structure

### Backend files

- Create: `backend/src/lib.rs`
  Responsibility: expose the app modules needed by integration tests so `backend/tests/newapi_auth.rs` can compile against a library target.
- Create: `backend/src/utils/newapi_auth.rs`
  Responsibility: small New API auth client, request/response structs, login/register/self calls, cookie/token forwarding, and server-to-server conversion into a local `ExternalIdentity` struct.
- Create: `backend/tests/newapi_auth.rs`
  Responsibility: backend integration tests for the New API adapter and auth-route exchange behavior using a mocked upstream server.
- Create: `backend/migration/src/m20260408_000001_add_external_user_id.rs`
  Responsibility: add `external_user_id` to the local `user` table and make external-auth mode refuse unmapped legacy rows.
- Modify: `backend/migration/src/lib.rs`
  Responsibility: register the new migration.
- Modify: `backend/entity/src/entities/user.rs`
  Responsibility: represent the new `external_user_id` column in SeaORM.
- Modify: `backend/src/main.rs`
  Responsibility: load New API auth configuration, inject the adapter into `AppState`, and keep browser CORS scoped only to llumen.
- Modify: `backend/src/routes/auth/login.rs`
  Responsibility: replace local password verification with New API login + local token exchange.
- Create: `backend/src/routes/auth/register.rs`
  Responsibility: implement New API-backed registration + local token exchange.
- Modify: `backend/src/routes/auth/mod.rs`
  Responsibility: register the new backend `/auth/register` route.
- Modify: `backend/src/routes/auth/helper.rs`
  Responsibility: split “mint local token” from “verify local password”; helper should work with an already-known local user id.
- Modify: `backend/src/routes/auth/renew.rs`
  Responsibility: keep renew behavior local-only and verify it still works after the login rewrite.
- Modify: `backend/src/routes/user/create.rs`
  Responsibility: reject local user creation once external auth is enabled.
- Modify: `backend/src/routes/user/delete.rs`
  Responsibility: reject or remove local destructive user deletion until an explicit external deletion flow exists.
- Modify: `backend/src/routes/user/list.rs`
  Responsibility: return a disabled/unsupported error or remove it from routing if the SPA no longer needs it.
- Modify: `backend/src/routes/user/update.rs`
  Responsibility: keep local preference updates, but reject local password updates when external auth mode is active.
- Modify: `backend/src/routes/user/read.rs`
  Responsibility: expose whether external auth mode is enabled so the SPA can hide conflicting UI.

### Frontend files

- Modify: `frontend/src/lib/api/auth.ts`
  Responsibility: add register mutation, keep login token storage, and centralize auth redirect behavior.
- Create: `frontend/src/lib/api/auth.test.ts`
  Responsibility: focused auth mutation and redirect tests instead of piggybacking on unrelated query-state tests.
- Modify: `frontend/src/routes/login/+page.svelte`
  Responsibility: keep login UI, add navigation to register, and adapt any error messaging to backend adapter errors.
- Create: `frontend/src/routes/register/+page.svelte`
  Responsibility: registration screen that posts only to llumen backend and never to the upstream New API domain.
- Modify: `frontend/src/routes/+page.svelte`
  Responsibility: route unauthenticated users to either login or register entry points cleanly.
- Modify: `frontend/src/lib/components/setting/tabs/account/AccountPassword.svelte`
  Responsibility: replace local password-change form with a note or external-account link.
- Modify: `frontend/src/lib/components/setting/tabs/Admin.svelte`
  Responsibility: remove local admin user-management tab content.
- Modify: `frontend/src/lib/components/setting/Setting.svelte`
  Responsibility: remove or hide the admin section if it becomes empty.
- Modify: `frontend/src/lib/api/user.svelte.ts`
  Responsibility: stop exposing local create/delete/list mutations that no longer have UI support.
- Modify: `frontend/src/lib/api/types.ts`
  Responsibility: generated `external_auth` and register-request fields after backend changes via `cargo xtask gen-ts`.

### Docs files

- Modify: `docs/user/config/environment.mdx`
  Responsibility: document new env vars for New API auth integration and remove the misleading default-password guidance.
- Modify: `BUILD.md`
  Responsibility: add any setup steps needed to run llumen against a New API instance during development.

## Chunk 1: Backend Identity Mapping And Auth Adapter

### Task 1: Add local external identity storage

**Files:**
- Create: `backend/migration/src/m20260408_000001_add_external_user_id.rs`
- Modify: `backend/migration/src/lib.rs`
- Modify: `backend/entity/src/entities/user.rs`
- Modify: `backend/src/main.rs`
- Test: `backend/tests/newapi_auth.rs`

- [ ] **Step 1: Write the failing migration/entity test**

```rust
#[tokio::test]
async fn user_row_can_be_selected_by_external_user_id() {
    let app = TestApp::new().await;
    let created = app.insert_user_with_external_id("newapi-user-42", "alice").await;

    let fetched = entity::user::Entity::find()
        .filter(entity::user::Column::ExternalUserId.eq("newapi-user-42"))
        .one(&app.db)
        .await
        .unwrap()
        .expect("user should exist");

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, "alice");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth user_row_can_be_selected_by_external_user_id -- --nocapture`
Expected: FAIL because `ExternalUserId` column and helper setup do not exist yet.

- [ ] **Step 3: Write the minimal schema change**

```rust
manager
    .alter_table(
        Table::alter()
            .table(User::Table)
            .add_column(ColumnDef::new(User::ExternalUserId).string().null())
            .to_owned(),
    )
    .await?;

manager
    .create_index(
        Index::create()
            .name("idx-user-external-user-id")
            .table(User::Table)
            .col(User::ExternalUserId)
            .unique()
            .to_owned(),
    )
    .await?;
```

- [ ] **Step 4: Finish the migration for real data**

Implementation notes:
- Keep `external_user_id` nullable at the schema level in this migration so upgraded databases remain migratable.
- Do **not** invent mappings. Make external-auth mode fail fast on startup if any `user.external_user_id IS NULL`, with an error telling the operator that legacy local-auth rows must be mapped or migrated before enabling New API auth.
- Add test coverage for three cases: empty database, fully mapped database, and legacy database with null `external_user_id` rejected when external auth is enabled.
- Implement the fail-fast check in `backend/src/main.rs` as part of startup whenever `NEWAPI_AUTH_BASE` is set.
- Update `backend/entity/src/entities/user.rs` to add:

```rust
pub external_user_id: Option<String>,
```

- [ ] **Step 5: Run the focused test again**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth user_row_can_be_selected_by_external_user_id -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/migration/src/lib.rs \
  backend/migration/src/m20260408_000001_add_external_user_id.rs \
  backend/entity/src/entities/user.rs \
  backend/src/main.rs \
  backend/tests/newapi_auth.rs
git commit -m "feat: map local users to external auth identities"
```

### Task 2: Add a minimal New API auth client

**Files:**
- Create: `backend/src/lib.rs`
- Create: `backend/src/utils/newapi_auth.rs`
- Modify: `backend/src/utils/mod.rs`
- Modify: `backend/src/main.rs`
- Modify: `BUILD.md`
- Test: `backend/tests/newapi_auth.rs`

- [ ] **Step 1: Write the failing upstream contract test**

```rust
#[tokio::test]
async fn login_uses_upstream_identity_to_build_local_identity() {
    let upstream = MockNewApi::new()
        .expect_login("alice", "secret")
        .expect_self(42, "alice");

    let client = NewApiAuthClient::new(upstream.base_url());
    let identity = client.login("alice", "secret").await.unwrap();

    assert_eq!(identity.external_user_id, "newapi-user-42");
    assert_eq!(identity.username, "alice");
}

#[tokio::test]
async fn login_reuses_upstream_session_cookie_for_self_lookup() {
    let upstream = MockNewApi::new_cookie_login_flow(42, "alice");
    let client = NewApiAuthClient::new(upstream.base_url());

    let identity = client.login("alice", "secret").await.unwrap();

    assert_eq!(identity.external_user_id, "newapi-user-42");
}

#[tokio::test]
async fn login_reuses_upstream_bearer_token_for_self_lookup() {
    let upstream = MockNewApi::new_bearer_login_flow(42, "alice");
    let client = NewApiAuthClient::new(upstream.base_url());

    let identity = client.login("alice", "secret").await.unwrap();

    assert_eq!(identity.external_user_id, "newapi-user-42");
}

#[tokio::test]
async fn upstream_auth_failures_become_login_fail_errors() {
    let upstream = MockNewApi::new().expect_login_failure();
    let client = NewApiAuthClient::new(upstream.base_url());

    let error = client.login("alice", "wrong-password").await.unwrap_err();

    assert!(matches!(error.kind(), ErrorKind::LoginFail));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth login_uses_upstream_identity_to_build_local_identity -- --nocapture`
Expected: FAIL because `NewApiAuthClient` does not exist.

- [ ] **Step 3: Capture the real upstream wire contract once**

Run: `curl -i -X POST "$NEWAPI_AUTH_BASE/api/user/login" -H "Content-Type: application/json" -d '{"username":"<test-user>","password":"<test-password>"}'`
Expected: a recorded sample showing whether auth continuation is via `Set-Cookie`, JSON bearer token, or both.

Run: `curl -i "$NEWAPI_AUTH_BASE/api/user/self" -H "Authorization: Bearer <token-if-present>"`
Expected: a recorded sample showing the upstream user id field shape and any required `New-Api-User` behavior.

Implementation notes:
- Copy the observed status codes, cookies, headers, and id field shape into comments or fixture builders inside `backend/tests/newapi_auth.rs`.
- Update `BUILD.md` with one short developer note explaining that these capture commands are the source of truth when upstream docs and runtime behavior differ.

- [ ] **Step 4: Write minimal implementation**

```rust
pub mod config;
pub mod errors;
pub mod middlewares;
pub mod routes;
pub mod utils;

pub struct ExternalIdentity {
    pub external_user_id: String,
    pub username: String,
}

pub struct NewApiAuthClient {
    http: reqwest::Client,
    base_url: String,
}

impl NewApiAuthClient {
    pub async fn login(&self, username: &str, password: &str) -> Result<ExternalIdentity> {
        let auth = self.post_login(username, password).await?;
        self.fetch_self(auth).await
    }

    pub async fn register(&self, username: &str, password: &str, email: Option<&str>) -> Result<ExternalIdentity> {
        let auth = self.post_register(username, password, email).await?;
        self.fetch_self(auth).await
    }
}
```

- [ ] **Step 5: Make the client resilient to the sparse docs**

Implementation notes:
- Build `reqwest::Client` with cookie store enabled.
- Support both documented auth continuation modes:
  - cookie/session returned by login/register
  - bearer token returned by login/register
- Normalize both into one internal `UpstreamAuth` enum used only inside `newapi_auth.rs`.
- Parse the upstream user id into a string-compatible local identifier so the adapter can safely handle either numeric or string ids from `/api/user/self`.
- Add config in `backend/src/main.rs` for:
  - `NEWAPI_AUTH_BASE`
  - `NEWAPI_AUTH_BEARER` (optional service token when calling protected management endpoints if needed)
  - `NEWAPI_AUTH_USER_HEADER` (optional override for `New-Api-User` behavior if the upstream requires it)
- Put all New API-specific HTTP code in this file; route handlers should only call methods on the client.

- [ ] **Step 6: Run the focused test again**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth login_uses_upstream_identity_to_build_local_identity -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth login_reuses_upstream_session_cookie_for_self_lookup -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth login_reuses_upstream_bearer_token_for_self_lookup -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth upstream_auth_failures_become_login_fail_errors -- --nocapture`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add backend/src/lib.rs backend/src/utils/mod.rs backend/src/utils/newapi_auth.rs backend/src/main.rs backend/tests/newapi_auth.rs
git commit -m "feat: add new api auth client"
```

### Task 3: Rewrite backend login to exchange external auth for a local llumen token

**Files:**
- Modify: `backend/src/routes/auth/login.rs`
- Modify: `backend/src/routes/auth/helper.rs`
- Modify: `backend/src/routes/auth/renew.rs`
- Modify: `backend/src/main.rs`
- Test: `backend/tests/newapi_auth.rs`

- [ ] **Step 1: Write the failing route test**

```rust
#[tokio::test]
async fn login_upserts_local_user_and_returns_llumen_token() {
    let app = TestApp::new().await.with_upstream_user(42, "alice");

    let response = app.login("alice", "secret").await;

    assert_eq!(response.status(), StatusCode::OK);
    let token = response.json::<LoginResp>().await.token;
    let local_user = app.decode_token_user(token).await;
    assert_eq!(local_user.external_user_id.as_deref(), Some("newapi-user-42"));
}

#[tokio::test]
async fn different_external_users_create_distinct_local_users_even_if_names_change() {
    let app = TestApp::new().await;

    let alice_v1 = app.login_as_external_user(1, "alice").await;
    let bob = app.login_as_external_user(2, "bob").await;
    let alice_v2 = app.login_as_external_user(1, "alice-renamed").await;

    assert_ne!(alice_v1.local_user_id, bob.local_user_id);
    assert_eq!(alice_v1.local_user_id, alice_v2.local_user_id);
}

#[tokio::test]
async fn different_external_users_cannot_cross_read_chat_after_login_exchange() {
    let app = TestApp::new().await;
    let alice = app.login_as_external_user(1, "alice").await;
    let bob = app.login_as_external_user(2, "bob").await;
    let chat_id = app.create_chat(alice.token.clone()).await;

    let response = app.read_chat(bob.token, chat_id).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn renew_still_works_for_token_minted_from_external_login() {
    let app = TestApp::new().await.with_upstream_user(42, "alice");
    let login = app.login("alice", "secret").await;
    let token = login.json::<LoginResp>().await.token;

    let renewed = app.renew(token).await;

    assert_eq!(renewed.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth login_upserts_local_user_and_returns_llumen_token -- --nocapture`
Expected: FAIL because login still reads local password hashes.

- [ ] **Step 3: Write minimal implementation**

```rust
let identity = app.newapi_auth.login(&req.username, &req.password).await?;
let local_user = upsert_local_user(&app.conn, identity).await?;
let Token { token, exp } = helper::new_token(&app, local_user.id)?;
Ok(Json(LoginResp { token, exp }))
```

- [ ] **Step 4: Implement the local upsert carefully**

Implementation notes:
- Upsert key must be `external_user_id`, not username.
- On repeat login, update local `name` if the upstream username changed.
- Do **not** overwrite local `preference` during login.
- Keep using local PASETO for llumen routes so the rest of the app remains unchanged.
- Map upstream auth failures to `ErrorKind::LoginFail`, not `Internal`.

- [ ] **Step 5: Run the focused test again**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth login_upserts_local_user_and_returns_llumen_token -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth different_external_users_create_distinct_local_users_even_if_names_change -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth different_external_users_cannot_cross_read_chat_after_login_exchange -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth renew_still_works_for_token_minted_from_external_login -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/src/routes/auth/login.rs backend/src/routes/auth/helper.rs backend/src/routes/auth/renew.rs backend/src/main.rs backend/tests/newapi_auth.rs
git commit -m "feat: exchange external login for local session token"
```

### Task 4: Add backend registration via New API and local user bootstrap

**Files:**
- Create: `backend/src/routes/auth/register.rs`
- Modify: `backend/src/routes/auth/mod.rs`
- Test: `backend/tests/newapi_auth.rs`

- [ ] **Step 1: Write the failing register route test**

```rust
#[tokio::test]
async fn register_creates_external_user_then_returns_llumen_token() {
    let app = TestApp::new().await.with_registering_upstream_user(77, "bob");

    let response = app.register("bob", "secret", Some("bob@example.com")).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.json::<RegisterResp>().await;
    assert!(!body.token.is_empty());
    let local_user = app.decode_token_user(body.token).await;
    assert_eq!(local_user.external_user_id.as_deref(), Some("newapi-user-77"));
}

#[tokio::test]
async fn register_supports_cookie_and_bearer_follow_up_identity_lookup() {
    let app = TestApp::new().await
        .with_cookie_registering_upstream_user(77, "bob")
        .with_bearer_registering_upstream_user(88, "carol");

    assert_eq!(app.register("bob", "secret", Some("bob@example.com")).await.status(), StatusCode::OK);
    assert_eq!(app.register("carol", "secret", Some("carol@example.com")).await.status(), StatusCode::OK);
}

#[tokio::test]
async fn upstream_registration_failures_are_returned_as_bad_request() {
    let app = TestApp::new().await.with_failing_upstream_registration();

    let response = app.register("taken", "secret", Some("taken@example.com")).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn separately_registered_users_get_isolated_local_history() {
    let app = TestApp::new().await
        .with_registering_upstream_user(77, "bob")
        .with_registering_upstream_user(88, "carol");

    let bob = app.register("bob", "secret", Some("bob@example.com")).await;
    let carol = app.register("carol", "secret", Some("carol@example.com")).await;
    let bob_token = bob.json::<RegisterResp>().await.token;
    let carol_token = carol.json::<RegisterResp>().await.token;
    let chat_id = app.create_chat(bob_token).await;

    let response = app.read_chat(carol_token, chat_id).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth register_creates_external_user_then_returns_llumen_token -- --nocapture`
Expected: FAIL because `/api/auth/register` does not exist.

- [ ] **Step 3: Write minimal implementation**

```rust
#[derive(Debug, Deserialize)]
pub struct RegisterReq {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub verification_code: Option<String>,
    pub aff_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResp {
    pub token: String,
    pub exp: String,
}
```

- [ ] **Step 4: Match New API’s real validation contract**

Implementation notes:
- Keep llumen backend validation minimal: non-empty username/password, optional email format sanity check only if the current frontend already validates email the same way.
- Pass `email`, `verification_code`, and `aff_code` through unchanged because they are documented by New API even if the current deployment leaves them optional.
- Reuse the same `upsert_local_user` helper as login.

- [ ] **Step 5: Run the focused test again**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth register_creates_external_user_then_returns_llumen_token -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth register_supports_cookie_and_bearer_follow_up_identity_lookup -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth upstream_registration_failures_are_returned_as_bad_request -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth separately_registered_users_get_isolated_local_history -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Run backend-wide verification for this chunk**

Run: `cargo check --manifest-path backend/Cargo.toml`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml`
Expected: PASS.

Run: `cargo +nightly fmt --manifest-path backend/Cargo.toml --all --check`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add backend/src/routes/auth/mod.rs backend/src/routes/auth/register.rs backend/tests/newapi_auth.rs
git commit -m "feat: add new api-backed registration"
```

## Chunk 2: Frontend Auth UX, Cleanup, And Verification

### Task 5: Add SPA registration and keep login flow local to llumen backend

**Files:**
- Modify: `frontend/src/lib/api/auth.ts`
- Create: `frontend/src/lib/api/auth.test.ts`
- Modify: `frontend/src/routes/login/+page.svelte`
- Create: `frontend/src/routes/register/+page.svelte`
- Modify: `frontend/src/routes/+page.svelte`
- Modify: `frontend/src/lib/api/types.ts`
- Test: `frontend/src/lib/api/auth.test.ts`

- [ ] **Step 1: Write the failing frontend auth tests**

```ts
it('stores the returned llumen token after registration', async () => {
	mockApi('auth/register', { token: 'abc', exp: '2099-01-01T00:00:00Z' });
	const mutation = Register();

	await mutation.mutateAsync({ username: 'bob', password: 'secret', email: 'bob@example.com' });

	expect(get(token)?.value).toBe('abc');
});

it('allows unauthenticated navigation to /register', async () => {
	mockPage('/register');
	initAuth();
	expect(mockGoto).not.toHaveBeenCalledWith('/login');
});

it('blocks submit when password confirmation does not match', async () => {
	render(RegisterPage);
	await userEvent.type(screen.getByLabelText(/password/i), 'secret');
	await userEvent.type(screen.getByLabelText(/confirm password/i), 'different');
	expect(screen.getByRole('button', { name: /register/i })).toBeDisabled();
});

it('submits all supported upstream registration fields', async () => {
	render(RegisterPage);
	await userEvent.type(screen.getByLabelText(/username/i), 'bob');
	await userEvent.type(screen.getByLabelText(/^password$/i), 'secret');
	await userEvent.type(screen.getByLabelText(/confirm password/i), 'secret');
	await userEvent.type(screen.getByLabelText(/email/i), 'bob@example.com');
	await userEvent.type(screen.getByLabelText(/verification code/i), '123456');
	await userEvent.type(screen.getByLabelText(/affiliate code/i), 'AFF-1');
	await userEvent.click(screen.getByRole('button', { name: /register/i }));

	expect(mockApi).toHaveBeenCalledWith('auth/register', expect.objectContaining({
		username: 'bob',
		password: 'secret',
		email: 'bob@example.com',
		verification_code: '123456',
		aff_code: 'AFF-1'
	}));
});

it('shows login and register entry points on the landing page when logged out', async () => {
	mockPage('/');
	render(HomePage);
	expect(screen.getByRole('link', { name: /login/i })).toBeInTheDocument();
	expect(screen.getByRole('link', { name: /register/i })).toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir frontend test frontend/src/lib/api/auth.test.ts`
Expected: FAIL because `Register()` is not implemented and `/register` is still redirected to `/login`.

- [ ] **Step 3: Regenerate backend-derived frontend types**

Run: `cargo run --package xtask --manifest-path backend/Cargo.toml -- gen-ts`
Expected: PASS and `frontend/src/lib/api/types.ts` now includes register request/response shapes plus `external_auth` on `UserReadResp`.

- [ ] **Step 4: Write minimal implementation**

```ts
export function Register(): MutationResult<RegisterReq, RegisterResp> {
	return createMutation({
		path: 'auth/register',
		onSuccess: storeTokenFromAuthResponse
	});
}
```

- [ ] **Step 5: Build the register screen**

Implementation notes:
- Reuse the current login page visual language.
- Post only to llumen backend, never directly from browser to New API.
- Keep `apiBase` pointed at llumen only. Do not add any `NEWAPI_AUTH_BASE` usage in frontend code.
- Add fields in the first pass:
  - username
  - password
  - confirm password
  - email
  - verification code
  - affiliate code
- Add a link from login to register and from register back to login.
- After success, reuse the same redirect callback handling as login.
- Update `frontend/src/lib/api/auth.ts` so unauthenticated route guards allow both `/login` and `/register`.
- Update `frontend/src/routes/+page.svelte` so the landing page can guide unauthenticated users to either sign in or register without bouncing.

- [ ] **Step 6: Run focused frontend verification**

Run: `pnpm --dir frontend test frontend/src/lib/api/auth.test.ts`
Expected: PASS.

Expected coverage includes: registration submits to llumen `auth/register`, not to the upstream New API host.

Run: `pnpm --dir frontend check`
Expected: PASS.

Run: `pnpm --dir frontend test`
Expected: PASS with the new auth page coverage included.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/api/auth.ts frontend/src/lib/api/auth.test.ts frontend/src/routes/login/+page.svelte frontend/src/routes/register/+page.svelte frontend/src/routes/+page.svelte frontend/src/lib/api/types.ts
git commit -m "feat: add registration flow to the spa"
```

### Task 6: Remove local-only account management that conflicts with external auth

**Files:**
- Modify: `frontend/src/lib/components/setting/tabs/account/AccountPassword.svelte`
- Modify: `frontend/src/lib/components/setting/tabs/Admin.svelte`
- Modify: `frontend/src/lib/components/setting/Setting.svelte`
- Create: `frontend/src/lib/components/setting/setting.test.ts`
- Modify: `frontend/src/lib/api/user.svelte.ts`
- Modify: `backend/src/routes/user/read.rs`
- Modify: `backend/src/routes/user/create.rs`
- Modify: `backend/src/routes/user/delete.rs`
- Modify: `backend/src/routes/user/list.rs`
- Modify: `backend/src/routes/user/update.rs`
- Test: `backend/tests/newapi_auth.rs`

- [ ] **Step 1: Write the failing backend and UI assertions**

```rust
#[tokio::test]
async fn updating_password_is_rejected_when_external_auth_is_enabled() {
    let app = TestApp::external_auth_enabled().await;
    let response = app.update_password("new-secret").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn local_user_management_routes_are_rejected_when_external_auth_is_enabled() {
    let app = TestApp::external_auth_enabled().await;

    assert_eq!(app.create_user("eve").await.status(), StatusCode::BAD_REQUEST);
    assert_eq!(app.list_users().await.status(), StatusCode::BAD_REQUEST);
    assert_eq!(app.delete_user(9).await.status(), StatusCode::BAD_REQUEST);
}
```

```ts
it('does not render local admin user creation when external auth is active', async () => {
	mockCurrentUser({ user_id: 1, username: 'alice', preference: {}, external_auth: true });
	render(Setting, { open: true });
	expect(screen.queryByText(/create user/i)).toBeNull();
});

it('replaces password editing with an external-auth notice', async () => {
	mockCurrentUser({ user_id: 1, username: 'alice', preference: {}, external_auth: true });
	render(Setting, { open: true });
	expect(screen.getByText(/managed by your external account provider/i)).toBeInTheDocument();
});

it('removes the admin tab when it has no remaining content', async () => {
	mockCurrentUser({ user_id: 1, username: 'alice', preference: {}, external_auth: true });
	render(Setting, { open: true });
	expect(screen.queryByText(/admin settings/i)).toBeNull();
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth updating_password_is_rejected_when_external_auth_is_enabled -- --nocapture`
Expected: FAIL because password updates still work locally.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth local_user_management_routes_are_rejected_when_external_auth_is_enabled -- --nocapture`
Expected: FAIL because the local user-management routes still work.

Run: `pnpm --dir frontend test`
Expected: FAIL because admin/local-password UI still exists.

- [ ] **Step 3: Write the minimal implementation**

```rust
if app.newapi_auth.is_enabled() && password.is_some() {
    return Err(Json(Error {
        error: ErrorKind::BadRequest,
        reason: "Password is managed by the external auth provider".to_owned(),
    }));
}
```

```rust
pub struct UserReadResp {
    pub user_id: i32,
    pub username: String,
    pub preference: UserPreference,
    pub external_auth: bool,
}
```

```svelte
<p class="text-sm text-text-subtle">
	Password changes are managed by your external account provider.
</p>
```

- [ ] **Step 4: Keep the cleanup minimal**

Implementation notes:
- Do not invent a new role system in this project phase.
- Remove the Admin tab entirely if it contains no remaining features.
- Keep `user/read` and `user/update` for local preference management.
- Make unsupported backend routes fail loudly with a clear message rather than silently no-op.

- [ ] **Step 5: Run focused verification again**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth updating_password_is_rejected_when_external_auth_is_enabled -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth local_user_management_routes_are_rejected_when_external_auth_is_enabled -- --nocapture`
Expected: PASS.

Run: `pnpm --dir frontend test`
Expected: PASS.

Run: `pnpm --dir frontend check`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/src/routes/user/read.rs backend/src/routes/user/create.rs backend/src/routes/user/delete.rs backend/src/routes/user/list.rs backend/src/routes/user/update.rs \
  frontend/src/lib/components/setting/tabs/account/AccountPassword.svelte \
  frontend/src/lib/components/setting/tabs/Admin.svelte \
  frontend/src/lib/components/setting/Setting.svelte \
  frontend/src/lib/components/setting/setting.test.ts \
  frontend/src/lib/api/user.svelte.ts
git commit -m "refactor: remove local-only user management in external auth mode"
```

### Task 7: Verify history isolation and finish docs

**Files:**
- Modify: `backend/tests/newapi_auth.rs`
- Modify: `docs/user/config/environment.mdx`
- Modify: `BUILD.md`

- [ ] **Step 1: Add ownership regression tests**

```rust
#[tokio::test]
async fn one_external_user_cannot_read_another_users_chat_history() {
    let app = TestApp::new().await;
    let alice = app.login_as_external_user(1, "alice").await;
    let bob = app.login_as_external_user(2, "bob").await;
    let chat_id = app.create_chat(alice.token).await;

    let response = app.read_chat(bob.token, chat_id).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn one_external_user_cannot_see_another_users_chat_list() {
    let app = TestApp::new().await;
    let alice = app.login_as_external_user(1, "alice").await;
    let bob = app.login_as_external_user(2, "bob").await;
    let _chat_id = app.create_chat(alice.token.clone()).await;

    let response = app.paginate_chats(bob.token).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.json::<ChatPaginateResp>().await.list.is_empty());
}

#[tokio::test]
async fn one_external_user_cannot_see_another_users_message_history() {
    let app = TestApp::new().await;
    let alice = app.login_as_external_user(1, "alice").await;
    let bob = app.login_as_external_user(2, "bob").await;
    let chat_id = app.create_chat(alice.token.clone()).await;
    app.create_message(alice.token, chat_id, "hello").await;

    let response = app.paginate_messages(bob.token, chat_id).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn register_then_later_login_reuses_the_same_local_history() {
    let app = TestApp::new().await.with_upstream_user(77, "bob");
    let register = app.register("bob", "secret", Some("bob@example.com")).await;
    let register_token = register.json::<RegisterResp>().await.token;
    let chat_id = app.create_chat(register_token.clone()).await;

    let login = app.login("bob", "secret").await;
    let login_token = login.json::<LoginResp>().await.token;
    let response = app.read_chat(login_token, chat_id).await;

    assert_eq!(response.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run the ownership regression tests**

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth one_external_user_cannot_read_another_users_chat_history -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth one_external_user_cannot_see_another_users_chat_list -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth one_external_user_cannot_see_another_users_message_history -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml --test newapi_auth register_then_later_login_reuses_the_same_local_history -- --nocapture`
Expected: PASS.

- [ ] **Step 3: Document the new setup**

Add to `docs/user/config/environment.mdx`:

```mdx
| `NEWAPI_AUTH_BASE` | Base URL for New API management auth | None |
| `NEWAPI_AUTH_BEARER` | Optional bearer token for protected management calls | None |
| `NEWAPI_AUTH_USER_HEADER` | Optional header name/value behavior for upstream user identification | `New-Api-User` |
```

Replace the old “default password” guidance with:

```mdx
<Accordion title="External account management">
  - User login and registration are delegated to your New API instance.
  - Llumen stores only local conversation history, preferences, and file ownership.
  - Password resets and account recovery happen in your New API deployment.
</Accordion>
```

- [ ] **Step 4: Add development workflow notes**

Add to `BUILD.md` a short section showing how to run llumen with a New API instance available, including required env vars and the note that frontend users authenticate through llumen backend, not directly to New API.

- [ ] **Step 5: Run full verification**

Run: `cargo check --manifest-path backend/Cargo.toml`
Expected: PASS.

Run: `cargo test --manifest-path backend/Cargo.toml`
Expected: PASS.

Run: `cargo +nightly fmt --manifest-path backend/Cargo.toml --all --check`
Expected: PASS.

Run: `cargo run --package xtask --manifest-path backend/Cargo.toml -- gen-ts`
Expected: PASS and updated frontend auth/user types.

Run: `pnpm --dir frontend check`
Expected: PASS.

Run: `pnpm --dir frontend test`
Expected: PASS.

Run: `pnpm --dir frontend lint`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/tests/newapi_auth.rs docs/user/config/environment.mdx BUILD.md frontend/src/lib/api/types.ts
git commit -m "docs: describe external auth configuration"
```

## Notes For The Implementer

- Keep the New API integration behind a small adapter. Do not scatter upstream URL building across route handlers.
- Do not change the existing owner checks in chat/message/file routes unless tests prove a bug.
- Prefer a single new backend test file with focused integration tests over many tiny files.
- If New API returns richer fields from `/api/user/self` than this plan assumed, persist only what llumen actually needs now: stable external id and display name.
- If you discover the upstream login endpoint is session-cookie-only, keep that detail entirely inside `newapi_auth.rs`; the frontend should still only receive llumen's local token.

Plan complete and saved to `docs/superpowers/plans/2026-04-08-newapi-external-auth-multi-user.md`. Ready to execute?
