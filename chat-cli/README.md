# bens-chat-cli

This is a CLI for getting messages from the parent chat project without having to open a web browser

## install with cargo:

```bash
cargo install bens-chat-cli
```

The .exe file is also provided


## Quick start:

run bens-chat-cli

type the name of the chat you want to enter or use "n" to create a new chat

type your message and hit enter to send it

use "/exit" to go back to chats

use "/update" to refresh if necessary


## Architecture
This application is represented as a finite state automata

The Window strict holds the application state

The transition method moves between state

The other handlers (that are called by transition on state change) are defined below the transition method

## Contributing 
### Pull requests are welcome
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change

## Licence
**[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)**
