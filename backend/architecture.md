This project uses a tree of messages.

Each message has a parent, but this will be null for root messages. A client can query the database where parent_id = ... and get a group of messages together.
A parent message is the same as a "chat" in other applications.
A parent is any message/node that has children — i.e. a message that has its id in another message's parent_id field.
A user has access to a chat (group of messages sharing a parent) iff they are associated with that parent node in the chat_participants table.

## Project structure

This is a Cargo workspace with three crates:

```
bens_chat/
├── Cargo.toml          # workspace root
├── shared/             # bens-chat-shared: types used by both backend and CLI
├── backend/            # axum HTTP + socket.io server
└── chat-cli/           # terminal CLI client
```

### shared

Contains all data types that cross the backend/CLI boundary: `User`, `Message<C>`,
`Note`, `SendMessage`, `SendableContent` (and its variants: `TextMessage`,
`TitleMessage`, `ImgMessage`, `Connect4`).

`Message` is generic over its content type `C`:
- The backend uses `Message<serde_json::Value>` — content stays as raw JSONB from the DB.
- The CLI uses `Message<SendableContent>` — content is deserialized into a typed enum.

`sqlx::FromRow` is derived on DB-facing types only when the `db` feature is enabled.
The backend enables it (`features = ["db"]`); the CLI does not, so sqlx is never
compiled into the CLI binary.

### backend

Axum server. DB types (`User`, `Message`, `Note`) are imported from `bens-chat-shared`.
Backend-only request/response types (`LoginRequest`, `FetchUrlQuery`, etc.) live in
`backend/src/models/misc.rs`.

### chat-cli

Terminal client. Imports shared data types and defines CLI-only behaviour on top of
them — display logic lives in `chat-cli/src/structs/display.rs` behind the `Showable`
trait (adding methods to foreign types via a local trait rather than inherent impls).


