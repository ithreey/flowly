/**
 * HAR (HTTP Archive) 格式转换工具
 * 规范: http://www.softwareishard.com/blog/har-12-spec/
 */

/**
 * 解析 URL 查询参数为 HAR queryString 数组
 * @param {string} url - 完整 URL
 * @returns {Array<{name: string, value: string}>}
 */
function parseQueryString(url) {
  try {
    const params = new URL(url).searchParams;
    return Array.from(params.entries()).map(([name, value]) => ({
      name,
      value,
    }));
  } catch {
    return [];
  }
}

/**
 * 从 headers 数组提取指定 header 的值
 * @param {Array<[string, string]>} headers - [[name, value], ...]
 * @param {string} headerName - header 名称（不区分大小写）
 * @returns {string} header 值，不存在返回默认值
 */
function getHeader(headers, headerName, defaultValue = "") {
  const header = headers.find(
    ([k]) => k.toLowerCase() === headerName.toLowerCase(),
  );
  return header ? header[1] : defaultValue;
}

export function isFormUrlEncoded(headers) {
  return getHeader(headers, "content-type")
    .toLowerCase()
    .includes("application/x-www-form-urlencoded");
}

export function parseFormParams(body) {
  if (!body) return [];
  try {
    return Array.from(new URLSearchParams(body).entries()).map(
      ([name, value]) => ({ name, value }),
    );
  } catch {
    return [];
  }
}

function buildPostData(txn) {
  if (!txn.reqBody) return undefined;

  const mimeType = getHeader(
    txn.reqHeaders,
    "content-type",
    "application/octet-stream",
  );
  const postData = {
    mimeType,
    text: txn.reqBody,
  };

  if (isFormUrlEncoded(txn.reqHeaders)) {
    postData.params = parseFormParams(txn.reqBody);
  }

  return postData;
}

/**
 * 将 TransactionDetail 转换为 HAR entry
 * @param {Object} txn - TransactionDetail 对象
 * @returns {Object} HAR entry 对象
 */
export function transactionToHarEntry(txn) {
  return {
    startedDateTime: new Date(Number(txn.summary.startedAt)).toISOString(),
    time: Number(txn.summary.durationMs),
    request: {
      method: txn.summary.method,
      url: txn.summary.url,
      httpVersion: "HTTP/1.1",
      headers: txn.reqHeaders.map(([name, value]) => ({ name, value })),
      queryString: parseQueryString(txn.summary.url),
      cookies: [],
      headersSize: -1,
      bodySize: txn.summary.reqSize || 0,
      postData: buildPostData(txn),
    },
    response: {
      status: txn.summary.status || 0,
      statusText: "",
      httpVersion: "HTTP/1.1",
      headers: txn.resHeaders.map(([name, value]) => ({ name, value })),
      cookies: [],
      content: {
        size: txn.summary.resSize || 0,
        mimeType: getHeader(
          txn.resHeaders,
          "content-type",
          "application/octet-stream",
        ),
        text: txn.resBody || "",
      },
      redirectURL: "",
      headersSize: -1,
      bodySize: txn.summary.resSize || 0,
    },
    cache: {},
    timings: {
      send: 0,
      wait: Number(txn.summary.durationMs),
      receive: 0,
    },
  };
}

/**
 * 生成完整的 HAR 文件对象
 * @param {Array<Object>} entries - HAR entry 数组
 * @returns {Object} 完整的 HAR 文件对象
 */
export function generateHarFile(entries) {
  return {
    log: {
      version: "1.2",
      creator: {
        name: "Flowly Proxy",
        version: "1.0.0",
      },
      entries: entries,
    },
  };
}
