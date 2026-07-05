This project uses a tree of messages. 

Each message has a parent, but this will be null for root messages. A client can query the database Where parnet = ... and get a grout of messages together.
A parent message is the same as a "chat" in other aplications.
A parent is any message/node that has children aka a message that has its id in it parent field.
A user has acses to a chat (group of messages sharing a parent) iff they are acociated with that parent node in the chat_participants table.



