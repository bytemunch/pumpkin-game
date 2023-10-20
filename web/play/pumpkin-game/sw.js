const addResourcesToCache = async (resources) => {
  const cache = await caches.open("v1");
  await cache.addAll(resources);
};

self.addEventListener("install", (event) => {
  event.waitUntil(
    addResourcesToCache([
      "./",
      "./index.html",
      "./bin/pumpkin-game.js",
      "./bin/pumpkin-game_opt.wasm",
      "./icons/icon512_maskable.png",
      "./assets/Creepster-Regular.ttf",
      "./assets/apple@128.png",
      "./assets/apple@256.png",
      "./assets/apple@32.png",
      "./assets/apple@512.png",
      "./assets/apple@64.png",
      "./assets/bat@128.png",
      "./assets/bat@256.png",
      "./assets/bat@32.png",
      "./assets/bat@512.png",
      "./assets/bat@64.png",
      "./assets/bg.png",
      "./assets/candy_apple@128.png",
      "./assets/candy_apple@256.png",
      "./assets/candy_apple@32.png",
      "./assets/candy_apple@512.png",
      "./assets/candy_apple@64.png",
      "./assets/drop-1.ogg",
      "./assets/fg.png",
      "./assets/frankenstein@128.png",
      "./assets/frankenstein@256.png",
      "./assets/frankenstein@32.png",
      "./assets/frankenstein@512.png",
      "./assets/frankenstein@64.png",
      "./assets/game-over.ogg",
      "./assets/ghost@128.png",
      "./assets/ghost@256.png",
      "./assets/ghost@32.png",
      "./assets/ghost@512.png",
      "./assets/ghost@64.png",
      "./assets/mummy@128.png",
      "./assets/mummy@256.png",
      "./assets/mummy@32.png",
      "./assets/mummy@512.png",
      "./assets/mummy@64.png",
      "./assets/pop-1.ogg",
      "./assets/pumpkin@128.png",
      "./assets/pumpkin@256.png",
      "./assets/pumpkin@32.png",
      "./assets/pumpkin@512.png",
      "./assets/pumpkin@64.png",
      "./assets/skull@128.png",
      "./assets/skull@256.png",
      "./assets/skull@32.png",
      "./assets/skull@512.png",
      "./assets/skull@64.png",
      "./assets/spider@128.png",
      "./assets/spider@256.png",
      "./assets/spider@32.png",
      "./assets/spider@512.png",
      "./assets/spider@64.png",
      "./assets/spook.ogg",
      "./assets/sweet@128.png",
      "./assets/sweet@256.png",
      "./assets/sweet@32.png",
      "./assets/sweet@512.png",
      "./assets/sweet@64.png",
      "./assets/vampire@128.png",
      "./assets/vampire@256.png",
      "./assets/vampire@32.png",
      "./assets/vampire@512.png",
      "./assets/vampire@64.png",
    ]),
  );
});

const putInCache = async (request, response) => {
  const cache = await caches.open("v1");
  await cache.put(request, response);
};

const cacheFirst = async ({ request, fallbackUrl }) => {
  // First try to get the resource from the cache
  const responseFromCache = await caches.match(request);
  if (responseFromCache) {
    return responseFromCache;
  }

  // Next try to get the resource from the network
  try {
    const responseFromNetwork = await fetch(request);
    // response may be used only once
    // we need to save clone to put one copy in cache
    // and serve second one
    putInCache(request, responseFromNetwork.clone());
    return responseFromNetwork;
  } catch (error) {
    const fallbackResponse = await caches.match(fallbackUrl);
    if (fallbackResponse) {
      return fallbackResponse;
    }
    // when even the fallback response is not available,
    // there is nothing we can do, but we must always
    // return a Response object
    return new Response("Network error happened", {
      status: 408,
      headers: { "Content-Type": "text/plain" },
    });
  }
};

self.addEventListener("fetch", (event) => {
  event.respondWith(
    cacheFirst({
      request: event.request,
      fallbackUrl: "/icons/icon512_maskable.png",
    }),
  );
});
