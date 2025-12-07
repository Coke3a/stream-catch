let b2State = {
  apiUrl: null,
  authToken: null,
};

const enc = new TextEncoder();

function decodeBase64Url(input) {
  // Pad base64 string
  const pad = input.length % 4 === 0 ? "" : "=".repeat(4 - (input.length % 4));
  const b64 = input.replace(/-/g, "+").replace(/_/g, "/") + pad;
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

async function verifyJwt(token, secret) {
  const parts = token.split(".");
  if (parts.length !== 3) {
    throw new Error("Invalid token format");
  }

  const [headerB64, payloadB64, signatureB64] = parts;
  const data = enc.encode(`${headerB64}.${payloadB64}`);
  const signature = decodeBase64Url(signatureB64);
  const keyData = enc.encode(secret);

  const key = await crypto.subtle.importKey(
    "raw",
    keyData,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["verify"]
  );

  const valid = await crypto.subtle.verify("HMAC", key, signature, data);
  if (!valid) {
    throw new Error("Invalid token signature");
  }

  const payloadJson = new TextDecoder().decode(decodeBase64Url(payloadB64));
  const payload = JSON.parse(payloadJson);

  const now = Math.floor(Date.now() / 1000);
  if (payload.exp && payload.exp < now) {
    throw new Error("Token expired");
  }

  return payload;
}

async function authorizeB2(env) {
  if (b2State.apiUrl && b2State.authToken) {
    return;
  }

  const keyId = env.B2_KEY_ID;
  const appKey = env.B2_APP_KEY;

  if (!keyId || !appKey) {
    throw new Error("B2_KEY_ID or B2_APP_KEY is not set in Worker env");
  }

  const basicAuth = btoa(`${keyId}:${appKey}`);

  const resp = await fetch("https://api.backblazeb2.com/b2api/v2/b2_authorize_account", {
    method: "GET",
    headers: {
      Authorization: `Basic ${basicAuth}`,
    },
  });

  if (!resp.ok) {
    throw new Error(`b2_authorize_account failed: ${resp.status} ${resp.statusText}`);
  }

  const data = await resp.json();

  b2State.apiUrl = data.apiUrl;
  b2State.authToken = data.authorizationToken;
}

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const bucketName = env.B2_BUCKET_NAME;

    if (!bucketName) {
      return new Response("B2_BUCKET_NAME is not set", { status: 500 });
    }

    let objectPath = url.pathname.replace(/^\/+/, "");
    if (!objectPath) {
      return new Response("Please specify a file path", { status: 400 });
    }

    const b2Path = `recordings/${objectPath}`;

    const token = url.searchParams.get("token");
    if (!token) {
      return new Response("Missing token", { status: 401 });
    }

    const jwtSecret = env.WATCH_URL_JWT_SECRET;
    if (!jwtSecret) {
      return new Response("Server misconfigured", { status: 500 });
    }

    const match = objectPath.match(/^recording-(.+)\.mp4$/);
    const recordingIdFromPath = match ? match[1] : null;
    if (!recordingIdFromPath) {
      return new Response("Invalid recording path", { status: 400 });
    }

    try {
      const claims = await verifyJwt(token, jwtSecret);
      if (claims.sub !== recordingIdFromPath) {
        return new Response("Token not valid for this recording", { status: 403 });
      }
    } catch (err) {
      console.error("Token verification failed:", err);
      return new Response("Invalid token", { status: 401 });
    }

    try {
      await authorizeB2(env);
    } catch (e) {
      console.error("B2 authorize failed:", e);
      return new Response("Internal error authorizing B2", { status: 500 });
    }

    const params = new URLSearchParams(url.searchParams);
    params.delete("token");
    const search = params.toString();

    const originUrl = `${b2State.apiUrl}/file/${bucketName}/${b2Path}${
      search ? `?${search}` : ""
    }`;
    console.info("Proxy to B2 (private):", originUrl);

    // 🔸 forward Range + method ไป B2
    const originHeaders = new Headers();
    originHeaders.set("Authorization", b2State.authToken);

    const range = request.headers.get("Range");
    if (range) {
      originHeaders.set("Range", range);
    }

    const method = request.method === "HEAD" ? "HEAD" : "GET";

    let originResp = await fetch(originUrl, {
      method,
      headers: originHeaders,
      redirect: "follow",
    });

    if (originResp.status === 401 || originResp.status === 403) {
      console.warn("B2 token possibly expired, re-authorizing...");
      b2State.apiUrl = null;
      b2State.authToken = null;
      try {
        await authorizeB2(env);
      } catch (e) {
        console.error("Re-authorize B2 failed:", e);
        return new Response("Internal error re-authorizing B2", { status: 500 });
      }
      originResp = await fetch(originUrl, {
        method,
        headers: originHeaders,
        redirect: "follow",
      });
    }

    if (!originResp.ok) {
      return new Response(
        `B2 responded with ${originResp.status} ${originResp.statusText}`,
        { status: originResp.status }
      );
    }

    const resp = new Response(originResp.body, originResp);
    resp.headers.set("Cache-Control", "public, max-age=3600");

    return resp;
  },
};
