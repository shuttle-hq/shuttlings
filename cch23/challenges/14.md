# 🎄 Day 14: Reindeering HTML

*Did you hear about the time when Santa became a web designer? He picked up coding with great enthusiasm. Each tag told a story, every element was a toy, and every attribute was a wish from a child around the world. He soon managed to build a website where children could easily send their letters filled with Christmas wishes, and the elves could more efficiently organize the toymaking process.*

## ⭐ Task 1: Ho-ho, Toymaking Magic Land! (HTML)

Today we are simulating an incident that happened shortly after Santa joined the web dev team at the North Pole.

Implement a POST endpoint `/14/unsafe` that takes some HTML content and *unsafely* renders it on a small HTML page.

### 🔔 Tips

If you choose to use a templating engine for this task, make sure you disable escaping to allow unsafe rendering.

### 💠 Example Input

```bash
curl -X POST http://localhost:8000/14/unsafe \
  -H "Content-Type: application/json" \
  -d '{"content": "<h1>Welcome to the North Pole!</h1>"}'
```

### 💠 Example Output

Make sure that no extra whitespace is rendered. The response content below is 124 bytes long.

```html
<html>
  <head>
    <title>CCH23 Day 14</title>
  </head>
  <body>
    <h1>Welcome to the North Pole!</h1>
  </body>
</html>
```

---

## 🎁 Task 2: Safety 2nd (100 bonus points)

Time to clean up the mess that Santa caused in Task 1.
Show him how it's done in `/14/safe` by securely rendering the HTML against script injection.

### 💠 Example Input

```bash
curl -X POST http://localhost:8000/14/safe \
  -H "Content-Type: application/json" \
  -d '{"content": "<script>alert(\"XSS Attack!\")</script>"}'
```

### 💠 Example Output

```html
<html>
  <head>
    <title>CCH23 Day 14</title>
  </head>
  <body>
    &lt;script&gt;alert(&quot;XSS Attack!&quot;)&lt;/script&gt;
  </body>
</html>
```

---

Authors: [orhun](https://github.com/orhun), [jonaro00](https://github.com/jonaro00)
