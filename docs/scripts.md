# Scripts

A rule can run a JavaScript file on the traffic it matches. On the Rules screen,
choose **Run a script**, give the URL pattern the rule matches, and the full path
of a `.js` file. Edit the file in any editor: tracewing reads it again when it
changes, from the next request on.

A script has `onRequest`, `onResponse`, or both:

```js
// Before the server gets it.
function onRequest(request) {
  request.headers["x-debug"] = "1";
  delete request.headers["cookie"];
}

// Before the app gets it.
function onResponse(request, response) {
  const body = JSON.parse(response.body);
  body.premium = true;
  response.body = JSON.stringify(body);
}
```

Each function changes what it is given in place, or returns it changed.

## What a script is given

A request:

```js
{
  method: "POST",
  url: "https://api.example.com/v1/items?page=2",
  headers: { "content-type": "application/json", "accept": "*/*" },
  body: "{\"name\":\"tea\"}"
}
```

A response:

```js
{ status: 200, headers: { "content-type": "application/json" }, body: "{...}" }
```

- **Headers** are keyed by lower-case name. Each value is a string, or an array
  of strings for a header sent more than once. Delete a key to remove the header.
  Headers left alone keep their place and their case.
- **A body** is text. A body that is not text (an image, Protobuf) is `null`. It
  goes on as it came unless the script sets one. A compressed body is given
  unpacked, and goes on unpacked.
- **`onResponse`** gets the request as it was sent, after `onRequest` changed it.

## Answering with no server asked

`onRequest` can return a response, and the server is never asked:

```js
function onRequest(request) {
  if (request.url.endsWith("/flags")) {
    return {
      response: {
        status: 200,
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ newCheckout: true })
      }
    };
  }
}
```

## Keeping something between calls

The global `state` object keeps whatever a script puts in it from one call to
the next. It starts empty again when the file changes.

```js
function onRequest(request) {
  state.count = (state.count || 0) + 1;
  request.headers["x-call"] = String(state.count);
}
```

## Seeing what a script does

`console.log` (and `console.info`, `warn`, `error`, `debug`) writes to the
script's log. The Rules screen shows the last line under the rule, with how many
times it ran. When a call fails, the rule shows why, for example
`TypeError: cannot read property 'x' of undefined`, with the line it came from.

A flow that a script changed names the rule in its **Rules** field. The flow
records the request as it went to the server.

## Limits

- A script runs in QuickJS, a small JavaScript engine (modern JavaScript, no
  Node or browser APIs: no `fetch`, no files, no timers).
- One call may take at most **one second** and **64 MiB**. One that takes more
  is stopped. Its traffic then goes on as it came, and the rule says why.
- A call that throws, or a script that does not parse, leaves traffic as it was.
- Bodies up to 16 MiB are given to scripts. A bigger one goes on with the
  script not run.
- Server-sent events, WebSocket messages and other streams are not given to
  scripts. A WebSocket's upgrade request is not either.
- Each script file runs on a thread of its own, one call at a time.
