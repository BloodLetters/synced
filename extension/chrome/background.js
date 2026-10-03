const DEFAULT_CONFIG = {
  enabled: true,
  port: 17890,
  notifyOnSuccess: true,
};

const handledDownloadIds = new Set();
const bypassedUrls = new Set();

async function getConfig() {
  try {
    const res = await chrome.storage.local.get(DEFAULT_CONFIG);
    return { ...DEFAULT_CONFIG, ...res };
  } catch (err) {
    return { ...DEFAULT_CONFIG };
  }
}

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "synced-download-link",
    title: "Download with synceD",
    contexts: ["link", "image", "video", "audio"],
  });
});

chrome.contextMenus.onClicked.addListener(async (info) => {
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

chrome.downloads.onCreated.addListener(async (downloadItem) => {
  const config = await getConfig();
  if (!config.enabled) {
    return;
  }

  const url = downloadItem.url || downloadItem.finalUrl;
  if (!url || typeof url !== "string") {
    return;
  }

  if (
    url.startsWith("blob:") ||
    url.startsWith("data:") ||
    url.startsWith("about:") ||
    url.startsWith("chrome:") ||
    url.startsWith("chrome-extension:")
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
    await chrome.downloads.cancel(downloadItem.id);
    await chrome.downloads.erase({ id: downloadItem.id });
  } catch (e) {
    // Already finished or cancelled
  }

  const success = await sendToSynced(url, downloadItem.filename || "", config.port);

  if (success) {
    if (config.notifyOnSuccess) {
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
      "Mengunduh file melalui browser Chrome..."
    );

    bypassedUrls.add(url);
    try {
      await chrome.downloads.download({
        url: url,
        filename: downloadItem.filename || undefined,
        conflictAction: "uniquify",
      });
    } catch (err) {
      console.error("Fallback download error:", err);
    }
  }
});

async function sendToSynced(url, filename, port = null) {
  if (!port) {
    const config = await getConfig();
    port = config.port;
  }

  const endpoint = `http://127.0.0.1:${port}/download`;
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
  const config = await getConfig();
  const endpoint = `http://127.0.0.1:${config.port}/health`;
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
    chrome.notifications.create({
      type: "basic",
      iconUrl: "icons/icon-48.png",
      title: title,
      message: message,
    });
  } catch (e) {
    // Notification error
  }
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
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
