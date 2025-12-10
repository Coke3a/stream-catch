/**
 * OLD flow (legacy streaming proxy):
 * 1) Client requests /.../recording-<uuid>.mp4?token=<jwt>
 * 2) Worker parses path + recording id, verifies JWT (HMAC SHA-256 with WATCH_URL_JWT_SECRET)
 * 3) Worker builds Wasabi URL and SigV4 headers, fetches Wasabi, and streams the multi-GB body back
 * 4) Worker sets Cache-Control: public, max-age=3600
 *    => Worker sits in the data path; large (10–15GB+) files stress the Worker and limit scalability.
 *
 * NEW flow (control plane only):
 * 1) Worker still validates path/JWT exactly the same and builds the canonical Wasabi key
 * 2) Worker signs a presigned URL (AWS SigV4, unsigned payload) for GET/HEAD
 * 3) Worker responds with a 307 redirect to the presigned URL + Cache-Control: public, max-age=3600
 *    => Video bytes flow directly between client/Cloudflare POP and Wasabi; Worker no longer streams payloads.
 *    => Token is used only for auth; the signed URL omits the token so caching keys remain per recording path.
 */
const enc = new TextEncoder();
const dec = new TextDecoder();
const AWS_ALGORITHM = "AWS4-HMAC-SHA256";
let cachedJwtKey = null;
let cachedJwtSecret = null;

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
  const key = await getJwtKey(secret);

  const valid = await crypto.subtle.verify("HMAC", key, signature, data);
  if (!valid) {
    throw new Error("Invalid token signature");
  }

  const payloadJson = dec.decode(decodeBase64Url(payloadB64));
  const payload = JSON.parse(payloadJson);

  const now = Math.floor(Date.now() / 1000);
  if (payload.exp && payload.exp < now) {
    throw new Error("Token expired");
  }

  return payload;
}

export default {
  async fetch(request, env, ctx) {
    // Control-plane only: validate token + path, then hand off via presigned URL.
    // Keeps existing security/env contracts while removing the Worker from the data path.
    const url = new URL(request.url);
    const bucketName = env.VIDEO_STORAGE_S3_BUCKET;
    const endpoint = env.VIDEO_STORAGE_S3_ENDPOINT;
    const region = env.VIDEO_STORAGE_S3_REGION;
    const accessKeyId = env.VIDEO_STORAGE_S3_ACCESS_KEY_ID;
    const secretAccessKey = env.VIDEO_STORAGE_S3_SECRET_ACCESS_KEY;
    const keyPrefix = normalizeKeyPrefix(env.VIDEO_STORAGE_S3_KEY_PREFIX || "recordings");

    if (!bucketName || !endpoint || !region || !accessKeyId || !secretAccessKey) {
      return new Response("Video storage is not configured", { status: 500 });
    }

    let endpointUrl;
    try {
      endpointUrl = new URL(endpoint);
    } catch (err) {
      console.error("Invalid VIDEO_STORAGE_S3_ENDPOINT:", err);
      return new Response("Server misconfigured", { status: 500 });
    }

    let objectPath = url.pathname.replace(/^\/+/, "");
    if (!objectPath) {
      return new Response("Please specify a file path", { status: 400 });
    }

    const objectKey = `${keyPrefix}${objectPath}`;

    const token = url.searchParams.get("token");
    if (!token) {
      return new Response("Missing token", { status: 401 });
    }

    const jwtSecret = env.WATCH_URL_JWT_SECRET;
    if (!jwtSecret) {
      return new Response("Server misconfigured", { status: 500 });
    }

    const fileName = objectPath.split("/").pop();
    const match = fileName?.match(/^recording-([0-9a-fA-F-]+)(?:_[^./]+)?\.mp4$/);
    const recordingIdFromPath = match ? match[1] : null;
    if (!recordingIdFromPath) {
      return new Response("Invalid recording path", { status: 400 });
    }

    try {
      const claims = await verifyJwt(token, jwtSecret);
      const tokenRecordingId = `${claims.sub || ""}`.toLowerCase();
      const pathRecordingId = recordingIdFromPath.toLowerCase();
      if (tokenRecordingId !== pathRecordingId) {
        console.warn("Token recording mismatch", { pathRecordingId, tokenRecordingId, objectPath });
        return new Response("Token not valid for this recording", { status: 403 });
      }
    } catch (err) {
      console.error("Token verification failed:", err);
      return new Response("Invalid token", { status: 401 });
    }

    // Strip token from downstream query; cache/cdn keys stay tied to the recording path/key, not the token.
    const params = new URLSearchParams(url.searchParams);
    params.delete("token");

    const canonicalUri = buildCanonicalUri(bucketName, objectKey);
    const method = request.method === "HEAD" ? "HEAD" : "GET";
    // Presigned URL carries only SigV4 params (no JWT), so Cloudflare/WASABI caching can key by object path.
    const presignedUrl = await buildPresignedUrl({
      method,
      endpointUrl,
      canonicalUri,
      baseQueryParams: params,
      region,
      accessKeyId,
      secretAccessKey,
      // Align presign lifetime with cache header so cached redirects don't outlive the signature.
      expiresSeconds: 3600,
    });

    console.info("Redirecting to Wasabi presigned URL (private)", {
      canonicalUri,
      method,
    });

    // 307 preserves the original method/headers (including Range) for direct Wasabi fetches.
    // Construct fresh response so headers are mutable.
    return new Response(null, {
      status: 307,
      headers: {
        Location: presignedUrl,
        "Cache-Control": "public, max-age=3600",
      },
    });
  },
};

