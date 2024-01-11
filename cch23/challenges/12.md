# 🎄 Day 12: Timekeeper

*One frosty night, Santa, dressed warmly in his favorite red coat, decided to take a midnight stroll around the elf workshop. As he pushed open the heavy wooden doors of the workshop, his eyes widened in surprise. He was completely stunned by the sight that greeted him.*

*Rows upon rows of conveyor belts had been set up, zipping toys from one corner to the other, resembling an intricate dance of festivity and efficiency. The elves were operating with military precision, organizing toys into specific categories and sending them down the right pathways.*

## ⭐ Task 1: How To Time Persist? (HTTP)

Presents are being packed and wrapped at *blazingly fast* speeds in the workshop.
In order to gather data on the production of presents, Santa needs a multi-stopwatch that can keep the time of many packet IDs at once.

Create two endpoints:

- POST `/12/save/<string>`: takes a string and stores it.
- GET `/12/load/<string>`: takes the same string and returns the number of whole seconds elapsed since the last time it was stored.

### 🔔 Tips

Find a suitable way to store data (pairs of strings and timestamps), either with an in-memory data structure or in a persistent fashion.

- [Sharing state in Axum](https://docs.rs/axum/latest/axum/#sharing-state-with-handlers)
- [Application State in Actix Web](https://actix.rs/docs/application#state)
- [State in Rocket](https://rocket.rs/v0.5/guide/state/)
- [Shuttle Persist](https://docs.shuttle.rs/resources/shuttle-persist)

### 💠 Example

```bash
curl -X POST http://localhost:8000/12/save/packet20231212
sleep 2
curl http://localhost:8000/12/load/packet20231212
echo
sleep 2
curl http://localhost:8000/12/load/packet20231212
echo
curl -X POST http://localhost:8000/12/save/packet20231212
curl http://localhost:8000/12/load/packet20231212

# After ~4 seconds:
2
4
0
```

## 🎁 Task 2: Unanimously Legendary IDentifier (ULID) (100 bonus points)

Santa, who likes old-school tech, now sees that some packets use modern ULIDs.
Help him rewind time a little bit by showing him them in an older format that he understands.

Make a POST endpoint `/12/ulids` that takes a JSON array of ULIDs.
Convert all the ULIDs to UUIDs and return a new array but in reverse order.

### 💠 Example

```bash
curl -X POST http://localhost:8000/12/ulids \
  -H 'Content-Type: application/json' \
  -d '[
    "01BJQ0E1C3Z56ABCD0E11HYX4M",
    "01BJQ0E1C3Z56ABCD0E11HYX5N",
    "01BJQ0E1C3Z56ABCD0E11HYX6Q",
    "01BJQ0E1C3Z56ABCD0E11HYX7R",
    "01BJQ0E1C3Z56ABCD0E11HYX8P"
  ]'

[
  "015cae07-0583-f94c-a5b1-a070431f7516",
  "015cae07-0583-f94c-a5b1-a070431f74f8",
  "015cae07-0583-f94c-a5b1-a070431f74d7",
  "015cae07-0583-f94c-a5b1-a070431f74b5",
  "015cae07-0583-f94c-a5b1-a070431f7494"
]
```

## 🎁 Task 3: Let Santa Broil (LSB) (200 bonus points)

Now that Santa is up to date on some newer data formats, he needs help with analyzing the manufacturing date of some packets he found in the corner of the workshop.

Create another variant of the same endpoint `/12/ulids/<weekday>` that counts the number of ULIDs that fulfill the following criteria (in the UTC timezone):

- How many of the ULIDs were generated on a Christmas Eve?
- How many were generated on a `<weekday>`? (A number in the path between 0 (Monday) and 6 (Sunday))
- How many were generated in the future? (has a date later than the current time)
- How many have entropy bits where the Least Significant Bit (LSB) is 1?

### 💠 Example

```bash
curl -X POST http://localhost:8000/12/ulids/5 \
  -H 'Content-Type: application/json' \
  -d '[
    "00WEGGF0G0J5HEYXS3D7RWZGV8",
    "76EP4G39R8JD1N8AQNYDVJBRCF",
    "018CJ7KMG0051CDCS3B7BFJ3AK",
    "00Y986KPG0AMGB78RD45E9109K",
    "010451HTG0NYWMPWCEXG6AJ8F2",
    "01HH9SJEG0KY16H81S3N1BMXM4",
    "01HH9SJEG0P9M22Z9VGHH9C8CX",
    "017F8YY0G0NQA16HHC2QT5JD6X",
    "03QCPC7P003V1NND3B3QJW72QJ"
  ]'

{
  "christmas eve": 3,
  "weekday": 1,
  "in the future": 2,
  "LSB is 1": 5
}
```

---

Authors: [orhun](https://github.com/orhun), [jonaro00](https://github.com/jonaro00)
