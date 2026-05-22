# frog-api-wrapper

A lightweight Rust proxy server that exposes an **OpenAI-compatible** `/v1/chat/completions` endpoint locally and forwards every request to the [Frog API](https://frogapi.app).

This lets you point any OpenAI SDK client (Python, JavaScript, etc.) or plain HTTP tool at `http://127.0.0.1:3000` and have it transparently use Frog API under the hood — without the client needing to know your Frog API key.

---

## How it works

```
Your client
    │  POST /v1/chat/completions  (OpenAI-compatible JSON)
    ▼
frog-api-wrapper  (localhost:3000)
    │  POST https://frogapi.app/v1/chat/completions
    │  Authorization: Bearer <FROG_API_KEY>
    ▼
Frog API  →  response passed back unchanged
```

Both standard (buffered) and streaming (`"stream": true`) responses are supported.

---

## Requirements

- Rust 1.85+ (edition 2024)
- A Frog API key from [https://frogapi.app](https://frogapi.app)

---

## Setup

### 1. Clone and enter the project

```bash
git clone <repo-url>
cd frog-api-wrapper
```

### 2. Configure environment variables

Copy the example file and fill in your API key:

```bash
# Linux / macOS
cp .env.example .env

# Windows (Command Prompt)
copy .env.example .env
```

Open `.env` and set at minimum:

```
FROG_API_KEY=frog_sk_your_api_key_here
```

All available variables:

| Variable        | Default                    | Description                        |
|-----------------|----------------------------|------------------------------------|
| `FROG_API_KEY`  | *(required)*               | Your Frog API key                  |
| `FROG_BASE_URL` | `https://frogapi.app/v1`   | Upstream Frog API base URL         |
| `HOST`          | `127.0.0.1`                | Local bind address                 |
| `PORT`          | `3000`                     | Local bind port                    |
| `RUST_LOG`      | `frog_api_wrapper=info`    | Log level filter                   |

### 3. Build and run

```bash
cargo run
```

You should see:

```
INFO frog_api_wrapper: frog-api-wrapper listening on http://127.0.0.1:3000  →  upstream: https://frogapi.app/v1
```

---

## API endpoint

### `POST /v1/chat/completions`

Accepts the standard OpenAI chat completions request body and returns the standard OpenAI chat completions response.

**Request body (JSON):**

```json
{
  "model": "gpt-5.4",
  "messages": [
    { "role": "user", "content": "Hello, frogAPI!" }
  ]
}
```

**Response** (`choices[0].message.content` contains the reply):

```json
{
  "id": "chatcmpl-...",
  "object": "chat.completion",
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop",
      "index": 0
    }
  ],
  ...
}
```

---

## Testing with curl

### Linux / macOS

```bash
curl http://127.0.0.1:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-5.4","messages":[{"role":"user","content":"Hello, frogAPI!"}]}'
```

Streaming:

```bash
curl http://127.0.0.1:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-5.4","stream":true,"messages":[{"role":"user","content":"Hello, frogAPI!"}]}'
```

### Windows (Command Prompt)

```bat
curl http://127.0.0.1:3000/v1/chat/completions ^
  -H "Content-Type: application/json" ^
  -d "{\"model\":\"gpt-5.4\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello, frogAPI!\"}]}"
```

Streaming:

```bat
curl http://127.0.0.1:3000/v1/chat/completions ^
  -H "Content-Type: application/json" ^
  -d "{\"model\":\"gpt-5.4\",\"stream\":true,\"messages\":[{\"role\":\"user\",\"content\":\"Hello, frogAPI!\"}]}"
```

### Windows (PowerShell)

> PowerShell strips single quotes when passing arguments to external executables. Assign the JSON body to a variable first.

**Using `curl.exe` (store body in a variable):**

```powershell
$body = '{"model":"gpt-5.4","messages":[{"role":"user","content":"Hello, frogAPI!"}]}'
curl.exe http://127.0.0.1:3000/v1/chat/completions -H "Content-Type: application/json" -d $body
```

Streaming:

```powershell
$body = '{"model":"gpt-5.4","stream":true,"messages":[{"role":"user","content":"Hello, frogAPI!"}]}'
curl.exe http://127.0.0.1:3000/v1/chat/completions -H "Content-Type: application/json" -d $body
```

**Using `Invoke-RestMethod` (native PowerShell alternative):**

> `Invoke-RestMethod` truncates nested objects in its default output. Store the result in a variable and access the content directly.

```powershell
$response = Invoke-RestMethod -Method POST `
  -Uri "http://127.0.0.1:3000/v1/chat/completions" `
  -ContentType "application/json" `
  -Body '{"model":"gpt-5.4","messages":[{"role":"user","content":"Hello, frogAPI!"}]}'

$response.choices[0].message.content
```

---

## Using with OpenAI SDKs

Point any OpenAI SDK at the local wrapper instead of `api.openai.com`. The API key passed to the SDK is ignored — the wrapper uses `FROG_API_KEY` from its own environment.

**Python:**

```python
from openai import OpenAI

client = OpenAI(
    api_key="ignored",          # wrapper handles auth
    base_url="http://127.0.0.1:3000/v1"
)

response = client.chat.completions.create(
    model="gpt-5.4",
    messages=[{"role": "user", "content": "Hello, frogAPI!"}]
)

print(response.choices[0].message.content)
```

**JavaScript / TypeScript:**

```javascript
import OpenAI from "openai";

const client = new OpenAI({
  apiKey: "ignored",
  baseURL: "http://127.0.0.1:3000/v1",
});

const response = await client.chat.completions.create({
  model: "gpt-5.4",
  messages: [{ role: "user", content: "Hello, frogAPI!" }],
});

console.log(response.choices[0].message.content);
```