async function getJwtKey(secret) {
  if (cachedJwtKey && cachedJwtSecret === secret) {
    return cachedJwtKey;
  }
  const keyData = enc.encode(secret);
  const key = await crypto.subtle.importKey(
    "raw",
    keyData,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["verify"]
  );
  cachedJwtKey = key;
  cachedJwtSecret = secret;
  return key;
}

function normalizeKeyPrefix(prefix) {
  const trimmed = prefix.replace(/^\/+/, "").replace(/\/+$/, "");
  if (!trimmed) {
    return "";
  }
  return `${trimmed}/`;
}

function canonicalizeQuery(params) {
  const grouped = new Map();
  for (const [key, value] of params.entries()) {
    if (!grouped.has(key)) {
      grouped.set(key, []);
    }
    grouped.get(key).push(value);
  }
  const parts = [];
  const sortedKeys = Array.from(grouped.keys()).sort();
  for (const key of sortedKeys) {
    const values = grouped.get(key).sort();
    for (const value of values) {
      parts.push(`${encodeRfc3986(key)}=${encodeRfc3986(value)}`);
    }
  }
  return parts.join("&");
}

function buildCanonicalUri(bucket, objectKey) {
  const segments = [bucket, ...objectKey.split("/")].filter((segment) => segment.length > 0);
  const encoded = segments.map((segment) => encodeRfc3986(segment));
  return `/${encoded.join("/")}`;
}

