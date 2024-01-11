# 🎄 Day 21: Around the Globe

*Once upon a frosty night in Christmas' season, ol' Santa was tidying up his archives. With his rosy cheeks and a finer air of mystery, he stumbled upon a pile of old, dusty tape drives. Intrigued, he gave a mighty tug and dust flew in the air, making him sneeze in the most jolly way possible.*

*As he dusted them off, memories flooded back. Such mirth and jingle echoed in his mind. They were his old present delivery logs and routes, the ones he hadn't seen in years!*

## ⭐ Task 1: Flat Squares on a Round Sphere?

Santa found a bunch of old tape drives in the archives.
Reading their contents revealed a bunch of coordinates in a strange format encoded with ones and zeroes.
He needs some help with parsing them.

Make a GET endpoint `/21/coords/<binary>` that takes a `u64` in binary representation representing an S2 cell ID.
Return the cell's center coordinates in DMS format rounded to 3 decimals (see format below).

### 🔔 Tips

- [S2 Cells](http://s2geometry.io/devguide/s2cell_hierarchy)
- [Decimal degrees](https://en.wikipedia.org/wiki/Decimal_degrees)

### 💠 Examples

```bash
curl http://localhost:8000/21/coords/0100111110010011000110011001010101011111000010100011110001011011

83°39'54.324''N 30°37'40.584''W
```

```bash
curl http://localhost:8000/21/coords/0010000111110000011111100000111010111100000100111101111011000101

18°54'55.944''S 47°31'17.976''E
```

## 🎁 Task 2: Turbo-fast Country Lookup (300 bonus points)

When Santa rides his sleigh across the world, he crosses so many country borders that he sometimes forgets which country he is in.
He needs a handy little API for quickly checking where he has ended up.

Make a GET endpoint `/21/country/<binary>` with the same type of input as in Task 1, that returns the english name of the country that the corresponding coordinates are in.

The input is guaranteed to represent coordinates that are within one country's borders.

Hint for an API that *can* be used: *"In a tunnel? Closed. On a street? Open. In a tunnel? Slow. Passing over? Turbo."*

```bash
curl http://localhost:8000/21/country/0010000111110000011111100000111010111100000100111101111011000101

Madagascar
```

---

Author: [jonaro00](https://github.com/jonaro00)
