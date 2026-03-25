## JWT Auth Implementation Plan for `bens_chat2`

### 1. Overview
- Goal: add password-verified JWT auth to the Axum + React chat app with minimal backend refactoring and minimal frontend disruption.
- Success criteria: signup hashes passwords with bcrypt, login verifies bcrypt and returns `{ user_id, token }`, protected API calls read `Authorization: Bearer <jwt>`, and the React app stores/sends the token automatically.
- Scope: keep current DB schema if possible, keep most handlers in `src/main.rs`, and avoid large route/model rewrites.
- Current backend routes in `src/main.rs`: `GET /health`, `GET /user-chats`, `POST /messages`, `POST /users`, `POST /chats`, `POST /user-chats`, `GET /messages`, `GET /users`, plus static fallback.
- Current login flow: `frontend/src/components/Login.js` collects only `username`, calls `api.login(username)` in `frontend/src/api/api.js`, which does `GET /users?username=...`; `src/main.rs` returns `{ user_id, username }` if the user exists.
- Current frontend auth usage: `frontend/src/App.js` stores `{ user_id, username }` in React state only; `CreateChat.js`, `ChatList.js`, `SendMessage.js`, and `ChatView.js` trust `currentUser.user_id` from the client.
- Current signup flow is also unsafe: `frontend/src/components/SignUp.js` sends the raw password in a field named `password_hash`, and `src/main.rs` inserts that value directly into `users.password_hash` using `src/models/user.rs`.
- Why insecure: anyone can log in by knowing a username, passwords may be stored unhashed/non-bcrypt, and callers can spoof `user_id`/`sender_id` in request bodies and query params.
- Repo-specific assumption for rollout: existing DB contents are disposable test data only, so breaking auth-related changes and resetting existing DB data are acceptable if that keeps the implementation simpler.

### 2. Prerequisites
- Rust deps to add: `bcrypt` and `jsonwebtoken` (no frontend package change required).
- New env/config: `JWT_SECRET` (required), optional `JWT_EXP_HOURS`.
- DB/schema: `migrations/0001_data.sql` already has `users.password_hash TEXT NOT NULL`, so no schema change is required for JWT auth itself.
- CORS: current `allow_headers(Any)` in `src/main.rs` should already allow `Authorization`.
- Because only test data exists, prefer resetting/recreating existing users over building legacy password compatibility logic.

### 3. Implementation Steps
1. **Step 1: Add auth DTOs and bcrypt-backed signup/login**
   - Files: modify `src/main.rs`, `src/models/user.rs`, `Cargo.toml`.
   - Details: keep `User` for DB rows, but add request/response DTOs such as `CreateUserRequest { username, email, password, phone_number }`, `LoginRequest { username, password }`, and `LoginResponse { user_id, token }`; change `POST /users` to hash with bcrypt before insert; replace login usage with a new `POST /auth/login` handler that loads the user by username and runs `bcrypt::verify`.
   - Testing: verify signup stores a bcrypt string in `users.password_hash`; verify login succeeds with a correct password and fails with wrong password / unknown user.
2. **Step 2: Add a small JWT auth module instead of refactoring all handlers**
   - Files: create `src/auth.rs`; modify `src/main.rs`.
   - Details: add `Claims { sub, username, exp }`, `create_token`, and `authorize` middleware that reads `Authorization: Bearer <jwt>`, validates the token, and stores claims/user id in request extensions; wire it in `src/main.rs` with the Axum middleware pattern, e.g. `.route("/protected/", get(...).layer(middleware::from_fn(auth::authorize)))` or an equivalent protected sub-router.
   - Testing: unit-test token creation/validation if convenient; integration-test missing header, malformed token, expired token, and valid token cases.
3. **Step 3: Protect the minimum route set while keeping current route structure**
   - Files: modify `src/main.rs`.
   - Details: make these routes public: `GET /health`, `POST /users`, `POST /auth/login`, static fallback. Make these routes protected: `GET /users?username=` (used for invite lookup), `GET /user-chats`, `POST /user-chats`, `GET /messages`, `POST /messages`, `POST /chats`.
   - Testing: confirm public routes still work without a token and protected routes return `401` without `Authorization`.
4. **Step 4: Stop trusting spoofable client identity fields where possible**
   - Files: modify `src/main.rs`, possibly `src/models/user.rs` if request structs change.
   - Details: for minimal disruption, keep current payload shapes initially, but read the authenticated user from JWT claims and either ignore or validate incoming `user_id`/`sender_id`; best minimal wins are: derive `user_id` for `GET /user-chats` from the token, and derive `sender_id` for `POST /messages` from the token instead of the request body.
   - Testing: verify a token for user A cannot fetch user B chats or send as user B by spoofing ids.
