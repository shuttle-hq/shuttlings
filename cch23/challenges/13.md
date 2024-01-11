# 🎄 Day 13: Santa's Gift Orders

*Santa Claus has started facing a pressing issue at the North Pole. The existing database, written in a legacy language, is becoming insufficient for handling the tidal wave of gift requests from children worldwide. This ancient system is not only slowing down operations, but it is also proving harder to maintain.*

*To ensure that not a single child's wish is overlooked and operations run as efficiently as possible, an immediate upgrade is a necessity.*

## ⭐ Task 1: SQL? Sequel? Squeel??

Santa's gift order database is written in an ancient language and needs to be oxidized.
Let's show him the power of Rust with your backend combined with a Postgres database.

Add a Postgres database with the [Shuttle Shared Database](https://docs.shuttle.rs/resources/shuttle-shared-db) plugin, and add the pool to your application state.
Add a GET endpoint `/13/sql` that executes the SQL query `SELECT 20231213` and responds with the query result (an `i32` turned into a string).

### 🔔 Tips

- [sqlx](https://docs.rs/sqlx/latest/sqlx/)
- [Shuttle Examples: Axum Postgres](https://github.com/shuttle-hq/shuttle-examples/tree/main/axum/postgres)
- [Shuttle Examples: Actix Web Postgres](https://github.com/shuttle-hq/shuttle-examples/tree/main/actix-web/postgres)
- [Shuttle Examples: Rocket Postgres](https://github.com/shuttle-hq/shuttle-examples/tree/main/rocket/postgres)

### 💠 Example

```bash
curl http://localhost:8000/13/sql

20231213
```

## ⭐ Task 2: Use code NorthPole2023 for 2023% off???

Now that the data can be migrated over to the new database, we see that Santa's workshop has received numerous gift orders from different regions. Time to do some basic analysis.

Create a POST endpoint `/13/reset` that (re-)creates the following schema in your database upon being called, and returns a plain `200 OK`.
It will be used at the start of each test to ensure a clean starting point.

```sql
DROP TABLE IF EXISTS orders;
CREATE TABLE orders (
  id INT PRIMARY KEY,
  region_id INT,
  gift_name VARCHAR(50),
  quantity INT
);
```

Then, create a POST endpoint `/13/orders` that takes a JSON array of order objects and inserts them into the table (see below). Return a plain `200 OK`.

Lastly, create a GET endpoint `/13/orders/total` that queries the table and returns the total number of gifts ordered (the sum of all quantities).

### 🔔 Tips

Use a `SELECT` statement and the `SUM()` function. The result can be extracted as one row with an `i64` on the Rust side.

### 💠 Example

```bash
curl -X POST http://localhost:8000/13/reset
curl -X POST http://localhost:8000/13/orders \
  -H 'Content-Type: application/json' \
  -d '[
    {"id":1,"region_id":2,"gift_name":"Toy Train","quantity":5},
    {"id":2,"region_id":2,"gift_name":"Doll","quantity":8},
    {"id":3,"region_id":3,"gift_name":"Action Figure","quantity":12},
    {"id":4,"region_id":4,"gift_name":"Board Game","quantity":10},
    {"id":5,"region_id":2,"gift_name":"Teddy Bear","quantity":6},
    {"id":6,"region_id":3,"gift_name":"Toy Train","quantity":3}
  ]'
curl http://localhost:8000/13/orders/total

{"total":44}
```

## 🎁 Task 3: Truly one of the gifts of all time (100 bonus points)

Add a GET endpoint `/13/orders/popular` that returns the name of the most popular gift.
If there is no most popular gift, use `null` instead of a string.

```bash
curl -X POST http://localhost:8000/13/reset
curl -X POST http://localhost:8000/13/orders \
  -H 'Content-Type: application/json' \
  -d '[
    {"id":1,"region_id":2,"gift_name":"Toy Train","quantity":5},
    {"id":2,"region_id":2,"gift_name":"Doll","quantity":8},
    {"id":3,"region_id":3,"gift_name":"Toy Train","quantity":4}
  ]'
curl http://localhost:8000/13/orders/popular

{"popular":"Toy Train"}
```

---

Authors: [orhun](https://github.com/orhun), [jonaro00](https://github.com/jonaro00)
