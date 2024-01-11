# 🎄 Day 19: Christmas Sockets on the Chimney

*On a cold and snowy winter day, Santa Claus was busy with his annual routine when he spotted a new delivery of vibrant socks hanging on his chimney. The hues and prints on these socks were unlike anything he had seen before - intricate patterns with tiny paddles embroidered on them. He chuckled, remembering how he used to juggle between writing protocols for his websocket apps and practising his backhand strokes on his virtual table tennis game.*

## ⭐ Task 1: Table Tennis Server 🏓

Write a WebSocket GET endpoint `/19/ws/ping` that listens for messages of type Text.

- If the incoming string is `serve`, the game starts in this WebSocket.
- If and only if the game has started, respond with a string `pong` whenever the incoming string is `ping`.
- All other incoming messages should be ignored.

### 🔔 Tips

Check your web framework's documentation for how to use WebSockets.

### 💠 Example

curl is not sufficient for testing WebSocket behavior with simple commands.
Use the official validator (link at bottom of page) to run local tests for this challenge.

## 🎁 Task 2: Bird App Simulator (500 bonus points)

To improve internal communications at the North Pole, Santa is trying out a real-time variant of Twitter (sometimes referred to as a "chat app").
*(Santa is old-school & cool - still calls it Twitter instead of X).*

In order to know how much the elves are using the platform, Santa wants some metrics.
He thinks it is sufficient to just count the total number of views on all tweets.

Here are the required endpoints:

- POST endpoint `/19/reset` that resets the counter of tweet views.
- GET endpoint `/19/views` that returns the current count of tweet views.
- GET endpoint `/19/ws/room/<number>/user/<string>` that opens a WebSocket and connects a user to a room.

This is how the app should work:

- A user can at any time send a tweet as a Text WebSocket message in the format `{"message":"Hello North Pole!"}`.
- When a tweet is received, broadcast it to everyone in the same room (including the sender).
- Tweets with more than 128 characters are too long and should be ignored by the server.
- Tweets sent out to room members should have the format `{"user":"xX_f4th3r_0f_chr1stm4s_Xx","message":"Hello North Pole!"}` where user is the author of the tweet (the username that the sender used in the endpoint's URL path).
- Every time a tweet is successfully sent out to a user, it counts as one view.
- Keep a running count of the number of views that happen, and return the current view count from the `/19/views` endpoint whenever requested.
- When a websocket closes, that user leaves the room and should no longer receive tweets.
- When the reset endpoint is called, the counter is set to 0.

The view counter can be in-memory and does not need to persist.

---

Author: [jonaro00](https://github.com/jonaro00)
