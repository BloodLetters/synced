const DEFAULT_CONFIG = {
  enabled: true,
  port: 17890,
  notifyOnSuccess: true,
};

let currentConfig = { ...DEFAULT_CONFIG };
const handledDownloadIds = new Set();
const bypassedUrls = new Set();

async function loadConfig() {
  try {
    const res = await browser.storage.local.get(DEFAULT_CONFIG);
    currentConfig = { ...DEFAULT_CONFIG, ...res };
  } catch (err) {
    currentConfig = { ...DEFAULT_CONFIG };
  }
}

loadConfig();

browser.storage.onChanged.addListener((changes, area) => {
  if (area === "local") {
    for (const key of Object.keys(changes)) {
      currentConfig[key] = changes[key].newValue;
    }
  }
});

browser.runtime.onInstalled.addListener(() => {
  browser.menus.create({
    id: "synced-download-link",
    title: "Download with synceD",
    contexts: ["link", "image", "video", "audio"],
  });
});

browser.menus.onClicked.addListener(async (info) => {
  const targetUrl = info.linkUrl || info.srcUrl;
  if (!targetUrl) return;

  const success = await sendToSynced(targetUrl, "");
  if (!success) {
    showNotification(
      "synceD Offline",
      "Gagal terhubung ke synceD. Pastikan aplikasi synceD desktop sedang berjalan."
    );
  }
});

browser.downloads.onCreated.addListener(async (downloadItem) => {
  if (!currentConfig.enabled) {
    return;
  }

  const url = downloadItem.url;
  if (!url || typeof url !== "string") {
    return;
  }

  if (
    url.startsWith("blob:") ||
    url.startsWith("data:") ||
    url.startsWith("about:") ||
    url.startsWith("moz-extension:")
  ) {
    return;
  }

  if (handledDownloadIds.has(downloadItem.id)) {
    return;
  }
  handledDownloadIds.add(downloadItem.id);

  if (bypassedUrls.has(url)) {
    bypassedUrls.delete(url);
    return;
  }

  try {
    await browser.downloads.cancel(downloadItem.id);
    await browser.downloads.erase({ id: downloadItem.id });
  } catch (e) {
    // Download might have ended or been handled
  }

  const success = await sendToSynced(url, downloadItem.filename || "");

  if (success) {
    if (currentConfig.notifyOnSuccess) {
      const displayName = downloadItem.filename
        ? downloadItem.filename.split(/[\\/]/).pop()
        : url.slice(0, 48);
      showNotification(
        "Download dialihkan ke synceD",
        displayName
      );
    }
  } else {
    showNotification(
      "synceD Tidak Aktif",
      "Mengunduh file melalui browser Firefox..."
    );

    bypassedUrls.add(url);
    try {
      await browser.downloads.download({
        url: url,
        filename: downloadItem.filename || undefined,
        conflictAction: "uniquify",
      });
    } catch (err) {
      console.error("Fallback download error:", err);
    }
  }
});

async function sendToSynced(url, filename) {
  const endpoint = `http://127.0.0.1:${currentConfig.port}/download`;
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 2500);

    const res = await fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ url, filename }),
      signal: controller.signal,
    });
    clearTimeout(timeoutId);

    return res.ok;
  } catch (err) {
    return false;
  }
}

async function pingSynced() {
  const endpoint = `http://127.0.0.1:${currentConfig.port}/health`;
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 1500);
    const res = await fetch(endpoint, { signal: controller.signal });
    clearTimeout(timeoutId);
    return res.ok;
  } catch (err) {
    return false;
  }
}

function showNotification(title, message) {
  try {
    browser.notifications.create({
      type: "basic",
      iconUrl: "icons/icon-48.png",
      title: title,
      message: message,
    });
  } catch (e) {
    // Notification permission or creation error
  }
}

browser.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.action === "PING") {
    pingSynced().then((online) => sendResponse({ online }));
    return true;
  }
  if (message.action === "SEND_URL") {
    sendToSynced(message.url, message.filename || "").then((success) =>
      sendResponse({ success })
    );
    return true;
  }
});