5. **Step 5: Update the frontend API layer once, then keep components small**
   - Files: modify `frontend/src/api/api.js`.
   - Details: add token helpers (`getToken`, `setToken`, `clearToken`, `authHeaders`), change `login` to `POST /auth/login`, change `signUp` to send `password` instead of `password_hash`, and automatically include `Authorization` on protected requests so chat components need minimal edits.
   - Testing: mock `fetch` and verify protected calls send the bearer token and login stores/returns the token payload.
6. **Step 6: Make minimal frontend component changes**
   - Files: modify `frontend/src/components/Login.js`, `frontend/src/components/SignUp.js`, `frontend/src/App.js`, `frontend/src/components/CreateChat.js`, `frontend/src/components/ChatList.js`, `frontend/src/components/SendMessage.js`, `frontend/src/components/ChatView.js`.
   - Details: `Login.js` needs a password field and should pass `{ user_id, username, token }` to the app; `SignUp.js` should rename local state from `password_hash` to `password`; `frontend/src/App.js` (actual location; repo does not have `frontend/src/components/App.js`) should persist/clear auth state and optionally hydrate from `localStorage`; the chat components should rely on the API helper for auth headers and handle `401` by logging out or showing an auth error.
   - Testing: update/add React tests so login renders username+password, app can transition to chat on successful login, and auth failures surface clearly.
7. **Step 7: Simplify rollout by resetting disposable auth data if needed**
   - Files: usually none beyond normal backend/frontend changes; review `migrations/0001_data.sql` and local Docker/Postgres setup only if a DB reset is needed.
   - Details: do not add legacy password-format support. If existing `users.password_hash` rows are plaintext or otherwise incompatible with bcrypt, delete/reset current test users (or recreate the local DB volume) so all future users are created through the new bcrypt-backed signup flow.
   - Testing: after the reset path, create a fresh user via signup and confirm login works only with the bcrypt-backed credentials.
8. **Step 8: Roll out incrementally**
   - Files: backend/frontend files above plus tests under `tests/` and `frontend/src/**/*.test.js`.
   - Details: recommended order is: signup hashing -> login endpoint -> frontend login/signup -> protect `GET /user-chats` first -> protect remaining chat/message routes -> tighten spoofed-id validation -> optional final Docker Compose smoke test.
   - Testing: after each sub-step, run the smallest relevant backend/frontend tests; at the end, do one manual login-and-chat smoke test only in an environment where Postgres is available, preferably via Docker Compose for this repo, and if time permits finish with a full containerized validation using Docker Compose.

### 4. File Changes Summary
- **Create:** `src/auth.rs`; likely `tests/auth_integration_tests.rs`; optionally `frontend/src/components/Login.test.js` or `SignUp.test.js`.
- **Modify:** `Cargo.toml`, `src/main.rs`, `src/models/user.rs`, `frontend/src/api/api.js`, `frontend/src/components/Login.js`, `frontend/src/components/SignUp.js`, `frontend/src/App.js`, `frontend/src/components/CreateChat.js`, `frontend/src/components/ChatList.js`, `frontend/src/components/SendMessage.js`, `frontend/src/components/ChatView.js`.
- **Review/usually unchanged:** `migrations/0001_data.sql` unless you decide to reset/reseed local test data through the Docker/local DB workflow.
- **Delete:** none.

### 5. Testing Strategy
- Backend: add integration coverage for `POST /users`, `POST /auth/login`, and `401/200` behavior on a protected route in `tests/`.
- Frontend: update `frontend/src/App.test.js` and add focused tests for login/signup/api header behavior.
- Manual smoke test: only run this after Postgres is available; for this repo the safest expectation is to start the stack with Docker Compose first, then sign up a new user, confirm DB stores bcrypt, log in, inspect the returned token, create a chat, fetch chats/messages with the token, and verify the same requests fail without a token.
- Recommended final validation: use Docker Compose (`docker compose build` and `docker compose up`) to provide Postgres and confirm the auth flow works in the packaged environment end-to-end.

### 6. Rollback Plan
- Revert frontend to username-only login UI and old `api.login` call.
- Remove auth middleware and `POST /auth/login`, reopening existing routes.
- Because current data is disposable, the simplest rollback/reset path is to rebuild/reseed the local DB rather than preserve incompatible auth records.
- Keep the old `GET /users?username=` handler until the new login path is proven, then remove or lock it down.

### 7. Estimated Effort
- Effort: about 0.5-1.5 days for the code changes and targeted tests; add extra time only if you include the final Docker Compose build/run validation.
- Complexity: medium. The codebase is small, but auth touches request models, route protection, frontend state, and test coverage.