function encodeRfc3986(value) {
  return encodeURIComponent(value).replace(/[!'()*]/g, (char) =>
    `%${char.charCodeAt(0).toString(16).toUpperCase()}`
  );
}

async function buildPresignedUrl({
  method,
  endpointUrl,
  canonicalUri,
  baseQueryParams,
  region,
  accessKeyId,
  secretAccessKey,
  expiresSeconds = 900,
}) {
  const payloadHash = "UNSIGNED-PAYLOAD";
  const now = new Date();
  const amzDate = toAmzDate(now);
  const dateStamp = amzDate.slice(0, 8);
  const credentialScope = `${dateStamp}/${region}/s3/aws4_request`;

  // Copy caller params to avoid mutating the original URLSearchParams.
  const params = new URLSearchParams(baseQueryParams || undefined);

  const { canonicalHeaders, signedHeaders } = canonicalizeHeaders({
    host: endpointUrl.host,
  });

  params.set("X-Amz-Algorithm", AWS_ALGORITHM);
  params.set("X-Amz-Credential", `${accessKeyId}/${credentialScope}`);
  params.set("X-Amz-Date", amzDate);
  params.set("X-Amz-Expires", `${expiresSeconds}`);
  params.set("X-Amz-SignedHeaders", signedHeaders);

  const canonicalQueryString = canonicalizeQuery(params);

  const canonicalRequest = [
    method,
    canonicalUri,
    canonicalQueryString,
    canonicalHeaders,
    signedHeaders,
    payloadHash,
  ].join("\n");

  const hashedCanonicalRequest = await sha256Hex(canonicalRequest);
  const stringToSign = [AWS_ALGORITHM, amzDate, credentialScope, hashedCanonicalRequest].join("\n");
  const signingKey = await getSignatureKey(secretAccessKey, dateStamp, region, "s3");
  const signature = await hmacHex(signingKey, stringToSign);

  params.set("X-Amz-Signature", signature);
  const finalQueryString = canonicalizeQuery(params);

  return `${endpointUrl.origin}${canonicalUri}${finalQueryString ? `?${finalQueryString}` : ""}`;
}

async function signS3Request({
  method,
  canonicalUri,
  canonicalQueryString,
  host,
  region,
  accessKeyId,
  secretAccessKey,
  rangeHeader,
}) {
  const payloadHash = "UNSIGNED-PAYLOAD";
  const now = new Date();
  const amzDate = toAmzDate(now);
  const dateStamp = amzDate.slice(0, 8);

  const headerMap = {
    host,
    "x-amz-content-sha256": payloadHash,
    "x-amz-date": amzDate,
  };

  if (rangeHeader) {
    headerMap.range = rangeHeader.trim();
  }

  const { canonicalHeaders, signedHeaders } = canonicalizeHeaders(headerMap);

  const canonicalRequest = [
    method,
    canonicalUri,
    canonicalQueryString,
    canonicalHeaders,
    signedHeaders,
    payloadHash,
  ].join("\n");

  const hashedCanonicalRequest = await sha256Hex(canonicalRequest);
  const credentialScope = `${dateStamp}/${region}/s3/aws4_request`;
  const stringToSign = [
    AWS_ALGORITHM,
    amzDate,
    credentialScope,
    hashedCanonicalRequest,
  ].join("\n");
  const signingKey = await getSignatureKey(secretAccessKey, dateStamp, region, "s3");
  const signature = await hmacHex(signingKey, stringToSign);

  const authorization = `${AWS_ALGORITHM} Credential=${accessKeyId}/${credentialScope}, SignedHeaders=${signedHeaders}, Signature=${signature}`;

  return {
    authorization,
    amzDate,
    payloadHash,
  };
}

function canonicalizeHeaders(headerMap) {
  const entries = Object.entries(headerMap).map(([name, value]) => [
    name.toLowerCase(),
    value.replace(/\s+/g, " ").trim(),
  ]);
  entries.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));

  const canonicalHeaders = entries.map(([name, value]) => `${name}:${value}\n`).join("");
  const signedHeaders = entries.map(([name]) => name).join(";");

  return { canonicalHeaders, signedHeaders };
}

function toAmzDate(date) {
  const yyyy = date.getUTCFullYear().toString();
  const mm = pad(date.getUTCMonth() + 1);
  const dd = pad(date.getUTCDate());
  const hh = pad(date.getUTCHours());
  const min = pad(date.getUTCMinutes());
  const ss = pad(date.getUTCSeconds());
  return `${yyyy}${mm}${dd}T${hh}${min}${ss}Z`;
}

function pad(value) {
  return value.toString().padStart(2, "0");
}

async function sha256Hex(value) {
  const data = enc.encode(value);
  const digest = await crypto.subtle.digest("SHA-256", data);
  return toHex(new Uint8Array(digest));
}

async function hmacHex(keyBytes, data) {
  const sig = await hmacSign(keyBytes, enc.encode(data));
  return toHex(sig);
}

async function hmacSign(keyBytes, dataBytes) {
  const cryptoKey = await crypto.subtle.importKey(
    "raw",
    keyBytes,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );
  const signature = await crypto.subtle.sign("HMAC", cryptoKey, dataBytes);
  return new Uint8Array(signature);
}

async function getSignatureKey(secretAccessKey, dateStamp, regionName, serviceName) {
  const kSecret = enc.encode(`AWS4${secretAccessKey}`);
  const kDate = await hmacSign(kSecret, enc.encode(dateStamp));
  const kRegion = await hmacSign(kDate, enc.encode(regionName));
  const kService = await hmacSign(kRegion, enc.encode(serviceName));
  return hmacSign(kService, enc.encode("aws4_request"));
}

function toHex(buffer) {
  return Array.from(buffer)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}
