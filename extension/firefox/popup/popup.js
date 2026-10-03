const statusPill = document.getElementById("status-pill");
const statusText = document.getElementById("status-text");
const interceptToggle = document.getElementById("intercept-toggle");
const urlInput = document.getElementById("url-input");
const sendBtn = document.getElementById("send-btn");
const feedback = document.getElementById("feedback");
const accordionToggle = document.getElementById("accordion-toggle");
const accordionBody = document.getElementById("accordion-body");
const accordionArrow = document.getElementById("accordion-arrow");
const portInput = document.getElementById("port-input");
const savePortBtn = document.getElementById("save-port-btn");

let currentPort = 17890;

async function init() {
  const config = await browser.storage.local.get({
    enabled: true,
    port: 17890,
  });

  interceptToggle.checked = config.enabled;
  portInput.value = config.port;
  currentPort = config.port;

  checkStatus();
}

async function checkStatus() {
  try {
    const response = await browser.runtime.sendMessage({ action: "PING" });
    if (response && response.online) {
      statusPill.className = "status-pill online";
      statusText.textContent = "ONLINE";
    } else {
      statusPill.className = "status-pill offline";
      statusText.textContent = "OFFLINE";
    }
  } catch (err) {
    statusPill.className = "status-pill offline";
    statusText.textContent = "OFFLINE";
  }
}

interceptToggle.addEventListener("change", async () => {
  await browser.storage.local.set({ enabled: interceptToggle.checked });
});

sendBtn.addEventListener("click", async () => {
  const url = urlInput.value.trim();
  if (!url) {
    showFeedback("Masukkan URL terlebih dahulu", false);
    return;
  }

  sendBtn.disabled = true;
  feedback.textContent = "Mengirim...";
  feedback.className = "feedback";

  try {
    const res = await browser.runtime.sendMessage({
      action: "SEND_URL",
      url: url,
    });

    if (res && res.success) {
      showFeedback("Terkirim ke synceD!", true);
      urlInput.value = "";
      statusPill.className = "status-pill online";
      statusText.textContent = "ONLINE";
    } else {
      showFeedback("Gagal: synceD tidak aktif di port " + currentPort, false);
      statusPill.className = "status-pill offline";
      statusText.textContent = "OFFLINE";
    }
  } catch (err) {
    showFeedback("Gagal terhubung ke synceD", false);
  } finally {
    sendBtn.disabled = false;
  }
});

urlInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    sendBtn.click();
  }
});

accordionToggle.addEventListener("click", () => {
  const isOpen = accordionBody.classList.toggle("open");
  accordionArrow.classList.toggle("open", isOpen);
});

savePortBtn.addEventListener("click", async () => {
  const val = parseInt(portInput.value, 10);
  if (!val || val < 1024 || val > 65535) {
    showFeedback("Port tidak valid (1024 - 65535)", false);
    return;
  }
  currentPort = val;
  await browser.storage.local.set({ port: val });
  showFeedback("Port tersimpan!", true);
  checkStatus();
});

function showFeedback(text, isSuccess) {
  feedback.textContent = text;
  feedback.className = "feedback " + (isSuccess ? "success" : "error");
  setTimeout(() => {
    if (feedback.textContent === text) {
      feedback.textContent = "";
      feedback.className = "feedback";
    }
  }, 3500);
}

document.addEventListener("DOMContentLoaded", init);
