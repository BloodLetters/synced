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
  chrome.storage.local.get(
    {
      enabled: true,
      port: 17890,
    },
    (config) => {
      interceptToggle.checked = config.enabled;
      portInput.value = config.port;
      currentPort = config.port;
      checkStatus();
    }
  );
}

function checkStatus() {
  chrome.runtime.sendMessage({ action: "PING" }, (response) => {
    if (chrome.runtime.lastError || !response || !response.online) {
      statusPill.className = "status-pill offline";
      statusText.textContent = "OFFLINE";
    } else {
      statusPill.className = "status-pill online";
      statusText.textContent = "ONLINE";
    }
  });
}

interceptToggle.addEventListener("change", () => {
  chrome.storage.local.set({ enabled: interceptToggle.checked });
});

sendBtn.addEventListener("click", () => {
  const url = urlInput.value.trim();
  if (!url) {
    showFeedback("Masukkan URL terlebih dahulu", false);
    return;
  }

  sendBtn.disabled = true;
  feedback.textContent = "Mengirim...";
  feedback.className = "feedback";

  chrome.runtime.sendMessage(
    {
      action: "SEND_URL",
      url: url,
    },
    (res) => {
      sendBtn.disabled = false;
      if (chrome.runtime.lastError || !res || !res.success) {
        showFeedback("Gagal: synceD tidak aktif di port " + currentPort, false);
        statusPill.className = "status-pill offline";
        statusText.textContent = "OFFLINE";
      } else {
        showFeedback("Terkirim ke synceD!", true);
        urlInput.value = "";
        statusPill.className = "status-pill online";
        statusText.textContent = "ONLINE";
      }
    }
  );
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

savePortBtn.addEventListener("click", () => {
  const val = parseInt(portInput.value, 10);
  if (!val || val < 1024 || val > 65535) {
    showFeedback("Port tidak valid (1024 - 65535)", false);
    return;
  }
  currentPort = val;
  chrome.storage.local.set({ port: val }, () => {
    showFeedback("Port tersimpan!", true);
    checkStatus();
  });
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
