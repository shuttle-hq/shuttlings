# 🎄 Day 5: Why did Santa's URL query go haywire on Christmas? Too many "present" parameters!

*In the technologically advanced North Pole, Santa decided to streamline his gift-tracking system using URL query parameters, entrusting the elves with entering present requests. However, the mischievous Grinch added duplicate parameters like "present=puzzle" and "present=unicorn" as a prank. On Christmas Eve, as Santa set out to deliver gifts, the excess parameters caused a glitch: the list of names entered an infinite loop.*

## ⭐ Task 1: Slicing the Loop

Santa has some lists of names that are becoming too long to deal with.
Help him by adding URL query parameters for paginating the list.

The task is to create a POST endpoint `/5` that takes a JSON list of names, and query parameters `offset` and `limit` as numbers.
Then, return the sub-slice of the list between index `offset` and `offset + limit`.

### 🔔 Tips

- [Query parameters in Axum](https://docs.rs/axum/latest/axum/extract/struct.Query.html)
- [Query strings in Actix Web](https://actix.rs/docs/extractors/)
- [Query strings in Rocket](https://rocket.rs/v0.5/guide/requests/#query-strings)

### 💠 Example

```bash
curl -X POST "http://localhost:8000/5?offset=3&limit=5" \
  -H 'Content-Type: application/json' \
  -d '[
    "Ava", "Caleb", "Mia", "Owen", "Lily", "Ethan", "Zoe",
    "Nolan", "Harper", "Lucas", "Stella", "Mason", "Olivia"
  ]'

["Owen", "Lily", "Ethan", "Zoe", "Nolan"]
```

---

## 🎁 Task 2: Time to Page Some Names (150 bonus points)

This time, Santa also needs to be able to get all pages at once.

Modify the same endpoint, so that it can also handle a `split` parameter.
All parameters should now be optional.
If not given, `offset` defaults to 0, and `limit` defaults to including all remaining items in the list.
If `split` is not given, no splitting will happen, but if given, the output list should be split into sub-lists with length according the the value.

### 💠 Example

```bash
curl -X POST http://localhost:8000/5?split=4 \
  -H 'Content-Type: application/json' \
  -d '[
    "Ava", "Caleb", "Mia", "Owen", "Lily", "Ethan", "Zoe",
    "Nolan", "Harper", "Lucas", "Stella", "Mason", "Olivia"
  ]'

[
  ["Ava", "Caleb", "Mia", "Owen"],
  ["Lily", "Ethan", "Zoe", "Nolan"],
  ["Harper", "Lucas", "Stella", "Mason"],
  ["Olivia"]
]
```

```bash
curl -X POST "http://localhost:8000/5?offset=5&split=2" \
  -H 'Content-Type: application/json' \
  -d '[
    "Ava", "Caleb", "Mia", "Owen", "Lily", "Ethan", "Zoe",
    "Nolan", "Harper", "Lucas", "Stella", "Mason", "Olivia"
  ]'

[
  ["Ethan", "Zoe"],
  ["Nolan", "Harper"],
  ["Lucas", "Stella"],
  ["Mason", "Olivia"]
]
```

---

Authors: [joshua-mo-143](https://github.com/joshua-mo-143), [jonaro00](https://github.com/jonaro00)
