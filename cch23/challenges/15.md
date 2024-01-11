# 🎄 Day 15: The Password Validator

*There had been a few naughty incidents where mischievous users had tinkered with other children's wish lists, not respecting the goodwill and spirit of the platform. It was clear: the website needed to add a layer of security to protect the integrity of each child's wish list.*

*The team behind the scenes, a dedicated crew of Santa's tech-savvy elves (led by you), rolled up their sleeves. They decided to implement a homemade password validation system that ensured that no Grinch could easily guess or crack the password.*

## ⭐ Task 1: Naughty or Nice Strings

Now that children can sign up to the letter sending website, we need an endpoint for validating their passwords.

Create an endpoint at `/15/nice` that accepts POST requests with a JSON payload containing a password string to validate.

The rules at this endpoint are:

- **Nice Strings**: Must contain at least three vowels (`aeiouy`), at least one letter that appears twice in a row, and must not contain the substrings: `ab`, `cd`, `pq`, or `xy`.
- **Naughty Strings**: Do not meet the criteria for nice strings.

Return an appropriate HTTP status code and message (see below) indicating whether the provided string is nice or naughty.

### 🔔 Tips

Use Rust's string and char methods or [regex](https://crates.io/crates/regex) for validation.

### 💠 Examples

```bash
curl -X POST http://localhost:8000/15/nice \
  -H 'Content-Type: application/json' \
  -d '{"input": "hello there"}'

# 200 OK
{"result":"nice"}
```

```bash
curl -X POST http://localhost:8000/15/nice \
  -H 'Content-Type: application/json' \
  -d '{"input": "abcd"}'

# 400 Bad Request
{"result":"naughty"}
```

```bash
curl -X POST http://localhost:8000/15/nice \
  -H 'Content-Type: application/json' \
  -d '{Grinch? GRINCH!}'

# 400 Bad Request
# response body does not matter
```

---

## 🎁 Task 2: Game of the Year (400 bonus points)

Santa thought this validation thing was so fun that it could be turned into a game!

Add a similar endpoint, POST `/15/game`, that has this set of rules:

- **Nice Strings**: Must adhere to all the rules:
  - Rule 1: must be at least 8 characters long
  - Rule 2: must contain uppercase letters, lowercase letters, and digits
  - Rule 3: must contain at least 5 digits
  - Rule 4: all *integers* (sequences of consecutive digits) in the string must add up to 2023
  - Rule 5: must contain the letters `j`, `o`, and `y` in that order and in no other order
  - Rule 6: must contain a letter that repeats with exactly one other letter between them (like `xyx`)
  - Rule 7: must contain at least one unicode character in the range `[U+2980, U+2BFF]`
  - Rule 8: must contain at least one emoji
  - Rule 9: the hexadecimal representation of the sha256 hash of the string must end with an `a`
- **Naughty Strings**: Do not meet the criteria for nice strings.

Check the string for the rules in the listed order. Return the corresponding status code and reason (and naughty/nice result) based on which rule was violated:

| Rule broken | Status Code | Reason |
|-------------|-------------|--------|
| 1 | 400 | `8 chars` |
| 2 | 400 | `more types of chars` |
| 3 | 400 | `55555` |
| 4 | 400 | `math is hard` |
| 5 | 406 | `not joyful enough` |
| 6 | 451 | `illegal: no sandwich` |
| 7 | 416 | `outranged` |
| 8 | 426 | `😳` |
| 9 | 418 | `not a coffee brewer` |
| None | 200 | `that's a nice password` |

### 💠 Examples

```bash
curl -X POST http://localhost:8000/15/game \
  -H 'Content-Type: application/json' \
  -d '{"input": "password"}'

# 400 Bad Request
{"result":"naughty","reason":"more types of chars"}
```

```bash
curl -X POST http://localhost:8000/15/game \
  -H 'Content-Type: application/json' \
  -d '{"input": "Password12345"}'

# 400 Bad Request
{"result":"naughty","reason":"math is hard"}
```

```bash
curl -X POST http://localhost:8000/15/game \
  -H 'Content-Type: application/json' \
  -d '{"input": "23jPassword2000y"}'

# 451 Unavailable For Legal Reasons
{"result":"naughty","reason":"illegal: no sandwich"}
```

---

Authors: [orhun](https://github.com/orhun), [jonaro00](https://github.com/jonaro00)
