// 숫자로만 된 경로(/123)와 /api/* 는 Railway 백엔드로 프록시하고,
// 나머지는 정적 파일(Dioxus 화면)을 내준다.
const PROXY_PATH = /^\/[0-9]+$/;

function shouldProxy(pathname) {
  return PROXY_PATH.test(pathname) || pathname.startsWith("/api/");
}

async function proxy(request, env) {
  const incoming = new URL(request.url);
  const target = new URL(incoming.pathname + incoming.search, env.BACKEND_URL);

  const headers = new Headers(request.headers);
  headers.delete("host");
  headers.set("X-Forwarded-Host", incoming.host);
  headers.set("X-Forwarded-Proto", incoming.protocol.replace(":", ""));

  const hasBody = request.method !== "GET" && request.method !== "HEAD";
  return fetch(target, {
    method: request.method,
    headers,
    body: hasBody ? request.body : undefined,
    redirect: "manual", // 302를 따라가지 않고 브라우저에 그대로 전달
  });
}

export default {
  async fetch(request, env) {
    const { pathname } = new URL(request.url);
    if (shouldProxy(pathname)) {
      return proxy(request, env);
    }
    return env.ASSETS.fetch(request);
  },
};
