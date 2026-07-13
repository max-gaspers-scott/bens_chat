const { io } = require("socket.io-client");

const socket = io("http://localhost:8081/test", {
    transports: ["websocket"] // force WebSocket transport
});

socket.on("connect", () => {
    console.log("Connected to /test namespace!");
    console.log("Sending 'message' event...");
    socket.emit("message", "ping");
});

socket.on("message-back", (data) => {
    console.log("Received 'message-back' event from server:", data);
    socket.disconnect();
    process.exit(0);
});

socket.on("connect_error", (err) => {
    console.error("Connection error:", err.message);
});
