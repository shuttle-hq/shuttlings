# 🎄 Day 18: Santa's Gift Orders: Data Analytics Edition

*Santa sat back in his plush seat, a mug of hot cocoa in his hand, and a smile on his jolly face. The database upgrade from the previous week had indeed worked out exceptionally well; the operations were running smoother than ever, the reports were accurate, and morale among his helpers was at an all-time high. This modern marvel of technology had infused a new spirit into the North Pole operations.*

## ⭐ Task 1: Mr. Worldwide

*This challenge continues from what was built for the Core tasks on Day 13.*

Santa is stoked about the speed and reliability of the new gift order database backend!
He wants you to expand it to support per-region analytics.

Copy the `/13/reset` endpoint from Day 13 to `/18/reset`, but modify the query like this:

```sql
DROP TABLE IF EXISTS regions;
DROP TABLE IF EXISTS orders;

CREATE TABLE regions (
  id INT PRIMARY KEY,
  name VARCHAR(50)
);

CREATE TABLE orders (
  id INT PRIMARY KEY,
  region_id INT,
  gift_name VARCHAR(50),
  quantity INT
);
```

We want to re-use the POST endpoint `/13/orders` at `/18/orders` for adding new orders.
You can either add the same handler under the new route, or just copy+paste the entire thing, as long as both endpoints are doing the same thing.

Now, add a POST endpoint `/18/regions` that inserts regions in the same way the orders endpoint does.

Lastly, add a GET endpoint `/18/regions/total` that returns the total number of orders per region.
To make it easier for Santa to find a location, the output should be alphabetically sorted on the region name.
Regions with no orders should not be listed in the result.

### 🔔 Tips

You can `JOIN` the tables and use `GROUP BY` and `SUM()`.

### 💠 Example

```bash
curl -X POST http://localhost:8000/18/reset
curl -X POST http://localhost:8000/18/regions \
  -H 'Content-Type: application/json' \
  -d '[
    {"id":1,"name":"North Pole"},
    {"id":2,"name":"Europe"},
    {"id":3,"name":"North America"},
    {"id":4,"name":"South America"},
    {"id":5,"name":"Africa"},
    {"id":6,"name":"Asia"},
    {"id":7,"name":"Oceania"}
  ]'
curl -X POST http://localhost:8000/18/orders \
  -H 'Content-Type: application/json' \
  -d '[
    {"id":1,"region_id":2,"gift_name":"Board Game","quantity":5},
    {"id":2,"region_id":2,"gift_name":"Origami Set","quantity":8},
    {"id":3,"region_id":3,"gift_name":"Action Figure","quantity":12},
    {"id":4,"region_id":4,"gift_name":"Teddy Bear","quantity":10},
    {"id":5,"region_id":2,"gift_name":"Yarn Ball","quantity":6},
    {"id":6,"region_id":3,"gift_name":"Art Set","quantity":3},
    {"id":7,"region_id":5,"gift_name":"Robot Lego Kit","quantity":5},
    {"id":8,"region_id":6,"gift_name":"Drone","quantity":9}
  ]'
curl http://localhost:8000/18/regions/total

[
  {"region":"Africa","total":5},
  {"region":"Asia","total":9},
  {"region":"Europe","total":19},
  {"region":"North America","total":15},
  {"region":"South America","total":10}
]
```

## 🎁 Task 2: West Pole to East Pole - Santa wants ALL the data (600 bonus points)

To optimize production of gifts for next year, Santa needs detailed insights into the best performing gifts in every region.

Create a GET endpoint `/18/regions/top_list/<number>` that retrieves the names of the regions along with the top `<number>` most ordered gifts in each region, considering the quantity of orders placed for each gift.

If there are less than `<number>` unique gifts in a region, the top list will be shorter.
If there are no gifts in a region, show that with an empty top list.

If there is a tie among gifts, use alphabetical ordering of the gift name to break it.
The final output shall once again be ordered by region name.

### 💠 Example

```bash
curl -X POST http://localhost:8000/18/reset
curl -X POST http://localhost:8000/18/regions \
  -H 'Content-Type: application/json' \
  -d '[
    {"id":1,"name":"North Pole"},
    {"id":2,"name":"South Pole"},
    {"id":3,"name":"Kiribati"},
    {"id":4,"name":"Baker Island"}
  ]'
curl -X POST http://localhost:8000/18/orders \
  -H 'Content-Type: application/json' \
  -d '[
    {"id":1,"region_id":2,"gift_name":"Toy Train","quantity":5},
    {"id":2,"region_id":2,"gift_name":"Toy Train","quantity":3},
    {"id":3,"region_id":2,"gift_name":"Doll","quantity":8},
    {"id":4,"region_id":3,"gift_name":"Toy Train","quantity":3},
    {"id":5,"region_id":2,"gift_name":"Teddy Bear","quantity":6},
    {"id":6,"region_id":3,"gift_name":"Action Figure","quantity":12},
    {"id":7,"region_id":4,"gift_name":"Board Game","quantity":10},
    {"id":8,"region_id":3,"gift_name":"Teddy Bear","quantity":1},
    {"id":9,"region_id":3,"gift_name":"Teddy Bear","quantity":2}
  ]'
curl http://localhost:8000/18/regions/top_list/2

[
  {"region":"Baker Island","top_gifts":["Board Game"]},
  {"region":"Kiribati","top_gifts":["Action Figure","Teddy Bear"]},
  {"region":"North Pole","top_gifts":[]},
  {"region":"South Pole","top_gifts":["Doll","Toy Train"]}
]
```

---

Authors: [jonaro00](https://github.com/jonaro00), [orhun](https://github.com/orhun)
