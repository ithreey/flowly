/**
 * 解析 cURL 命令字符串为请求对象。
 * @param {string} text - cURL 命令文本
 * @returns {{ method: string, url: string, headers: Array<{key:string, value:string, enabled:boolean}>, bodyType: string, body: string } | null}
 */
export function parseCurl(text) {
  if (!text || !text.trim().startsWith("curl")) return null;

  try {
    const args = tokenize(text);
    let method = null;
    let url = "";
    const headers = [];
    let data = null;

    for (let i = 0; i < args.length; i++) {
      const arg = args[i];
      if (arg === "curl") continue;

      if (arg === "-X" || arg === "--request") {
        method = args[++i]?.toUpperCase();
      } else if (arg === "-H" || arg === "--header") {
        const headerStr = args[++i] || "";
        const colonIdx = headerStr.indexOf(":");
        if (colonIdx > 0) {
          headers.push({
            key: headerStr.slice(0, colonIdx).trim(),
            value: headerStr.slice(colonIdx + 1).trim(),
            enabled: true,
          });
        }
      } else if (
        arg === "-d" ||
        arg === "--data" ||
        arg === "--data-raw" ||
        arg === "--data-urlencode"
      ) {
        data = args[++i] || "";
      } else if (arg.startsWith("-") || arg.startsWith("--")) {
        // 跳过未知 flag 及其值
        if (i + 1 < args.length && !args[i + 1].startsWith("-")) {
          i++;
        }
      } else if (!url) {
        url = arg;
      }
    }

    if (!url) return null;
    if (!url.startsWith("http://") && !url.startsWith("https://")) {
      url = "http://" + url;
    }

    let bodyType = "none";
    let body = "";
    if (data != null) {
      body = data;
      bodyType = "raw";
      const ctHeader = headers.find(
        (h) => h.key.toLowerCase() === "content-type"
      );
      if (!ctHeader) {
        if (body.startsWith("{") || body.startsWith("[")) {
          headers.push({
            key: "Content-Type",
            value: "application/json",
            enabled: true,
          });
        } else if (body.includes("=")) {
          headers.push({
            key: "Content-Type",
            value: "application/x-www-form-urlencoded",
            enabled: true,
          });
        }
      }
    }

    if (!method) {
      method = data != null ? "POST" : "GET";
    }

    return { method, url, headers, bodyType, body };
  } catch {
    return null;
  }
}

/**
 * 将请求对象转为 cURL 命令字符串。
 */
export function toCurl(request) {
  const parts = [`curl`];

  parts.push(`-X ${request.method}`);
  parts.push(`'${request.url}'`);

  for (const h of request.headers || []) {
    if (h.enabled !== false) {
      parts.push(`-H '${h.key}: ${h.value}'`);
    }
  }

  if (request.body && request.bodyType !== "none") {
    parts.push(`-d '${request.body.replace(/'/g, "'\\''")}'`);
  }

  return parts.join(" \\\n  ");
}

/** 简易 shell token 解析：处理单引号和双引号包裹的参数。 */
function tokenize(text) {
  const tokens = [];
  let current = "";
  let inQuote = null;

  for (let i = 0; i < text.length; i++) {
    const ch = text[i];

    if (inQuote) {
      if (ch === inQuote) {
        inQuote = null;
      } else {
        current += ch;
      }
      continue;
    }

    if (ch === "'" || ch === '"') {
      inQuote = ch;
      continue;
    }

    if (ch === "\\" && i + 1 < text.length) {
      current += text[++i];
      continue;
    }

    if (/\s/.test(ch)) {
      if (current) {
        tokens.push(current);
        current = "";
      }
      continue;
    }

    current += ch;
  }

  if (current) tokens.push(current);
  return tokens;
}
